Read docs/coda/BRIEF.md.

# CENTROID-LABEL: Detect a mislabelled centroid or profile file and relabel it, logged

**Goal.** Detect an mzML labelled 'profile' that holds centroids (or the reverse) from the point spacing and peak shape, report the evidence, and relabel with the user's choice logged.

**Why.** Measured in the lab: mzML labelled profile made centroiding take 60-70 % of the run, and another tool lost about 70 % of the counts until the file was relabelled (the numbers are private: ask the coordinator).

**Read first.** `crates/mzlab-cli/src/raw_cmd.rs` (`warn.raw.labelMismatch`), `docs/agenti/raw-access.md`, `crates/mzlab-cli/src/convert_help.rs` (`--peaks file|profile|mzlab`, `--spectrum-type`), `crates/mzlab-cli/src/flow_centroid.rs`, `docs/agenti/raw-mzml.md`.

**Already on main (checked 25 Nov, re-check before coding).** `raw info` already warns `labelMismatch` on probed scans. Conversion can write centroid or profile as asked. Missing: the same detection in the places that cost time (`file info`, `flow run` centroiding, `file convert`) and a clear relabel path with a logged choice. Check `grep -n 'labelMismatch' -r crates --include=*.rs`.

**Done means.** 1. One detector function (point spacing, shape) in core with its evidence, used by `raw info`, `file info` and the centroiding step; the step skips centroiding for data that is already centroid, logging it as an automatic choice. 2. Relabel via a flag with the old and new label in the conversion report. 3. Synthetic files: profile labelled centroid and the reverse. Parity site=app=CLI.

**Out of scope.** Changing how real profile data is centroided.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
