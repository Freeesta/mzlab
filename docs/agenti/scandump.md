# scandump/1: the per-scan dump, written the same by RFR and by our reader

The RFR oracle (Thermo RawFileReader, only on the Mac, never in the repository) and our reader write the same format; `tools/rosina/scandump_compare.py` compares them field by field. Oracle rules: `thermo-speed.md`. Outputs: `thermo_scandump` (ours), `scandump_compare.py` (comparison and aggregate), `verita_scandump.py` (→ `verita/1` for the F0-F5 bench), test `crates/mzlab-lettori/tests/scandump_thermo.rs` (RFR dump of the private data in `scratchpad/rfr/<file>.scandump.jsonl.gz`, skipped without `MZLAB_DATI`).

## Format rules
- JSON Lines, UTF-8, gzip (`.scandump.jsonl.gz`); each line is an object with `type`. Keys in English, times in minutes, m/z in Th, intensities as the file gives them.
- **A field the dump producer does not expose is omitted** (never `null`, never an invented value). An empty list is information (e.g. `reactions: []` in an MS1).
- Order: `file` first, then the `scan` lines by increasing `n`; the optional lines after, in any order.

## `file` line (one, the first)
`schema` (`scandump/1`), `name` (logical name, no path), `sha256`, `format_version` (version of the `.raw` format), `instrument` (model), `controller` (index of the MS controller), `n_scans`, `producer` (`rfr` or `mzlab`), `producer_version`, `open_ms` (open and index), `read_all_ms` (read of all scans), `read_one_ms` (one scan in the middle). With `--downstream` also `downstream: true`.

## `scan` line (one per scan)
- `n` (scan number in the file), `rt`, `level`, `polarity` (`+` or `-`), `analyzer` (ITMS, TQMS, SQMS, TOFMS, FTMS, Sector), `mode` (`profile` or `centroid`), `filter` (Thermo filter string), `scan_type` (full, zoom, sim, srm, crm, q1, q3), `segment`, `event` (as in the scan index), `master` (number of the scan the precursor is chosen from), `tic`, `bp_mz`, `bp_int`, `mz_low`, `mz_high`, `n_peaks`.
- `reactions[]`, one entry per reaction, in order: `precursor` (isolation centre), `iso_width`, `iso_offset`, `activation` (CID, HCD, ETD, EThcD, UVPD, ...), `energy`, `energy_unit` (`NCE` or `eV`, omitted if the file does not say), `multiple` (boolean), `supplemental` (supplemental energy). Empty in MS1. In SRM scans the activation is `Any`, as RFR gives it.
- `sps_precursors[]` (m/z of the SPS-MS3 precursors), `faims_cv`.
- `trailer`: object label → value as text; labels without the trailing colon, in file order.
- With `--downstream` (ours only) the `down` object: `filter` (filter as it comes out of the reader's index), `mzml` (`precursors[]` read from the written mzML: `spectrum_ref`, `selected_mz`, `iso_target`, `iso_lower`, `iso_upper`, `charge`, `activation[]`, `energy{name,value}`), `rawfilter` (`spectrum_ref` of the first precursor after `MzmlFilter`), `mzidx` (`level`, `precursor`, `rt`: the row of the `.mzidx` table), `msngraph` (`node`, `level`, `chain[]` of `mz`, `activation`, `energy`: the entry node of `msn-graph/1`).

## Optional lines
- `peaks`: `n`, `mz[]`, `int[]` for 8 chosen scans (first, last, median, maximum TIC, one per level); all of them with `--full`.
- `chromatogram`: `id`, `kind` (tic, bpc, pressure, analog, pda, ...), `unit`, `time[]`, `values[]`.
- `status`: `scan`, `rt`, `values{label: text}`, sampled (at most 16 instants).
- `error`: `rt`, `message` (one per entry of the error log).
- `sequence`: the sequence row (`row_number`, `injection_vial`, `injection_volume`, `sample_weight`, `sample_volume`, `istd_amount`, `dilution_factor`, `id`, `comment`, `instrument_method`, `processing_method`, `raw_file_name`, `path`, `vial`, `user_text[]`).
  Our reader adds `unknown_numbers{}` and `unknown_texts{}` by position (fields whose meaning is unknown, extras for the comparator: they serve to pair them with Sample Type, Level, calibration of the RFR dump). `asinfo`: `preamble` (24 bytes in hexadecimal), `text`.
  Per scan our reader adds `raw_records[]` (bench extra): all the records of the raw event in order, even with precursor ≤ 0, `{precursor, energy, flags, double, long2}`; written only with at least 2 records.
- `method`: `block` (index of the text block), `length`, `sha256`, and `text` only with `--text`.
- `tune`: `values{label: text}` (our reader does not expose it: omitted; the place in the file with the tune values per segment was not found without an RFR dump).

## Comparison thresholds (F.2)
RT 1e-4 min; m/z 0.5 ppm (`--tol hr`) or 1e-3 Da (`--tol lr`); intensity 1e-5 relative; text equal after normalisation (spaces, case, trailing colons of labels). A field present only in B (ours) is `extra`, not an error. Lines that are not scans (`sequence`, `asinfo`, `error`, `status`, `tune`) are compared key by key like the trailer (`<type>.<key>`; `status` per sampled instant).
The reaction RFR gives to Orbitrap/Q Exactive MS1 (precursor = centre of a range, isolation = width, HCD, energy 0) is not a precursor: the comparator omits it on both sides (`reactions_of`) when an MS1 with ONE reaction matches (within 0.5) `[mz_low-mz_high]` or one of the `[a-b]` ranges of the `filter` text; for MS2+ the rule does not apply.

## Commands
```
cargo run --release -p mzlab-lettori --features thermo --example thermo_scandump -- FILE.raw OUT.scandump.jsonl.gz [--full] [--text] [--downstream]
python3 tools/rosina/scandump_compare.py RFR.jsonl.gz NOSTRO.jsonl.gz [--tol hr|lr] [--out report.json] [--aggregate DIR]
python3 tools/rosina/verita_scandump.py DUMP.jsonl.gz > FILE.verita.json
```
`--aggregate DIR` gathers the `report.json` files of the folder into a field × file table (counts only, no private file name in the public repository). Our dump writes in `reactions[]` one element per step of the chain (`PrecursorInfo::reactions`, MS2…MS8): `iso_width` only for the final step (from the trailer), `multiple` for simultaneous precursors (MSX, multiband), `sps_precursors` and `faims_cv` from the trailer.
Non-final elements also carry `raw_double` and `raw_long2` (bench extras, not RFR). For MS3+ the comparator also writes the per-level fields `reactions.<field>@ms3…@ms8` and compares the `iso_width` / `iso_offset` of RFR with `raw_double` / `raw_long2` of the non-final steps (`reactions.iso_width~raw_double@ms3`): when a level is equal over the whole corpus, `raw_double` becomes `isolation_width` for every step (`reaction.rs`); until RFR dumps of MS3+ exist (R-03) they stay "not verified against RFR".

## Status of the comparisons per level (MSn)
No RFR dump of MS3+ exists in `scratchpad/rfr/`: the fields `reactions.*@ms3…@ms8`, `sps_precursors`, `faims_cv` and the activations other than CID/HCD (ETD, ECD, PQD, UVPD, MPD) are proven only on events built in the tests (`reaction.rs`, `scan_filter.rs`, `mzml.rs`, `tests/test_scandump_compare.py`) and `VERIFICATI` stays empty. The per-step filter changes only the MSn with activations that differ between steps. Dumps requested from the local chat (accessions): MSn infusion up to MS8, Fusion/Tribrid DDA with CID+HCD, SPS-MS3 (TMT), ETD/EThcD/UVPD/PQD if in the corpus, FAIMS.
