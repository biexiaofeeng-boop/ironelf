# Issue-Checks Collaboration Rules

## Goal

- Keep issue, implementation, validation, and handoff documents in one predictable structure.
- Align `ironelf` with the same planning workflow already used in `chimera-core`.

## Directory Layout

- Current planning documents live under `docs/Issue-Checks/YYYY-MM/`.
- Shared templates live under `docs/Issue-Checks/templates/`.
- The monthly index file is the single entry point for active work.

## Naming

- Index: `00-INDEX-YYYY-MM.md`
- Task package: `NN-TaskPackage-<topic>-v1-YYYY-MM-DD.md`
- Task cards: `NN-TaskCards-<topic>-v1-YYYY-MM-DD.md`
- Checks: `NN-Checks-<topic>-v1-YYYY-MM-DD.md`
- Prompt / SOP / split notes: `NN-<topic>-v1-YYYY-MM-DD.md`

## Workflow

1. Create or update the monthly index first.
2. Add the task package before any branch implementation starts.
3. Split cross-repo work by boundary, not by feature list.
4. Keep validation evidence in the matching `Checks` file.
5. Treat docs changes with the same branch discipline as code changes.

## Cross-Repo Rule

- `chimera-core` remains the collaboration/control plane.
- `ironelf` remains the runtime/executor plane.
- Shared bridge contracts must be documented first in `ironelf` and referenced from `chimera-core` when implementation begins.
