#!/usr/bin/env bash
# Bootstrap a SpecKit project: scaffold .specify/, register the community
# extension catalog, install + enable the required extension set, register their
# command files for the requested integration, and install the workflow
# definitions. Idempotent -- safe to re-run.
#
# This is the single source of truth for the spec-kit side of SpecKit setup.
# The global `project-setup` skill delegates here (after `apm install speckit`)
# rather than carrying its own copy.
#
# Prereqs: `specify` CLI on PATH (uv tool install specify-cli).
# The APM speckit orchestration bundle (agents, DAG, hooks) carries this script;
# the bundle's DAG keys off the `.specify/` scaffold this produces.
#
# Usage: setup-speckit.sh [--integration <name>] [--render-for <csv>] [--script <sh|ps>] [--force]
#   --integration   PRIMARY coding-agent integration -- the one `specify init` records as
#                   default_integration and the one this script lands on at the end.
#                   DEFAULT: auto-detected from the agent running this script (see below);
#                   falls back to codex with a warning only when undetectable. The agent
#                   invoking the skill SHOULD pass this explicitly (it knows what it is).
#   --render-for    Comma-separated integrations to render extension command files for, so
#                   /speckit.* exists in every agent the project compiles steering for (e.g.
#                   "claude,codex"). The primary is always included. DEFAULT: just --integration.
#   --script        script flavor for `specify init` (default: sh)
#   --force         re-run `specify init` even if .specify/ already exists (re-scaffold)
#
# WHY auto-detect the primary: `specify extension add` renders an extension's command files
# ONLY for the integration active at add-time. If `specify init` records the wrong primary
# (historically a hardcoded `codex`), every extension renders for codex even when a Claude Code
# session is driving setup -- and a naive re-run repeats the mistake. Detecting the running
# agent makes the default correct; the explicit --integration parameter lets the agent decide.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKFLOW_ROOT="$SCRIPT_DIR/workflows"

INTEGRATION=""        # empty => auto-detect (resolve_primary_integration below)
RENDER_FOR=""         # empty => render for the primary only
SCRIPT_FLAVOR="sh"
FORCE=""
FAILED_EXTENSIONS=""  # accumulates skipped extension names for end-of-step summary

while [ $# -gt 0 ]; do
  case "$1" in
    --integration) INTEGRATION="${2:?--integration needs a value}"; shift 2 ;;
    --render-for)  RENDER_FOR="${2:?--render-for needs a value}"; shift 2 ;;
    --script)      SCRIPT_FLAVOR="${2:?--script needs a value}"; shift 2 ;;
    --force)       FORCE="--force"; shift ;;
    -h|--help)     sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Detect which coding agent is running this script from its environment. Returns the
# spec-kit integration name (claude|codex) or empty if undetectable (plain shell / CI).
# Ordered most-specific-first; AI_AGENT is the normalized cross-agent marker, then each
# agent's own native env vars as a fallback.
detect_agent() {
  case "${AI_AGENT:-}" in
    *claude*) echo claude; return ;;
    *codex*)  echo codex;  return ;;
  esac
  if [ -n "${CLAUDECODE:-}" ] || [ -n "${CLAUDE_CODE_ENTRYPOINT:-}" ] || [ -n "${CLAUDE_CODE_SESSION_ID:-}" ]; then
    echo claude; return
  fi
  if [ -n "${CODEX_SANDBOX:-}" ] || [ -n "${CODEX_HOME:-}" ] || [ -n "${CODEX_SANDBOX_NETWORK_DISABLED:-}" ]; then
    echo codex; return
  fi
  echo ""
}

# Built-in integration to bounce through when forcing a re-registration switch onto an
# already-active integration (switch-to-self is a no-op and renders nothing).
other_builtin() { [ "$1" = "codex" ] && echo claude || echo codex; }

# Resolve the primary integration: explicit flag wins; else detect; else codex + warn.
if [ -z "$INTEGRATION" ]; then
  INTEGRATION="$(detect_agent)"
  if [ -n "$INTEGRATION" ]; then
    echo "==> auto-detected primary integration: $INTEGRATION (pass --integration to override)"
  else
    INTEGRATION="codex"
    echo "WARNING: could not detect the running agent; defaulting primary integration to '$INTEGRATION'." >&2
    echo "         Pass --integration <claude|codex> explicitly to be sure." >&2
  fi
fi

# Build the ordered, de-duplicated render list: every requested integration with the
# primary forced LAST, so the script lands on the primary after rendering the others.
RENDER_LIST=()
_add_render() {
  local want="$1" have
  [ -z "$want" ] && return
  # Guard the array expansion: bash 3.2 (macOS default) under `set -u` errors on
  # "${arr[@]}" when arr is empty, so only iterate when there is something to compare.
  if [ "${#RENDER_LIST[@]}" -gt 0 ]; then
    for have in "${RENDER_LIST[@]}"; do [ "$have" = "$want" ] && return; done
  fi
  RENDER_LIST+=("$want")
}
# Split --render-for on commas (bash 3.2-safe), excluding the primary for now.
_old_ifs="$IFS"; IFS=','
for _r in $RENDER_FOR; do
  _r="${_r#"${_r%%[![:space:]]*}"}"; _r="${_r%"${_r##*[![:space:]]}"}"  # trim
  [ "$_r" = "$INTEGRATION" ] && continue
  _add_render "$_r"
done
IFS="$_old_ifs"
_add_render "$INTEGRATION"   # primary always last

CATALOG_NAME="community"
CATALOG_URL="https://raw.githubusercontent.com/github/spec-kit/main/extensions/catalog.community.json"

# The required extension set the DAG depends on. Keep in sync with the README
# "Setting up a SpecKit project" list and the speckit-dag node coverage.
# agent-assign is mandatory: steering routes implementation through the
# agent-assign flow and the DAG hard-blocks the deprecated /speckit.implement.
#
# Entries are either a bare extension name (resolved from the community catalog)
# or `name=source-url` for a first-party extension not yet in the catalog, which
# installs via `specify extension add --from <url>`. Custom-source installs are
# best-effort: an unreachable/unpublished source warns and is skipped rather than
# aborting setup. One list, one source of truth; bash 3.2-safe (no associative arrays).
#   roadmap -- the spec-roadmap extension (srobroek/speckit-roadmap); accepted into the
#   community catalog 2026-06, so it resolves by name like the rest.
#
# An entry's source value (after `=`) takes one of two forms:
#   * a direct archive URL              -> installed via `specify extension add NAME --from <url>`
#   * `latest-release:<owner>/<repo>`   -> the latest published GitHub release tag is resolved
#                                          at setup time and its .zip archive is installed
# `specify extension add --from` requires a real archive URL (a bare repo URL is fetched as a
# zip and fails); `latest-release:` exists so we track newest WITHOUT pinning a version.
#   status-report -- the Open-Agent-Tools/spec-kit-status extension (catalog id
#   `status-report`), NOT the single-commit KhawarHabibKhan `status` extension it
#   replaced. Both ship a read-only progress command; status-report is the more
#   maintained one (script-driven JSON, cross-platform). It provides
#   `/speckit.status-report.show`. NOTE: despite its read-only catalog tag it
#   writes `specs/spec-status.md` on every run -- gitignored in the scaffold.
#   Installed via `latest-release:` (newest GitHub release tag resolved at setup
#   time) rather than the community catalog, which lags behind upstream.
#
# verify + verify-tasks are NOT in this list: verification runs via the merged
# `speckit-verify` local agent (spawned as a prompt step in the workflow YAMLs),
# which writes the required report files that gate downstream DAG nodes.
EXTENSIONS=(
  agent-assign
  cleanup critique
  fix-findings iterate qa
  retro review roadmap security-review
  status-report=latest-release:Open-Agent-Tools/spec-kit-status
  tinyspec
)

# Workflow definitions, installed via the `workflow` primitive (since spec-kit
# 0.11.x workflows are a first-class primitive, NOT extensions -- they do not
# resolve through `extension add`). All three ship in this package under
# workflows/<id>/workflow.yml and are installed from those local dirs:
#   speckit          -- our gated override of the upstream Full SDD Cycle
#   speckit-quality  -- post-implementation QA cycle
#   speckit-full     -- spec -> implement -> QA in one run
WORKFLOWS=(speckit speckit-quality speckit-full)

need() { command -v "$1" >/dev/null 2>&1 || { echo "ERROR: '$1' not found on PATH" >&2; exit 1; }; }
need specify

# Require spec-kit >= 0.12.0: workflows are a first-class primitive (not an extension),
# --no-git was removed, and specify-cli is published natively on PyPI.
# Upgrade with: uv tool install specify-cli  (installs/upgrades from PyPI)
_specify_ver="$(specify --version 2>/dev/null | grep -Eo '[0-9]+\.[0-9]+' | head -n1)"
_specify_major="${_specify_ver%%.*}"
_specify_minor="${_specify_ver#*.}"; _specify_minor="${_specify_minor%%.*}"
_ver_ok=0
if [ -n "$_specify_major" ] && [ -n "$_specify_minor" ]; then
  if [ "$_specify_major" -gt 0 ]; then
    _ver_ok=1   # major >= 1 is fine
  elif [ "$_specify_major" -eq 0 ] && [ "$_specify_minor" -ge 12 ]; then
    _ver_ok=1   # 0.12.x or higher 0.x
  fi
fi
if [ "$_ver_ok" -ne 1 ]; then
  echo "ERROR: specify >= 0.12.0 required (found: ${_specify_ver:-unknown})" >&2
  echo "       Upgrade with: uv tool install specify-cli" >&2
  exit 1
fi
unset _specify_ver _specify_major _specify_minor _ver_ok

echo "==> 1/6 specify init (.specify/ scaffold) -- integration=$INTEGRATION script=$SCRIPT_FLAVOR"
if [ -d .specify ] && [ -z "$FORCE" ]; then
  echo "    .specify/ already present -- skipping init (pass --force to re-run)"
else
  # Always pass --force so the init is unconditionally non-interactive: on a
  # fresh git repo .git/ makes the directory non-empty and specify prompts y/N
  # (default: abort) when stdin is /dev/null. --force skips that check entirely.
  # stdin from /dev/null so the post-init "Agent Folder Security" prompt and any
  # other interactive confirmations resolve to their non-interactive default
  # instead of blocking (or aborting under set -e).
  specify init --here --integration "$INTEGRATION" --script "$SCRIPT_FLAVOR" --force </dev/null
fi

echo "==> 2/6 register community extension catalog"
# Match on URL, not just name: a default catalog (e.g. 'custom' from
# SPECKIT_CATALOG_URL) may already point at this community URL.
#
# Two failure modes this guards against (both make re-runs non-deterministic):
#   1. `specify ... catalog list` WRAPS the URL across terminal lines even in
#      captured (non-TTY) output, so a single-line `grep -F "$CATALOG_URL"`
#      never matches a wrapped URL. Collapse all whitespace before matching.
#   2. When SPECKIT_CATALOG_URL is set, `catalog list` shows only the env-var
#      'custom' catalog and MASKS a persistent same-URL 'community' catalog from
#      an earlier run. The name guard then misses it, we fall through to
#      `catalog add --name community`, specify rejects the duplicate, and
#      `set -e` aborts before extensions install. So treat a failing add whose
#      cause is "already exists" as success -- the desired end state (a catalog
#      for this URL is registered) is already true.
catalogs="$(specify extension catalog list 2>/dev/null || true)"
# Whitespace-collapsed haystack so a line-wrapped URL still matches.
catalogs_flat="$(printf '%s' "$catalogs" | tr -s '[:space:]' ' ')"
if printf '%s' "$catalogs_flat" | grep -qF "$CATALOG_URL"; then
  echo "    a catalog for this URL is already registered -- skipping"
elif printf '%s\n' "$catalogs" | grep -qw "$CATALOG_NAME"; then
  echo "    catalog '$CATALOG_NAME' already registered -- skipping"
else
  # Disable -e around the add so an "already exists" rejection (masked catalog,
  # mode 2 above) does not abort the whole setup; surface any other failure.
  set +e
  add_out="$(specify extension catalog add --name "$CATALOG_NAME" --install-allowed "$CATALOG_URL" </dev/null 2>&1)"
  add_rc=$?
  set -e
  if [ "$add_rc" -ne 0 ]; then
    if printf '%s' "$add_out" | grep -qi 'already exists'; then
      echo "    catalog '$CATALOG_NAME' already exists (masked from list) -- skipping"
    else
      printf '%s\n' "$add_out" >&2
      echo "ERROR: failed to register catalog '$CATALOG_NAME' ($CATALOG_URL)" >&2
      exit 1
    fi
  fi
fi

echo "==> 3/6 install + enable ${#EXTENSIONS[@]} extensions"
installed="$(specify extension list 2>/dev/null || true)"
for entry in "${EXTENSIONS[@]}"; do
  # Split "name=source" (custom source) from a bare "name" (community catalog).
  ext="${entry%%=*}"
  src="${entry#*=}"
  [ "$src" = "$entry" ] && src=""   # no '=' present -> no custom source
  if printf '%s\n' "$installed" | grep -qw "$ext"; then
    echo "    = $ext (already installed)"
  elif [ -n "$src" ]; then
    # Custom-source extension (not in the community catalog). Best-effort:
    # an unreachable/unpublished source warns and continues, leaving the rest
    # of the required catalog set intact.
    case "$src" in
      latest-release:*)
        repo="${src#latest-release:}"
        # Resolve the latest published release tag. Prefer `gh api` (authenticated,
        # no rate-limit risk) with a curl fallback. Both are wrapped so a failure
        # yields an empty tag rather than aborting the script under set -e -o pipefail.
        tag=""
        if command -v gh >/dev/null 2>&1; then
          tag="$(gh api "repos/${repo}/releases/latest" --jq '.tag_name' 2>/dev/null || true)"
        fi
        if [ -z "$tag" ]; then
          tag="$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null \
                   | grep -m1 '"tag_name"' | sed 's/.*"tag_name"[^"]*"\([^"]*\)".*/\1/' || true)"
        fi
        if [ -z "$tag" ]; then
          echo "    WARNING: could not resolve latest release of '$repo' for '$ext' -- skipping" >&2
          continue
        fi
        url="https://github.com/${repo}/archive/refs/tags/${tag}.zip"
        echo "    + $ext (latest release $tag of $repo)"
        ;;
      *)
        url="$src"
        echo "    + $ext (from $url)"
        ;;
    esac
    # `specify extension add --from` may prompt y/N (default: abort) for the
    # directory-not-empty check on a fresh git repo -- pipe `y` to confirm.
    if ! echo y | specify extension add "$ext" --from "$url"; then
      echo "    WARNING: could not install '$ext' from $url -- skipping (publish it or check access)" >&2
      continue
    fi
  else
    echo "    + $ext"
    # Best-effort, matching the custom-source branch above: a single broken
    # upstream (e.g. an extension whose tagged release archive 404s/400s) must
    # NOT abort the whole required-extension install under set -e. Warn, record,
    # and continue so the remaining catalog extensions still install.
    if ! specify extension add "$ext" </dev/null; then
      echo "    WARNING: could not install '$ext' from the '$CATALOG_NAME' catalog -- skipping" >&2
      FAILED_EXTENSIONS="$FAILED_EXTENSIONS $ext"
      continue
    fi
  fi
  specify extension enable "$ext" </dev/null >/dev/null 2>&1 || true
done

# Surface any skipped extensions as a single end-of-step summary so a partial
# install is visible without scrolling back through the per-extension output.
if [ -n "${FAILED_EXTENSIONS# }" ]; then
  echo "    NOTE: these extensions were skipped (upstream unavailable):${FAILED_EXTENSIONS}" >&2
  echo "          re-run setup-speckit.sh later to retry them once upstream is fixed." >&2
fi

echo "==> 4/6 register extension commands for: ${RENDER_LIST[*]} (primary=$INTEGRATION)"
# `specify extension add` only renders an extension's command files for the
# integration that is ACTIVE at add-time, and `specify integration switch`
# re-registers all installed+enabled extensions ONLY on a genuine switch
# (switching to the already-active integration is a no-op). So if extensions were
# added under a different integration than the one now requested (e.g. the
# default `codex` init, then later using `claude`), their command files are never
# rendered for the requested agent -- and re-running this script does not fix it,
# because the extensions are already "installed" and the install loop skips them.
#
# We render for EVERY integration in RENDER_LIST so /speckit.* exists in each agent
# the project compiles steering for, walking the list in order (primary last so we
# land on it). For each target:
#   - target is NOT the active integration -> one genuine switch re-registers all.
#   - target IS already active             -> bounce through the other built-in and
#     back to force a re-registration (switch-to-self is a no-op).
# Switching built-in integrations (claude/codex) is offline; only the local
# extension registry is read to re-render command files.
read_active_integration() {
  grep -o '"default_integration"[[:space:]]*:[[:space:]]*"[^"]*"' .specify/integration.json 2>/dev/null \
    | sed 's/.*"\([^"]*\)".*/\1/' | head -n1
}
current_integration="$(read_active_integration)"
for target in "${RENDER_LIST[@]}"; do
  if [ -n "$current_integration" ] && [ "$current_integration" != "$target" ]; then
    specify integration switch "$target" </dev/null
    echo "    switched $current_integration -> $target (extensions re-registered)"
    current_integration="$target"
  else
    bounce="$(other_builtin "$target")"
    echo "    $target already active -- bouncing via $bounce to force re-registration"
    # Disable -e around the bounce so a mid-bounce failure cannot leave the project
    # stranded on the bounce integration; always attempt to land back on "$target".
    set +e
    specify integration switch "$bounce" </dev/null && specify integration switch "$target" </dev/null
    bounce_rc=$?
    set -e
    if [ "$bounce_rc" -ne 0 ]; then
      echo "    WARNING: re-registration bounce failed; ensuring active integration is $target" >&2
      specify integration switch "$target" </dev/null || true
    fi
    current_integration="$target"
  fi
done

echo "==> 5/6 install workflow definitions from local dirs: ${WORKFLOWS[*]}"
for wf in "${WORKFLOWS[@]}"; do
  wf_dir="$WORKFLOW_ROOT/$wf"
  if [ ! -f "$wf_dir/workflow.yml" ]; then
    echo "    WARN: workflow asset missing for $wf at $wf_dir -- skipping" >&2
    continue
  fi
  # Replace any existing definition so our opinionated overrides win over the
  # version spec-kit bundles at init (e.g. the upstream `speckit` workflow).
  if specify workflow list 2>/dev/null | grep -qw "$wf"; then
    echo "    ~ $wf (replacing existing)"
    specify workflow remove "$wf" </dev/null >/dev/null 2>&1 || true
  else
    echo "    + $wf"
  fi
  specify workflow add "$wf_dir" </dev/null
done

echo "==> 6/7 provision speckit-gate (gates.yaml-driven enforcement)"
# speckit-gate is a Python CLI distributed on PyPI and consumed via uvx (no APM
# dependency). It supersedes speckit-dag-hooks. Guard: skip gracefully when uvx
# cannot resolve it (publish may be pending) rather than aborting setup.
if uvx speckit-gate --help >/dev/null 2>&1; then
  echo "    speckit-gate available -- running init/compile/install"
  # Init writes gates.yaml from the built-in defaults (--defaults skips the
  # interactive wizard). Idempotent: re-running overwrites with the same content.
  uvx speckit-gate init --defaults

  # Merge the project overlay (A2 policy: deprecated implement, agent-assign
  # chain, verify/verify-tasks spawn-agent gates). Append the overlay's `gates:`
  # entries to the project's gates.yaml. This is safe because:
  #   1. init --defaults writes built-in commands only (core preset).
  #   2. The overlay keys (implement, agent-assign-*, verify, verify-tasks) are
  #      NOT in the core preset, so there are no collisions on a fresh init.
  #   3. If gates.yaml already has these keys (re-run) the append creates
  #      duplicates that speckit-gate compile will reject with a clear error;
  #      prefer the duplicate-key compile error over silently losing the overlay.
  OVERLAY="$SCRIPT_DIR/gates-overlay.yaml"
  if [ -f "$OVERLAY" ]; then
    GATES_FILE="gates.yaml"
    if [ -f "$GATES_FILE" ]; then
      # Extract only the `gates:` block from the overlay and append it.
      # sed -n '/^gates:/,$ p' preserves the header comment block under gates:.
      echo "    merging gates-overlay.yaml into $GATES_FILE"
      printf '\n' >> "$GATES_FILE"
      sed -n '/^gates:/,$ p' "$OVERLAY" | tail -n +2 >> "$GATES_FILE"
    else
      echo "    WARNING: gates.yaml not found after init -- overlay not merged" >&2
    fi
  else
    echo "    WARNING: gates-overlay.yaml not found at $OVERLAY -- skipping overlay merge" >&2
  fi

  # Compile resolves the merged gates.yaml into the hook dispatch table.
  uvx speckit-gate compile

  # Install merges the compiled hooks into .claude/settings.json (Claude harness).
  uvx speckit-gate install --harness claude
  echo "    speckit-gate: init + overlay + compile + install complete"
else
  echo "    SKIP: uvx could not resolve speckit-gate" >&2
  echo "          Install hint: pip install speckit-gate  OR  wait for PyPI publish" >&2
  echo "          Re-run setup-speckit.sh once speckit-gate is available to enable gate enforcement." >&2
fi

echo "==> 7/7 ignore generated status-report artefact"
# The status-report extension (/speckit.status-report.show) regenerates
# specs/spec-status.md on every run despite its read-only catalog tag. It is a
# derived report, not a tracked spec artefact (spec.md/plan.md/tasks.md ARE
# tracked), so ignore just that one file to keep it out of `git status` and
# accidental commits. Idempotent: append the entry only if absent.
GITIGNORE_ENTRY="specs/**/spec-status.md"
if [ -f .gitignore ] && grep -qxF "$GITIGNORE_ENTRY" .gitignore; then
  echo "    = $GITIGNORE_ENTRY (already ignored)"
else
  echo "    + $GITIGNORE_ENTRY"
  # Ensure a trailing newline before appending so we never glue onto a last
  # line that lacks one.
  [ -f .gitignore ] && [ -n "$(tail -c1 .gitignore 2>/dev/null)" ] && printf '\n' >> .gitignore
  printf '# SpecKit status-report generated artefact (regenerated each run)\n%s\n' "$GITIGNORE_ENTRY" >> .gitignore
fi

echo ""
echo "==> SpecKit setup complete."
echo "    The speckit orchestration layer (agents + gate hooks) ships in the same"
echo "    package as this script. If steering is not yet compiled, run:"
echo "      apm compile --target codex,claude --no-constitution"
echo "    Then start the workflow with /speckit.specify."
