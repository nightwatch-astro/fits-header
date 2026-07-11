#!/usr/bin/env bash
set -euo pipefail

# detect-release-please.sh -- report whether (and how) a repo is managed by
# release-please, so the skill and hooks can decide to engage.
#
# Portability floor: bash 3.2.57 + BSD grep/sed (stock macOS). Only POSIX
# parameter expansion, `case` globbing, and portable `grep -E` are used.
#
# Usage:
#   detect-release-please.sh [--json] [DIR]
#
#   DIR       repo root to inspect (default: git toplevel of $PWD, else $PWD).
#   --json    emit a machine-readable JSON object instead of KEY=VALUE lines.
#
# Exit status: 0 when release-please config is present, 1 when absent, 2 on a
# usage error. The caller uses the exit code as the "is this a release-please
# repo?" signal; the printed fields describe HOW it is configured.
#
# Fields reported:
#   present               true|false   -- any release-please config found
#   config_file           path         -- release-please-config.json (or legacy)
#   manifest_file         path         -- .release-please-manifest.json
#   workflow_files        csv paths    -- workflows referencing release-please
#   mode                  manifest|config-only|inline-action|none
#   separate_pull_requests true|false|unknown
#   include_component_in_tag true|false|unknown
#   tag_separator         string|unknown
#   package_count         integer      -- entries under "packages" (manifest mode)

emit_json=false
dir=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --json) emit_json=true ;;
    --help|-h)
      grep -E '^#( |$)' "$0" | sed -E 's/^# ?//'
      exit 0
      ;;
    -*)
      printf 'detect-release-please: unknown option: %s\n' "$1" >&2
      exit 2
      ;;
    *)
      if [ -n "$dir" ]; then
        printf 'detect-release-please: too many arguments\n' >&2
        exit 2
      fi
      dir="$1"
      ;;
  esac
  shift
done

# Resolve the repo root. Prefer git's toplevel so the check works from any
# subdirectory; fall back to the given dir or $PWD when not in a work tree.
if [ -z "$dir" ]; then
  dir="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  [ -n "$dir" ] || dir="$PWD"
fi
[ -d "$dir" ] || { printf 'detect-release-please: not a directory: %s\n' "$dir" >&2; exit 2; }

# --- locate config + manifest -------------------------------------------------
# release-please recognises release-please-config.json and, for legacy setups,
# a couple of alternate names. The manifest is .release-please-manifest.json.
config_file=""
for c in "release-please-config.json" ".release-please-config.json"; do
  if [ -f "$dir/$c" ]; then config_file="$dir/$c"; break; fi
done

manifest_file=""
if [ -f "$dir/.release-please-manifest.json" ]; then
  manifest_file="$dir/.release-please-manifest.json"
fi

# --- locate workflows that invoke the action ---------------------------------
# grep the workflow dir for the action slug; report matching files.
workflow_files=""
wf_dir="$dir/.github/workflows"
if [ -d "$wf_dir" ]; then
  # -l lists files; ignore errors when no match. Portable across BSD/GNU grep.
  matches="$(grep -rlE 'release-please-action|googleapis/release-please|release-please-action@' "$wf_dir" 2>/dev/null || true)"
  if [ -n "$matches" ]; then
    # Comma-join, stripping the repo prefix for readability.
    workflow_files="$(printf '%s\n' "$matches" | sed -E "s#^$dir/##" | paste -sd, - 2>/dev/null || printf '%s' "$matches" | tr '\n' ',')"
    workflow_files="${workflow_files%,}"
  fi
fi

# --- classify mode ------------------------------------------------------------
present=false
mode="none"
if [ -n "$config_file" ] && [ -n "$manifest_file" ]; then
  present=true; mode="manifest"
elif [ -n "$config_file" ]; then
  present=true; mode="config-only"
elif [ -n "$workflow_files" ]; then
  # No config file but the action is wired in a workflow -> inline/simple mode
  # (release-please can run purely from action inputs without a config file).
  present=true; mode="inline-action"
fi

# --- extract a few load-bearing config values (best-effort, no jq dependency) -
sep_prs="unknown"
inc_comp="unknown"
tag_sep="unknown"
pkg_count=0

if [ -n "$config_file" ]; then
  cfg="$(cat "$config_file" 2>/dev/null || true)"

  # "separate-pull-requests": true|false  (whitespace-tolerant)
  if printf '%s' "$cfg" | grep -Eq '"separate-pull-requests"[[:space:]]*:[[:space:]]*true'; then
    sep_prs="true"
  elif printf '%s' "$cfg" | grep -Eq '"separate-pull-requests"[[:space:]]*:[[:space:]]*false'; then
    sep_prs="false"
  fi

  if printf '%s' "$cfg" | grep -Eq '"include-component-in-tag"[[:space:]]*:[[:space:]]*true'; then
    inc_comp="true"
  elif printf '%s' "$cfg" | grep -Eq '"include-component-in-tag"[[:space:]]*:[[:space:]]*false'; then
    inc_comp="false"
  fi

  # "tag-separator": "X"  -> capture X
  ts="$(printf '%s' "$cfg" | grep -Eo '"tag-separator"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed -E 's/.*"tag-separator"[[:space:]]*:[[:space:]]*"([^"]*)".*/\1/' || true)"
  [ -n "$ts" ] && tag_sep="$ts"
fi

# package_count from the manifest (each key is a released path/component).
if [ -n "$manifest_file" ]; then
  pkg_count="$(grep -Ec '"[^"]+"[[:space:]]*:' "$manifest_file" 2>/dev/null || printf '0')"
fi

# --- emit ---------------------------------------------------------------------
if [ "$emit_json" = true ]; then
  # Hand-rolled JSON (no jq dependency); values are simple/known-safe.
  printf '{'
  printf '"present":%s,' "$present"
  printf '"mode":"%s",' "$mode"
  printf '"config_file":"%s",' "$config_file"
  printf '"manifest_file":"%s",' "$manifest_file"
  printf '"workflow_files":"%s",' "$workflow_files"
  printf '"separate_pull_requests":"%s",' "$sep_prs"
  printf '"include_component_in_tag":"%s",' "$inc_comp"
  printf '"tag_separator":"%s",' "$tag_sep"
  printf '"package_count":%s' "$pkg_count"
  printf '}\n'
else
  printf 'present=%s\n' "$present"
  printf 'mode=%s\n' "$mode"
  printf 'config_file=%s\n' "$config_file"
  printf 'manifest_file=%s\n' "$manifest_file"
  printf 'workflow_files=%s\n' "$workflow_files"
  printf 'separate_pull_requests=%s\n' "$sep_prs"
  printf 'include_component_in_tag=%s\n' "$inc_comp"
  printf 'tag_separator=%s\n' "$tag_sep"
  printf 'package_count=%s\n' "$pkg_count"
fi

[ "$present" = true ] && exit 0 || exit 1
