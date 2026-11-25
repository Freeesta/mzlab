Read docs/coda/BRIEF.md.

# ROLE-SHORT-NAMES: Short ambiguous names as roles

**Goal.** Short names such as `dk`, `dc` or a lone `D` are neither guessed nor ignored silently: the proposal says that a short token could be a role, names it as evidence, and the user picks.

**Why.** The role words are whole tokens of a known list; a short mark is ambiguous (a dark control, a day, a replicate letter), so it is not proposed today. The roles-in-TP PR left it out on purpose.

**Read first.** `crates/mzlab-core/src/samples/names.rs` (`lone_d` and the word tables), `positions.rs`, `crates/mzlab-core/tests/samples_roles.rs`, `docs/agenti/samples.md`. Start with `grep -n 'sample file roles' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** `lone_d` reads a `d` right after a known condition only. Check how `sample import` and `sample pair` already report a file with no role word.

**Done means.** 1. A rule that lists the ambiguous short tokens found in a name set with the file and position, without changing any proposed role (a new key or a stderr line, never a changed value). 2. A test on synthetic names. 3. The note appears in `sample table` and `sample pair` output, with a `--json` key.

**Out of scope.** User word lists (row ROLE-WORDS-CLI).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
