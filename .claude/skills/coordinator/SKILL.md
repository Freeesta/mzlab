---
name: coordinator
description: How the mzLab Coordinator chat watches sessions and PRs, spawns DEV chats and is the only one that merges. Read when acting as Coordinator.
---
# Coordinator

The Coordinator is the only one who merges. DEV chats never do (`steward` skill).

**Watch, without filling your context**
- Sessions: the `session-status` agent (one row per session; flags those above 200k of context).
- PRs, CI, Pages, leftover branches: the `github-status` agent (head SHA check runs; red jobs by log tail only).
- Failing local verification: the `verifier` agent.

**Spawn work**: one DEV chat per task, one task per chat (`create_session`, then a prompt that points to the `dev-task` skill and states the task, the area and the branch topic). Several tasks in parallel only when they touch different files.

**Merge** (`docs/agenti/ci-fast.md` for what the checks mean)
1. Get the PR's head SHA from `github-status` (40 characters; the DEV's one-line report carries it).
2. `python3 tools/coo_merge.py PR SHA` (needs `git fetch` access to `origin`; stat-ci branch events). It refuses when the PR head differs from SHA, when `test` or `rust` is not green on that SHA, when a local merge with `origin/main` conflicts or fails `tools/controlla.py` / `tools/indice.py --verify`, or when the PR touches a risky path (`Cargo.toml`, `Cargo.lock`, `.github/workflows/*`, `*.wasm`, `crates/mzlab-desktop/*`, the CSP in `mzlab/web/index.html`). Last stdout line is JSON `{pr, sha, verde, combine, rischi, unire, motivo}`; exit 0 only when `unire` is true.
3. Merge only on exit 0, with the same SHA. A risky path needs your explicit go: say what and why, then decide.
4. A clean PR is not updated with main. A real conflict goes back to the DEV chat: `python3 tools/fondi_main.py` (merge, never rebase or force-push).
5. After the merge: stop; no scheduled check-ins, no polling.
