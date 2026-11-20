---
name: night-triage
description: Triage of a failed nightly (notte.yml) run: read failed-job log tails, classify each red as known, load or real, without skipping tests.
---
# Night triage (`notte.yml`)

1. List the latest `notte` runs and failed jobs with the `github-status` agent. Read only the failed-job log tails (`failed_only`, `tail_lines=60`), never whole logs. The full round is `completa (K/5)`; each shard uploads `notte-completa-K`.
2. For each red e2e name, check `tools/noti_rossi.txt` (`name | reason`; listed reds show as NOTO and do not fail a run). Listed: **known**.
3. Not listed: is it yours or main's? Per `docs/agenti/cloud-environment.md` ("Is a red yours?"): rerun the same `python3 tools/verifica.py --solo <name>` on a worktree of `origin/main` (`git worktree add --detach <folder> origin/main`). Red there too: **real, on main**. A red that passes alone on a second try: **load** (`e2e_tour`, `e2e_tocco`, `e2e_hr_scanbin` are the known load-sensitive ones).
4. Classify each red as known / load / real. For a real one give: test name, decisive log line, first bad commit if found, and propose the fix; the cause is fixed in code or in the test in the same PR that changed the text, id or button.
5. Never skip, disable or quarantine a test. A fixed test loses its line in `tools/noti_rossi.txt`.
6. `tools/ci_e2e_verdi.json` (the safety-set allow-list of the `e2e-cambiati` job) is only rewritten with `python3 tools/ci_e2e_cambiati.py --verdi <ultimo.md of each shard>` from the `notte-completa-K` artifacts, never by hand.
7. Report a table: job / test, class, evidence, action.
