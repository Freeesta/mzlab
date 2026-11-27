Read docs/coda/BRIEF.md.

# AI-FEATURE-PROPOSAL: The assistant proposes structures for one feature

**Goal.** On request only, the assistant reads the text card of one feature (shown to the user first, sent by the user), proposes 1-3 structures (SMILES checked by `mol check`) with pros and cons; the user accepts, edits or discards; the level stays the user's.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 16 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `docs/agenti/llm-choices.md`, `llm.md`, `assistant-proxy.md`, `crates/mzlab-llm/`.

**Already on main (checked 26 Nov, re-check before coding).** Opt-in assistant with policies and logs exists; no structure task.

**Done means.** 1. The task and its context built in Rust from the card. 2. Proposals stored with actor `agent`; never a level. 3. CLI `llm` route and tests.

**Out of scope.** A local model.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
