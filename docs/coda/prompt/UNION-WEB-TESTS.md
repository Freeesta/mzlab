Read docs/coda/BRIEF.md.

# UNION-WEB-TESTS: Test the cache-hit path of flow-web-companions and the real lr_union answer

**Goal.** Add the missing tests from #3710: the cache-hit path of `flow-web-companions.js`, and the real-worker answer shape of the `lr_union` op; and record the current sizes of the modules the step uses.

**Why.** Both paths run in the browser and are untested; a shape change would break them silently.

**Read first.** `flow/lazy/flow-web-companions.js`, `tests/test_flow_web_contract_js.py`, `tests/test_flow_web_union_js.py`, `tests/real_pool.js`, `docs/agenti/wasm-caps.md`.

**Already on main (checked 25 Nov, re-check before coding).** The two runners and a Node harness over the real `ident` exist; the cache-hit path is not covered.

**Done means.** 1. A failing-in-CI test for each (tools missing = fail, per the brief). 2. In the PR body, module, raw bytes of `mzlab-core` and `mzlab-feat` consumers as CI measures them. No cap changes.

**Out of scope.** New features.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
