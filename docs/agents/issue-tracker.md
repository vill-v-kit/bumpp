# Issue tracker: Linear

Issues and PRDs for this repo live in Linear. Use the authenticated Linear MCP tools for all operations.

## Scope

- Default project: `villv-bump`
- Resolve the owning Linear team from the project metadata.
- If the project belongs to multiple teams and the target is ambiguous, ask before creating an issue.
- The Gitee remote is the code host, not the issue tracker.

## Conventions

- Search the `villv-bump` project for an existing issue before creating one.
- Create new issues in the `villv-bump` project.
- Read the issue description, status, labels, assignee, relations, and comments.
- Add comments for progress or decisions instead of rewriting history.
- Apply triage labels according to `docs/agents/triage-labels.md`.
- Preserve the project's existing Linear workflow statuses.
- Close an issue only after recording the outcome.

## When a skill says "publish to the issue tracker"

Create a Linear issue in the `villv-bump` project.

## When a skill says "fetch the relevant ticket"

Fetch the Linear issue by identifier or URL, including its comments and relations.

## Wayfinding operations

- **Map**: one Linear issue labelled `wayfinder:map`.
- **Child ticket**: a related or sub-issue labelled `wayfinder:<type>`.
- **Blocking**: use Linear's blocking relations; fall back to a `Blocked by:` line.
- **Frontier**: choose the first open, unblocked, unassigned child.
- **Claim**: assign the issue to the driving developer before starting work.
- **Resolve**: comment with the answer, close the child, then update the map.
