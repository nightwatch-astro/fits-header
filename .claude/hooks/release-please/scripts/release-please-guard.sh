#!/usr/bin/env bash
set -euo pipefail

# release-please-guard.sh -- PreToolUse:Bash advisory guard.
#
# WARN-ONLY. On a repo managed by release-please, manually cutting a release or
# merging to a protected branch outside the release PR is the #1 way to botch
# the release loop indefinitely (release-please then sees an untagged/mislabeled
# release and stops auto-tagging). This hook does NOT block -- it injects an
# advisory note (additionalContext) so the model reconsiders and reads the
# release-please skill. The command still runs.
#
# Never emits permissionDecision "deny" or "ask": per the repo hook policy,
# blocking decisions stall autonomous runs, and these operations are legitimate
# in recovery scenarios. The note is the whole point.
#
# Cross-tool contract: stdout JSON drives Claude; the exit code drives Codex
# (0 = allow). We ALWAYS exit 0 -- the warning lives in the JSON, never the code.
# Every jq call ends `2>/dev/null || true` so a jq hiccup under `set -e` cannot
# turn an advisory into a Codex-side block.
#
# Portability floor: bash 3.2.57 + BSD grep (stock macOS). POSIX parameter
# expansion, `case` globbing, bash `[[ =~ ]]` ERE only.

payload="$(cat 2>/dev/null || true)"
[ -z "$payload" ] && exit 0

# tool_input is either an object ({command:"..."}) OR a bare string. The naive
# `.tool_input.command // .tool_input` THROWS on a string in jq and silently
# leaves $command empty (bypass). Type-check first so both shapes are read.
command="$(
  printf '%s' "$payload" | jq -r '
    if (.tool_input | type) == "string" then .tool_input
    else (.tool_input.command // empty)
    end
  ' 2>/dev/null || true
)"
if [ -z "$command" ] || [ "$command" = "null" ]; then
  exit 0
fi

# Directory the command runs in. Both Claude and Codex put it in `.cwd`; fall
# back to $PWD when absent or not a real dir.
cwd="$(printf '%s' "$payload" | jq -r '.cwd // empty' 2>/dev/null || true)"
if [ -z "$cwd" ] || [ "$cwd" = "null" ] || [ ! -d "$cwd" ]; then
  cwd="$PWD"
fi

# Only engage when THIS repo is release-please managed. The detection script
# lives alongside the skill; resolve it relative to this hook (package root) with
# a couple of fallbacks so it works whether installed natively or from source.
here="$(cd "$(dirname "$0")" 2>/dev/null && pwd || true)"
detect=""
for cand in \
  "$here/../.apm/skills/release-please/scripts/detect-release-please.sh" \
  "$here/../skills/release-please/scripts/detect-release-please.sh" \
  "$here/detect-release-please.sh"; do
  if [ -f "$cand" ]; then detect="$cand"; break; fi
done

# If we can find the detector, use it; otherwise fall back to a cheap inline
# check so the guard still works when co-location differs.
is_rp_repo=false
if [ -n "$detect" ]; then
  if ( cd "$cwd" 2>/dev/null && bash "$detect" >/dev/null 2>&1 ); then
    is_rp_repo=true
  fi
else
  if [ -f "$cwd/release-please-config.json" ] || [ -f "$cwd/.release-please-manifest.json" ]; then
    is_rp_repo=true
  fi
fi
[ "$is_rp_repo" = true ] || exit 0

lowered="$(printf '%s' "$command" | tr '[:upper:]' '[:lower:]')"

warn() {
  jq -cn --arg ctx "$1" '{hookSpecificOutput:{hookEventName:"PreToolUse",additionalContext:$ctx}}' 2>/dev/null || true
  exit 0
}

# --- 1. Manual GitHub Release creation ---------------------------------------
# `gh release create ...` on a release-please repo creates a release the tool
# did not author, so it never flips autorelease:pending -> autorelease:tagged.
if [[ "$lowered" =~ (^|[[:space:]])gh[[:space:]]+release[[:space:]]+create([[:space:]]|$) ]]; then
  warn "RELEASE-PLEASE REPO: this repo is managed by release-please, which creates GitHub Releases automatically when the release PR merges. Running 'gh release create' manually cuts a release release-please did not author -- it will not flip the autorelease:pending label to autorelease:tagged, which can stall auto-tagging on every future release. Prefer merging the release PR. Only cut a manual release as a deliberate, documented fallback. See the release-please skill (references/pitfalls-recovery.md)."
fi

# --- 2. Manual version tag creation ------------------------------------------
# release-please owns the version tags (component--vX.Y.Z or vX.Y.Z). Creating
# one by hand can collide with the tag release-please will cut (duplicate-tag
# error) or leave the manifest and tags out of sync.
if [[ "$lowered" =~ (^|[[:space:]])git([[:space:]]+-[^[:space:]]+([[:space:]]+[^[:space:]]+)?)*[[:space:]]+tag([[:space:]]+-[a-z]+)*[[:space:]]+v?[0-9]+\.[0-9]+\.[0-9]+ ]]; then
  warn "RELEASE-PLEASE REPO: release-please owns version tags and cuts them automatically when the release PR merges. Creating a version tag by hand can collide with the tag release-please will create (duplicate-tag failure) or desync the manifest from the tags. Let the release PR do it. See the release-please skill."
fi

# Also catch pushing tags explicitly (git push --tags / git push <remote> <tag>).
if [[ "$lowered" =~ (^|[[:space:]])git([[:space:]]+-[^[:space:]]+)*[[:space:]]+push([[:space:]]+[^[:space:]]+)*[[:space:]]+(--tags|--follow-tags)([[:space:]]|$) ]]; then
  warn "RELEASE-PLEASE REPO: pushing tags manually (git push --tags/--follow-tags) can publish a version tag that collides with the one release-please cuts on release-PR merge. release-please pushes its own tags. See the release-please skill."
fi

# --- 3. Manual merge to a protected branch (bypassing the release PR) --------
# A real `git merge` on main/master (not the release PR flow). This is advisory:
# merging feature work to main is normal; the note reminds that RELEASES flow
# only through the release PR, never a hand-merge of a release branch.
git_prefix='git([[:space:]]+-[^[:space:]]+([[:space:]]+[^[:space:]]+)?)*[[:space:]]+'
if [[ "$lowered" =~ ${git_prefix}merge([[:space:]]|$) ]]; then
  # Skip recovery/inspection forms.
  if ! [[ "$lowered" =~ (^|[[:space:]])--(abort|continue|quit)([[:space:]=]|$) ]]; then
    cur_branch="$(cd "$cwd" 2>/dev/null && git branch --show-current 2>/dev/null || true)"
    case "$cur_branch" in
      main|master)
        warn "RELEASE-PLEASE REPO + PROTECTED BRANCH: you are on '$cur_branch'. Releases must flow through release-please's release PR (merge it via the PR, which triggers the tag + GitHub Release). Do NOT hand-merge a release branch (release-please--branches--*) into '$cur_branch' -- that bypasses the tagging step and can leave an untagged, merged release PR that stalls the loop. Ordinary feature merges are fine; releases are not. See the release-please skill."
        ;;
    esac
  fi
fi

exit 0
