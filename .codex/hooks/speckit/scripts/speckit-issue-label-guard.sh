#!/usr/bin/env bash
# Hook: PreToolUse:Bash -- enforce spec:, phase:, and deferred label conventions
# on gh/glab issue create CLI commands and GraphQL createIssue mutations.
# Non-blocking: emits an advisory (permissionDecision:"allow") and exits 0.

# Only activate in speckit projects
[ -d ".specify" ] || exit 0

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

_advise() {
  local msg="$1"
  jq -cn --arg ctx "$msg" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"allow",additionalContext:$ctx}}' \
    2>/dev/null || printf '%s\n' "$msg" >&2
}

# Check CLI issue creation (gh/glab).
# Anchor to a real command word so echoed/quoted/comment phrases don't fire.
if echo "$COMMAND" | grep -qE '(^|[;&|]|&&|\|\|)[[:space:]]*(gh|glab)[[:space:]]+issue[[:space:]]+create\b'; then
  # Label values must START with the required prefix (reject e.g. myspec:),
  # accepting -l/--label, = or space separators, and optional quoting.
  LABEL_FLAG='(--label|-l)[= ]+["'"'"']?'
  if ! echo "$COMMAND" | grep -qE "${LABEL_FLAG}spec:"; then
    _advise "Issue creation is missing a spec: label. Add --label 'spec:{spec-id}' to link this issue to its specification."
    exit 0
  fi
  # Require phase: label
  if ! echo "$COMMAND" | grep -qE "${LABEL_FLAG}phase:"; then
    _advise "Issue creation is missing a phase: label. Add --label 'phase:{name}' (e.g. phase:build, phase:design) to indicate the workflow phase."
    exit 0
  fi
  # Deferred issues need: deferred label + TWO spec: labels (source + target).
  # Detect a deferred issue ONLY by a real `deferred` LABEL VALUE -- not a loose
  # substring like `defer`/`deferred` anywhere in the command. The old substring
  # match blocked legitimate titles such as "fix deferred loading". Anchor the
  # value end so `deferred` is the whole label, not a prefix of e.g. `deferred-x`.
  if echo "$COMMAND" | grep -qE "${LABEL_FLAG}deferred([\"', ]|$)"; then
    SPEC_COUNT=$(echo "$COMMAND" | grep -oE "${LABEL_FLAG}spec:[^ \"']*" | wc -l | tr -d ' ')
    if [ "$SPEC_COUNT" -lt 2 ]; then
      _advise "Deferred issues must have TWO spec: labels -- spec:{source} (where discovered) and spec:{blocking} (what must complete before this work can proceed). Found $SPEC_COUNT."
      exit 0
    fi
  fi
fi

# Check GraphQL issue creation. Require BOTH:
#   1) an actual `gh api graphql` / `glab api graphql` COMMAND (so an echoed or
#      quoted mutation string — `echo "mutation { createIssue(...) }"` — or a
#      shell comment does NOT fire), AND
#   2) `createIssue(` within a mutation operation body.
# Anchoring (1) to the real CLI command is what distinguishes a genuine API call
# from arbitrary text that merely contains GraphQL-looking words.
if echo "$COMMAND" | grep -qE '(gh|glab)[[:space:]]+api[[:space:]]+graphql' \
  && echo "$COMMAND" | grep -qE 'mutation[[:space:]]*[A-Za-z_]*[[:space:]]*[({][^|;&]*createIssue[[:space:]]*\('; then
  if ! echo "$COMMAND" | grep -qE 'spec:'; then
    _advise "GraphQL createIssue mutation appears to be missing a spec: label. Include a spec:{spec-id} label in the mutation's labelIds or labels field."
    exit 0
  fi
fi

exit 0
