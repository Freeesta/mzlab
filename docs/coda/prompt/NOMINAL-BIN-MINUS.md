Read docs/coda/BRIEF.md.

# NOMINAL-BIN-MINUS: nominal bins of a blank-subtracted or summed spectrum

**Goal.** `spectrum get --bin nominal|0.1` also works with `--minus FILE` (the sample minus a blank) and with `--sum`, applying the offset first and logging the rule and the offset as it does today.

**Why.** At unit resolution the sample and the blank split the same fragment between two integers; subtracting before binning leaves residues at the wrong bin.

**Read first.** `docs/agenti/data-channels.md` (the `--minus`, `--sum` and `--bin` bullets), `crates/mzlab-cli/src/spectrum_bins.rs` (`options` refuses the combination), `crates/mzlab-cli/src/spectrum_cmd.rs`, `crates/mzlab-core/src/specdiff.rs`, `nominalbin.rs`. Start with `grep -n 'spectrum get' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** `--bin` with `--minus` or `--sum` is refused with exit 2. Decide the order: bin each side and subtract the bins, or subtract the points and bin; show both on a synthetic split fragment and state the choice in the output.

**Done means.** 1. The two combinations answer with `spectrum-bins/1` plus the blank's evidence. 2. A synthetic sample and blank with a split fragment give the expected bins. 3. Route shared by site and app, parity byte test, schema keys, catalogs updated.

**Out of scope.** HR profile files (row NOMINAL-BIN-HR); spectrum comparison.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
