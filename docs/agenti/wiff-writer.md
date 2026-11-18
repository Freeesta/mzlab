# `.wiff` writer: design study and the read-only inventory (`wiff list`, `wiff verify`); the writer itself is not built

Goal of the owner: create a `.wiff` + `.wiff.scan` pair, add or remove samples (recombine samples of different pairs), lose nothing the reader sees (or anything else in the samples), and have Analyst 1.6.3 still open the result.
Source of every statement below: the reader (`crates/mzlab-core/src/wiff/`, `ole.rs`), `wiff.md`, `wiff-full-spec.md`, `wiff-variants.md` and the synthetic fixtures of `tools/wiff_riferimento.py`. Nothing here was checked on a real file or in Analyst: no lab data is reachable from a cloud chat (`MZLAB_DATI` empty). Where the repo does not show something it says UNKNOWN and names the experiment (E1..E8, last section).

## Verdict
- Feasible in principle, and the only safe design is a byte-level container rewrite: copy the streams and `.scan` sections of every kept sample verbatim, rebuild only the structures that name positions (OLE directory/FAT, `SampleIdxTable`, numbers).
- Two things decide "Analyst still opens it" and the repo cannot answer either: what else in a real file refers to the sample list (E1-E5, can be settled with a read-only inventory of real files), and Analyst's own acceptance (E6, only the owner can run it, checklist below).
- Recommended plan: no decode/re-encode anywhere; staged delivery, each stage gated by an owner test in Analyst: identity copy → remove a sample → merge samples of two files.

## (a) What makes up one sample, what is file-global
Known from the reader. Container: OLE2 file `.wiff` (read whole into memory, small) + `.wiff.scan` (big, read by position). `.wiff` is never edited in place; a rewrite always makes a new pair.

| Item | Scope | Evidence | Rewrite rule |
|---|---|---|---|
| `SampleSubtree/SampleN/Idx` (32 B header + 54 B rows: u32 offset from section start, u32 size, f64 time ms, f64 TIC, f64 points) | per sample | `idx.rs` | copy verbatim: offsets are RELATIVE to the section, so a section can move in the `.scan` without touching Idx |
| section of sample N in the `.wiff.scan`: starts at `B - 20` (B = table position), blocks at `section + offset + 44`, runs to the next section | per sample | `idx.rs` (`TABLE_LEAD` 20, `PAYLOAD_SKIP` 44), `unindexed.rs` (marker `11 11 11 11 82 05 00 00` + u32 sample number, found on partial copies) | copy the byte range verbatim; end = next section start or end of file (UNKNOWN: trailer, E1) |
| `SampleSubtree/SampleIdxTable` (32 B header, 16 B rows: u32 number, u32 B) | file-global | `idx.rs` | REBUILT: only B changes (new section start + 20), numbers as the plan says |
| `SampleN/Log`, `SampleN/SampleDABE/DATA` (name, `.dam` name, vial) and `SampleDABE/CFR_INFO`, `SampleN/Devices/Device_K/{Channel, DevRealTimeHeader, DevData}`, `.../DAD/{DADSampleHeader, DADIdx, DADRealTimeData}`, `SampleN/DDERealTimeData` | per sample | `wiff.md`, `dad.md`, spec §1, §3, §4 | copy verbatim |
| `SampleN/Wiff2SampleInfo` / `WiffNSampleInfo` (u32 sample number at +32, sample GUID, batch GUID), `Itc`, `TDCInfo`, `TDCStatistics`, `TOFCalibrationData` | per sample, TOF / SCIEX OS only | spec §4.3-4.4 | out of scope for phase 2 (TOF and `.wiff2` are refused by the reader); the number at +32 would need renumbering |
| `MethodSubtree/MethodM/...` (`DeviceMethod0/Period0/ExperimentI/{ExperimentHeader, MassRangeEx, sMRM, IonSourceParamsTable}`, `DeviceMethod1/{VendorAppMethod, LCPumpStepK}`, `GLPTables` with the Q1/Q3 calibration tables, `AcqMethodFileInfoStm`) | per METHOD, shared by samples | `methodinfo`, `wiff-variants.md` | copy with the sample(s) that use it (see method sharing below) |
| `SampleSubtree/DabsInfo`, `SampleSubtree/SampleTable` (strings `s1`, `s2`...: sample name, plate, position) | file-global batch description | `method_wiff/extended.rs` (`campi`), read only as text | UNKNOWN whether Analyst needs it consistent with the sample list (E5) |
| `SummaryInformation` (creation date, the mzML `startTimeStamp`) | file-global | `wiff.md` | keep from the first source or leave out: owner decision (D4) |
| OLE directory entries (names, CLSID, creation/modified times, state bits), FAT, DIFAT, mini FAT, sector size | container | `ole.rs` keeps only name, type, links, start, size | rebuilt; CLSID/times/state bits of every kept entry copied (UNKNOWN whether Analyst stamps per-stream times, E2) |

- Sample-number occurrences found in the repo: the table row, the storage name `SampleN`, the section marker (u32 after the 8 marker bytes; seen only on partial copies), `Wiff2SampleInfo` +32, and the name `MethodN` when a method is named after its sample. Others UNKNOWN (E3: search every stream of a real file for the u32 of each sample number).
- Method sharing: a batch has one `MethodM` per distinct method (fewer than samples). The reader chooses a sample's method by the `.dam` name (`SampleDABE/DATA`, `AcqMethodFileInfoStm`), else the nearest `MethodN` below the sample (`wiff::method_no`, `borrow_agrees`). Renumbering a method can change which one a sample resolves to. Simplest rule that keeps the reader's answer: give each written sample its own method storage numbered like the sample, byte-copied (loses sharing; whether Analyst needs sharing is UNKNOWN, E7). Decision D3.
- Empty injection (table row with B < 20, no `Idx`): a sample with no section; it is copied as such (row kept, no streams), never dropped.

## (b) What the reader ignores today (unparsed bytes = potential loss)
Measured with a throwaway script over the 32 synthetic `.wiff` of `tools/wiff_riferimento.py` (204 streams, 107 821 stream bytes, 17 stream classes):
- every one of the 17 stream classes is read by some code path; no whole stream class is unread in the fixtures. This proves little: the fixtures hold only what our own writer put in them. Real files carry streams the fixtures never have and the reader only touches as text or not at all: `SummaryInformation`, `DabsInfo`, `SampleTable`, `GLPTables`, `CFR_INFO`, `AcqMethodFileInfoStm`. The real list needs E1/E2 on a real file.
- inside streams, bytes no code looks at: the 32-byte OLE stream header of every stream (6 392 B, 5.9 %); in `Idx` the u8 at +16 and the 20 bytes +34..+53 of each 54-byte row (21 B x 1 185 rows = 24 885 B, 23 % of all fixture bytes; all zero in the fixtures, and per the spec also zero on QTRAP, but TOF rows use +34 and +46); in `ExperimentHeader` about 20 of 180 bytes are used (+42, +86, +122, +176): 53 headers, 9 540 B, about 89 % unparsed (the spec names the experiment name at +90 and the MRM marker at +120, the reader does not use them).
- in the `.wiff.scan`: bytes inside a section but outside any block (the 44-byte section lead, the 4-byte padding after a block stream, gaps between `offset+size` and the next row) are never decoded; bytes outside every section (file header, trailer) are UNKNOWN: the reader only enters the file through table positions (E1).
- container level: directory times/CLSID/state bits, FAT layout, free sectors are not read.
- A decode/re-encode writer would drop all of the above silently. A verbatim copy keeps all of it by construction, whatever the reader understands.

## (c) Safest approach
Reject decode/re-encode. Reasons: (1) the nibble codec writer of the instrument is not canonical (long/short forms, the 3200 pads a stream of `u` bytes to `u + 4 - u % 4`), so a re-encode cannot be byte-identical and any difference is unprovable against Analyst; (2) it gains nothing, since no sample content has to change; (3) it cannot carry the unparsed bytes of (b). Keep: one rule, "a kept sample is copied, never interpreted".
Design (all logic in Rust, `crates/mzlab-core/src/wiffwrite/`, streaming through `ReadAt` / `WriteAt` like `mzmlwrite`):
1. Plan: a list of output samples, each = (source pair, source sample number, output number, method source). Pure data, printable, checked before any byte is written.
2. `.wiff.scan`: write the sections in plan order, each copied byte-exactly from its source range (the `.scan` is never loaded whole; 4 MiB blocks as in `blocks.rs`); remember each new start.
3. `.wiff`: a new OLE2 file. Streams of the kept samples and their methods copied byte-exactly with their directory metadata; `SampleIdxTable` rebuilt from the new starts; the file-global streams per D4/D5. Needs an OLE WRITER in core: the repo has only test writers (`tools/cfb_write.py`, `mzlab-lettori/tests/common/ole.rs`: 512-byte sectors, no DIFAT, about 7 MB cap, no mini stream in the Rust one). The production writer needs version-3 and version-4 sectors, DIFAT, mini stream below the 4096 cutoff, a valid directory tree (the reader walks any tree), and the source's sector size.
4. Output is a new pair beside nothing existing: written to a temp name, renamed when complete, never over an input, never over an existing file without `--force`. The inputs are never modified.
5. Refuse, with a named error and nothing written: TOF, `.wiff2`, more than one period, sections the index does not list (`unindexed.rs`), a sample the reader marks damaged or truncated (the copy would carry the damage; the report says which). Policy for each is D6.
6. The writer reads sources through the same `Ole`/`ReadAt` code the reader uses, so a sample the reader opens is one the writer can place; anything the reader cannot place is listed, not guessed.

## (d) How "nothing lost" is verified, and what cannot be
Automatic (a Rust integration test, later phase, synthetic data only):
- split: every sample of a multi-sample fixture (`nomi`, `gaps`, `two`, `prodotto`, `dadmetodo`, `dispositivi`, `variante`, `ida`) into its own pair; merge them back in the original order; compare with the source.
- raw level, per sample: every stream under `SampleN` and its method byte-equal (after the number mapping); the `.scan` section byte range equal (SHA-256); directory metadata of every kept entry equal; free-form extra streams injected into a fixture (random names and bytes) must survive (the canary for "unparsed stream").
- decoded level: `Wiff::run` of each sample gives equal spectra, chromatograms, polarity, CE, precursors, `metodo/1`, names, `losses` zero; also the DAD and device traces.
- operations: remove one sample (the rest equal, `.scan` smaller by exactly that section); add (the new sample equal to its source); every permutation and subset of 3 samples; the same plan twice gives identical bytes (determinism); an empty injection survives.
- parity: site wasm (in Node), app native and CLI write the same SHA-256 for the same plan (the `test_wiff_*_parita.py` style).
- evidence for real files, usable by the owner without Analyst: `mzlab-cli wiff verify` (below) re-reads the output and each source and compares stream and section SHA-256 per sample; the JSON is the proof that every byte of every kept sample is present.
Cannot be verified here: that Analyst 1.6.3 opens the file, and everything depending on real-file layout not present in the fixtures (E1-E5). A green synthetic round trip says the writer is lossless for what our fixtures contain, not that Analyst accepts it.
Checklist for the owner (do it on copies, never on the originals; Analyst 1.6.3 is Windows software: the machine that has it):
1. Identity: `mzlab-cli wiff write --out copy.wiff one.wiff` (all samples, same order). Open `copy.wiff` in Analyst (Explore). Note any dialog text and send it exactly.
2. Compare with the original, per sample: sample name and order in the list; the TIC overlaid on the original's (same shape and intensity); a spectrum at one fixed RT; for an MS/MS sample the CE in the spectrum properties; the acquisition method view (transitions, CE, dwell); the instrument calibration/properties; the acquisition date and time shown.
3. Removal: write a pair without one sample; repeat 1-2; check the samples left are untouched.
4. Merge: two samples from two different files in one pair; repeat 1-2 for both; open them in Quantitation (peak areas of one transition equal the original's).
5. In Analyst, save the opened result under another name and reopen it (Analyst can rewrite it).
6. If any step fails: stop, send the dialog text, the output of `mzlab-cli wiff list --json` of the input and `python3 tools/wiff_dump.py FILE.wiff --all` (without `--log`, which may hold the operator's name). Do not go on to the next stage.

## (e) Risks and decision points for the owner
Risks:
- R1 Analyst may check something the reader never looks at (a checksum, a count of samples, a batch table, per-stream times). Mitigation: verbatim copy of everything per-sample, staged owner tests, E1-E5 inventory before the first merge.
- R2 Hidden cross references to the sample number or order (E3, E5). If found, they are rewritten by the plan, not guessed.
- R3 A merged file mixes instruments, methods, tunings, calibrations. The calibration travels in the method's `GLPTables`; the file-global header describes only one source.
- R4 Audit trail: `CFR_INFO` suggests Analyst's 21 CFR 11 mode. A recombined file is a modified record. For regulated work the output must be marked as derived. Not a technical limit, but the owner's call (D5).
- R5 Size: `.scan` can reach many GB; streaming only, the OPFS quota limits the site, the app and CLI have no such limit.
- R6 Real files may contain partial, unindexed or damaged sections (`unindexed.rs`, `losses`): the default is refuse and report, not copy silently.
Decisions asked (recommendation first):
- D1 Scope of phase 2: 3200 QTRAP / Analyst 1.6.3 only (period 0, blocks v1-3; the OLE version of real files is UNKNOWN until E1, the writer supports 3 and 4); TOF, `.wiff2`, several periods refused. Recommended.
- D2 Numbering: keep each source's number when unique, else renumber sequentially, always printed in the plan (`--renumber keep|sequential`).
- D3 Methods: one byte-copied method per sample (keeps the reader's resolution) or dedupe identical methods (smaller, risks the resolution rule). Recommended: per sample, until E7.
- D4 File-global streams (`SummaryInformation`, `DabsInfo`, `SampleTable`): copy from the first source (recommended, consistent with the output starting from a real file), leave out, or rebuild `SampleTable` from the plan if E5 shows Analyst reads it.
- D5 Provenance: write a sidecar `OUT.wiff.mzlab.json` (sources, SHA-256, plan; recommended, Analyst cannot see it) or also an extra stream inside the `.wiff` (traceable inside the file, but an unknown stream in an Analyst file is itself a risk).
- D6 Damaged, truncated or unindexed samples: refuse (recommended) or copy with a warning.
- D7 New wasm module (see below).

## CLI shape (`mzlab-cli wiff`, same logic as site and app; usable by agents)
- Built (read only, feature `wiffwrite`, `crates/mzlab-core/src/wiff/inventory.rs`, `ole/inventory.rs`, `crates/mzlab-cli/src/wiff_cmd.rs`): `mzlab-cli wiff list IN.wiff... [--inventory] [--json]`, schema `wiff.list/1`. Per file: name, SHA-256 of the `.wiff`, `container` (OLE version, sector sizes, bytes), `scan` (bytes, `head_bytes` before the first section, `unindexed_skipped`), `global_streams`.
  Per sample: `number`, `name`, `method`, `scans`, `status` (`ok|refused|truncated|damaged|unindexed`), `reason`, `section` (start, bytes, `slack` after the last block, SHA-256, `lead_hex` of the first 44 bytes), `idx` (rows, zero rows), `streams` (path, bytes, SHA-256).
- `--inventory` adds the SHA-256 of the whole `.wiff.scan` and `directory` (every entry: kind, bytes, CLSID, state bits, creation and modification FILETIME as hex). A refused sample is listed with its reason, never hidden: a code the reader refuses, a TOF experiment or block, periods, an unknown block version. Exit 3 when any sample is not `ok`.
- Built: `mzlab-cli wiff verify OUT.wiff --against SRC.wiff...`, schema `wiff.verify/1`. Each output sample is matched to the source sample with the same section SHA-256 (an empty injection: the same name) and compared stream by stream (path without the sample and method numbers) and on the directory metadata; the streams shared by all samples must equal those of the first source; `equal` is the answer and `differences` the evidence. Exit 0 equal, 1 any difference or a sample that is not `ok`.
- The site serves the same two functions from the lazily loaded module `wiffwrite` (`wiff_list`, `wiff_verify`, `crates/mzlab-lettori-wasm/src/wiffwrite.rs`); `tests/test_wiff_inventory_wasm.py` compares its text with the CLI's byte for byte.
- Planned, not built:
- `mzlab-cli wiff write --out OUT.wiff SRC.wiff[#N[,M..]]... [--exclude SRC.wiff#N]... [--renumber keep|sequential] [--dry-run] [--force] [--json]`: positional sources in output order (no `#`: every sample of that file); `--dry-run` prints the plan (`wiff.plan/1`: output number, source, bytes copied, method, collisions, refusals, projected sizes) and writes nothing; a real run prints `wiff.write/1` (the plan plus SHA-256 of both outputs, `verified`).
- `mzlab-cli wiff split IN.wiff --out DIR` (one pair per sample; sugar over `write`).
- Streams: stdout one JSON document with `--json`, progress and warnings on stderr, texts through `lang/` keys (`err.wiff.*`, `warn.wiff.*`).
- Exit codes (the pattern of `validate` and `flow`): 0 done and verified; 1 done but `verify` found a difference (the output is removed, nothing is left behind); 2 usage, read or write error; 3 plan refused (unsupported variant, damaged section, unresolved collision): nothing written.
- Writes are atomic (temp name then rename), refuse an output equal to an input, refuse to overwrite without `--force`.

## Site / app parity plan and wasm cap
- Parity rule (CLAUDE.md): the core function exists in the site (`.wasm` + OPFS) and the app (native + CLI), with a byte-for-byte test: CLI, native app command and wasm (run in Node on the same plan) give the same SHA-256 for `.wiff` and `.wiff.scan`. One half alone is incomplete unless a technical limit is stated in the PR body. The Flow does not need a step (no feature-table input or output); to be confirmed by the owner if he wants one.
- Site: wasm exports write through a `WriteAt` sink over OPFS (no function returns a whole `Vec<u8>`, `tools/controlla.py`); the `.wiff` is read whole, the `.wiff.scan` by 4 MiB blocks. UI in a later PR: select samples in the Files panel (several `.wiff` loaded), «Combine into .wiff», plan preview identical to `--dry-run`, download of the pair. App: Tauri command calling the same core; the desktop app opens a `.wiff` through its `.wiff.scan` companion (`mzlab-desktop/src/opening.rs`); the write side is a new command there.
- Wasm cap (numbers from `tools/moduli/*.toml`; the writer's size is an ESTIMATE, to be measured with `python3 tools/moduli.py --misura` in the writer PR): OLE writer + plan + copy loops about 1 000-1 400 lines of Rust, estimated +20 to +45 KB optimized.
  - in `lettori-sciex` (485 158 B, cap 486 400, room 1 242 B): +4 to +9 %, far above the 1 % rule.
  - in `motore` (2 147 104 B, cap 2 183 168, room 36 064 B): +1 to +2 % of the module; the low end fits under the cap, the high end passes it by less than the 1 % allowance (21 832 B), the middle is unclear until measured.
  - the module `wiffwrite` (like the `lettori-*` modules, loaded only on use) holds the inventory today and takes the writer next, with its own entry `tools/moduli/lettori-wiffwrite.toml`; no existing cap changes (owner decision D7). Its numbers are in `wasm-caps-measurements.md`.
  - No existing cap or limit is changed.

## Unknowns and experiments
- E1 `.wiff.scan` coverage map of real files: bytes before the first section, between sections, after the last, and what the 44-byte section lead and the 12 bytes after the marker hold. Read-only tool in the first implementation PR (`wiff list --inventory`), run by the owner on his files (or in the private data chat).
- E2 Directory entry metadata (CLSID, times, state bits) of a real file, and whether per-stream times differ between samples.
- E3 Every place the sample number (u32 and as text) appears in a real file, beyond the four known.
- E4 Whether `Idx` row 0 always has offset 0, and whether any block is referenced from outside `Idx`.
- E5 Content of `DabsInfo` and `SampleTable` against the number and names of the samples; whether editing the batch changes them.
- E6 Analyst 1.6.3 opens the identity copy, a removal, a merge (owner checklist above).
- E7 Whether two samples of different files that were given the same method name can be told apart by Analyst; whether Analyst needs shared methods.
- E8 How often real files hold unindexed sections or truncated `.scan` tails (the reader's `losses` already counts them).

## Next steps (each one PR, after the owner answers D1-D7)
1. Done: `wiff list --json --inventory` and `wiff verify`, read only, no OLE writer; the owner runs them on real files to settle E1-E5 (E3, every place a sample number occurs, is not searched by the tool).
2. OLE writer in core with an identity copy of one sample file and its round-trip test; owner checklist step 1.
3. Remove and add, CLI first; owner checklist steps 3-5.
4. Site and app surfaces with the parity tests.
