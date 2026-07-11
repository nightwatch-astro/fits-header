---
paths:
  - "{.specify/**,specs/**,**/spec.md,**/tasks.md,**/pending-iteration.md}"
---

# SpecKit Workflow

## Rules

- All workflow steps are mandatory; skip only on explicit user request.
- Invoke speckit via the Skill tool only. Never write spec artifacts manually.
- Approval gates: default → approval between phases; "run unattended" → skip non-interactive gates only; clarify/analyze/checklist → always interactive.
- Never proceed past a stage with open questions, unresolved gaps, or items requiring review.
- Never silently deviate from spec. Material deviation: stop, explain, get approval, route through iterate. Minor: flag in commit.
- Record every non-trivial hard-to-reverse decision as an ADR under `docs/adr/NNNN-title.md` (MADR, status proposed/accepted/superseded) at the moment it lands.
- `/speckit.implement` is deprecated; use agent-assign flow: assign → validate → execute.
- Orchestrator review gate: review actual git diff against task requirements before accepting. Keep the sub-agent alive; use `SendMessage` to send corrections to the same agent. Dismiss only once work passes. Applies under parallel (worktree-isolated) execution too.

## Workflow Steps

Phases run in order (1 → 2 → 3); parallel pairs run concurrently.

### Phase 1 — Specification

| Step | Command | Mode | Notes |
|------|---------|------|-------|
| 1 | `/speckit.specify` | auto → approval | Creates spec.md |
| 2 | `/speckit.clarify` | interactive | Incorporate feedback |
| 3 | `/speckit.plan` | auto → approval | Architecture and approach |
| 4 | `/speckit.tasks` | auto → approval | Task breakdown with dependencies |
| 5 | `/speckit.checklist` | interactive | Requirements-quality gate |
| 5b | `/speckit.critique.run` | parallel with 5c | Plan + task quality gate |
| 5c | `/speckit.security-review` | parallel with 5b | Security review of plan/tasks |
| 6 | `/speckit.analyze` | interactive | Risk analysis |
| 7 | `/speckit.taskstoissues` | auto | Creates GitHub/GitLab issues |
| 8 | commit + tag (git, no extension) | auto | Snapshot before implementation |

### Phase 2 — Implementation

| Step | Command | Mode |
|------|---------|------|
| 9a | `/speckit.agent-assign.assign` | auto → approval |
| 9b | `/speckit.agent-assign.validate` | auto |
| 9c | `/speckit.agent-assign.execute` | auto |

### Phase 3 — Post-implementation quality (all mandatory)

| Step | Command | Mode | Notes |
|------|---------|------|-------|
| 10 | spawn `speckit-verify` agent (mode: tasks) → writes `verify-tasks-report.md` | subagent | Phantom completion detection |
| 11 | spawn `speckit-verify` agent (mode: requirements) → writes `verify-report.md` | subagent | Validate code against spec |
| 11b | `/speckit.review.run` | auto | Full review cycle; findings → fix-findings |
| 11c | `/speckit.qa.run` | auto | QA retest; failures → fix-findings |
| 12+13 | `/speckit.code-review` + `/speckit.security-review` | parallel subagents | After 11c |
| 14 | `/speckit.cleanup` | main thread | |
| 15+16 | `/speckit.sync.analyze` (scope: drift) + `/speckit.sync.conflicts` (scope: conflicts) | parallel subagents | |
| 17 | `/speckit.retro.run` | main thread | |
| 18 | Documentation update | main thread | |
| 19 | commit + tag (git, no extension) | auto | Final checkpoint |

## Scope Change (iterate)

Mandatory once tasks.md exists. Trigger: requirements change or approach won't work.

1. `/speckit.iterate.define` → `pending-iteration.md`; present to user
2. `/speckit.iterate.apply` → updates spec/plan/tasks
3. `/speckit.roadmap.write` → re-sync roadmap (mandatory after every iterate)
4. commit + tag → resume at the triggering step

## Gap Closing (converge)

| Situation | Tool |
|-----------|------|
| Spec is right, code is incomplete | `converge` |
| Spec/intent must change | `iterate` |
| Built code has a defect | `bugfix` skill |
| Review/QA surfaced findings | `fix-findings` |

`/speckit.converge`: appends remaining work as new tasks (append-only). If tasks appended, implement via agent-assign flow. If clean, resume the QA step.

## Tinyspec (small changes)

For a change one paragraph or smaller: `/speckit.tinyspec.classify` → `/speckit.tinyspec.tinyspec` → `/speckit.tinyspec.implement`. Skip full SDD.
