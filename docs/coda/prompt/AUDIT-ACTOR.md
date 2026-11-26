Read docs/coda/BRIEF.md.

# AUDIT-ACTOR: Who made each choice: user, auto or agent

**Goal.** Every decision and automatic choice in the saved logs carries `actor` (`user`, `auto`, `agent`); an agent can propose but never overwrite a decision whose actor is `user` (refused with an `err.*` key and listed).

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 14 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §A (log line).

**Read first.** `docs/agenti/llm.md`, `llm-choices.md`, `tp.review` code, `feature_decisions.csv` writer, `logOp` in the site, `prov/1` `llm` block.

**Already on main (checked 26 Nov, re-check before coding).** Assistant actions are marked separately; no common actor field.

**Done means.** 1. The field in Rust formats with a format version step. 2. The refusal with its test. 3. Site, app and CLI the same bytes.

**Out of scope.** Rewriting old logs.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
