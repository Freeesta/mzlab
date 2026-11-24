# `file pack`, `file unpack`, `file check-pack`: a campaign as split parts (`pack/1`)

Many files (a `.D` folder, a `.wiff` + `.wiff.scan`, a whole campaign) become **parts** of a fixed size plus ONE manifest, so they can be sent over a link that takes small files
and rebuilt and checked on the other side. Code: `crates/mzlab-core/src/{zipstream,pack,pack_read}.rs` (all logic, generic over small traits), `crates/mzlab-cli/src/pack_cmd.rs`
(files), `crates/mzlab-pack-wasm` + `mzlab/web/rust-pack.js` (site, OPFS). Tests: `pack_tests.rs`, `pack_cmd_tests.rs`, `tests/test_parita_pack.py` (wasm vs CLI, byte for byte).

## Commands
- `file pack PATH... --out DIR [--part-size 25MB] [--name NAME] [--dry-run] [--force]`: writes `NAME.part0001`, `NAME.part0002`, ... and `NAME.pack.json` (default name `pack`).
  Files are stored in relative-path order (folders walked, symlinked folders and special files skipped, anything inside `--out` skipped; each choice is logged on stderr).
  `--part-size`: `K`/`KB`/`M`/`MB`/`G`/`GB` are decimal, `KiB`/`MiB`/`GiB` binary, at least 1 KiB, default 25MB (logged). `--dry-run` plans (parts, offsets) and reads nothing, but is refused (exit 3) for the names a real run refuses.
  Existing parts or manifest of that name: `err.pack.exists` (exit 6) unless `--force`. Parts are written under temporary names and renamed only when the whole pack succeeded, so a failed run leaves an older pack untouched; stale parts of the same name are removed after.
- `file check-pack NAME.pack.json [--parts DIR]`: hashes every part against the manifest, writes nothing. `--parts` defaults to the folder of the manifest. (Not `verify-pack`: a subcommand name has at most 10 characters, `name_rule_tests.rs` in the CLI crate; the document keeps the schema name `verify-pack/1`.)
- `file unpack NAME.pack.json --out DIR [--parts DIR] [--dry-run] [--force]`: writes every file that lies in intact parts, through a temporary name moved into place only when
  its SHA-256 matched; files in damaged parts are listed as lost, the others are still written. An existing target is `err.pack.exists` before anything is written, and again just before a file is written (a name the disk folds onto a file written a moment ago, such as an accent in two Unicode forms on macOS, is not replaced), unless `--force`. Pack does not fold Unicode forms: the engine has no table for it.
- `--json`: `pack-run/1`, `verify-pack/1`, `unpack-run/1` (`mzlab-cli schema NAME`); they carry no paths of the machine, so the site and the CLI give the same bytes.
  `verify-pack/1` has `missing`, `corrupt` and `resend` (part names): re-send only those, then verify again.

## Exit codes
0 ok · 1 parts missing (the other files are still written) · 2 usage · 3 corrupt part, SHA mismatch, bad manifest, unsafe path, or paths that clash: equal, equal in lower case, or a file named like a folder of another (outranks missing) · 4 input or manifest not found ·
6 output exists without `--force`, or write failure · 8 an input changed size while it was read. `check-pack` and `unpack` keep the document on stdout for exit 1 and 3.

## Format
- The stream is a **stored ZIP** written once, in order: local header, data, data descriptor per file, then the central directory. The parts are that stream cut every `part_size`
  bytes, so concatenating them in order gives a ZIP; a pack of ONE part IS a valid ZIP (`unzip -t`). Fixed DOS date: same input, same bytes. ZIP64 per entry (size or offset
  of 4 GiB or more) and for the end record (65,535 entries or more).
- The manifest (`pack/1`) lists per file: path (`/` separators, never absolute, `..`, `\`, `:` anywhere (drive, stream) or NUL: `err.pack.path`; two equal in lower case, or a file named like a folder of another: `err.pack.duplicate`), size, SHA-256, offset of its data in the stream;
  per part: name, size, SHA-256; plus `part_size`, `stream_bytes`. Unpack uses the offsets and does not parse the ZIP.
- Memory is one 1 MiB block whatever the sizes. Resumable: a part is checked alone.

## Site
`rust-pack.js` (`packFiles`, `verifyPack`, `unpackPack`) loads the lazy `.wasm` `pack` and works on OPFS sync handles. Limit: a sync handle can only be opened with `await` and the
module calls back synchronously, so every file and part is opened before the call: at most `MAX_OPEN` (1024) files and 1024 parts per call (use a larger part size, or the app).
`packFiles` takes `force` and `dryRun` as the command does: an existing pack is `err.pack.exists`, the new parts and manifest are written under temporary names and moved into place (manifest temp file first), a failed run removes its own files only, and `unpackPack` has the module read the manifest before any folder is made. `file pack` has no page yet: the route and the loader are the deliverable, a panel can call them.
