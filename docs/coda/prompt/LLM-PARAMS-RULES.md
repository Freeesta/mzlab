Read docs/coda/BRIEF.md.

# LLM-PARAMS-RULES: llm params: merge the estimate rules and fix the blank rule text

**Goal.** Merge the rules of `llm params` with the evidence of `estimate`, and correct the text of rule R-blank-1.

**Why.** Two places propose the same parameters with different words (follow-ups of #3709 and #3711).

**Read first.** `docs/agenti/llm-params.md`, `crates/mzlab-llm/src/params_rules.rs` and `params_rules_tab.rs` (`R-blank-1` maps to `table.filter blank_ratio`), `docs/agenti/estimate.md`. Start with `grep -n 'llm params\|llm-params' docs/agenti/INDICE.md`, then `grep -n 'R-blank' crates/mzlab-llm/src/params_rules.rs`.

**Already on main (checked 25 Nov, re-check before coding).** Both exist. `counts.blank` now also exists for the plain filter, so the R-blank-1 text may describe an older behaviour; read it against `crates/mzlab-feat/src/view.rs::blank_evidence`.

**Done means.** 1. One source of the rule texts; `llm params` quotes the estimate evidence when files are given. 2. R-blank-1 text matches the code. 3. Golden proposals regenerated with their tool only if the text changed (state which). Parity site=app=CLI.

**Out of scope.** New rules.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
