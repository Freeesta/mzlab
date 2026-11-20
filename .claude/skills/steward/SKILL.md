---
name: steward
description: mzLab rules for taking your own PR to green CI (reds, conflicts, reviews) and reporting to the Coordinator. Read when events arrive on an open PR opened by this chat.
---
# Taking an mzLab PR to green

The Coordinator merges the PR, never the chat that wrote it. The chat takes it to green and says so once.

1. **Red CI**: read only the log tail of the red job (`github-status` agent), reproduce locally, fix, `python3 tools/pre_pr.py`, one push. A red on main too, or in an area the PR does not touch: one line to the Coordinator (job, why it is not yours) and no empty rerun. `e2e_tour` is a known flaky: one rerun only.
2. **Conflict with main**: only then `python3 tools/fondi_main.py` (merge, never rebase or force-push); generated files are regenerated with their tool, never by hand. A clean PR is not updated with main.
3. **Reviews**: fix small, local requests; big ones (API, formats, several files) go to the Coordinator with your proposal.
4. **Communication**: no GitHub comments; everything goes to the Coordinator with `send_message`, to the session named in the prompt. At green CI send ONE message: `PR #n · 40-char SHA · CI green · what remains`. A real blocker is one line with two options, then you continue with something else.
5. **Never**: merge, enable auto-merge unless the prompt says so, push to main, skip or quarantine a test, empty commits, ask the owner.
6. **After the merge**: the next PR starts from a new branch from `origin/main`. If the context exceeds 200k tokens, send the hand-over block (at most 15 lines) and stop.
