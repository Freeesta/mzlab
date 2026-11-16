# Security study of mzLab (report only)

Scope: what an attacker gains by handing the user a crafted file (data file, folder, Flow plan) or by controlling the published site content. Nothing is fixed here: every item has a severity, the place in the code, how it was checked and a one-sentence fix. CONFIRMED means reproduced by running the code (command given) or read directly in a configuration file; SUSPECTED means traced by reading only, or bounded by a mitigation.

## Coverage

| Area | State | What was done |
|---|---|---|
| Rust readers | partial | `unsafe` inventory (whole workspace); the mutation test of the readers at 20 times its rounds; a wider mutation harness on Agilent `.D`, ANDI `.cdf` (every byte offset) and part of Bruker PASEF |
| Desktop app | reviewed by reading | the 24 Tauri commands, capabilities, `mzlab://` serving, updater, downloads, assistant; two items run (C1, C2) |
| Browser front end | partial | CSP of `index.html` and `tauri.conf.json`, `message` handlers, a sample of the HTML sinks, the caching rules of `sw.js` |
| Supply chain | partial | the 17 workflows (pins, triggers, permissions, expressions, installs), `deny.toml`, Dependabot; no advisory database run |

Not done: the wider harness on `.wiff`, Thermo, Waters, Shimadzu, Bruker TDF/TSF, mzML, mzXML, gzip, the libraries (MSP, MGF, MassBank, JSON), drawing files and `.mzidx`; a full triage of the 375 HTML sinks of `mzlab/web/*.js`; OPFS names built from file names; the registration and update flow of the service worker; `cargo audit` / `cargo deny check advisories` (no tool in the study machine; the daily job `advisories.yml` is the standing check); versions of the vendored JavaScript (`mzlab/web/vendor/`).

## CONFIRMED

### C1 - high - the engine route `feat/attributio/review` writes a file anywhere the user can write
- Where: `crates/mzlab-desktop/src/feat_attributio.rs:37` and `:111-125` (`write_in`), dispatched at `crates/mzlab-desktop/src/feat.rs:420`.
- What: `folder` is any absolute path taken from the request; only the relative `path` is checked for plain components. `path` is the `feature_decisions` value of the `campaign.toml` the request itself carries, and the text is a `;` table whose free-text cells (`justification`, `decided_by`) keep their line breaks. A script running in the window of the app therefore chooses folder, file name and whole lines of the content: a line of its own in a shell start-up file, or a new file in a start-up folder, is code execution as the user.
- Reach: needs script in the window (an HTML injection that passes the CSP, or a hostile web part delivered by the update, see S3). No page code sends `folder` today.
- Reproduction (engine level, no window): a test in `crates/mzlab-desktop/tests/` built like `feat_attributio.rs`, with `feature_decisions = ".profile-demo"` in the `campaign.toml` of the request, `"justification": "x\necho INJECTED-LINE\n#"` in the decision and `folder` set to an empty temporary folder; `State::new().rpc("feat/attributio/review", ...)` answers `written=true` and the folder holds `.profile-demo` with `echo INJECTED-LINE` on a line of its own.
- Fix: drop `folder` from the route and write only under a folder chosen through a native dialog and kept on the native side.

### C2 - medium - the `id` of a Flow step is used as a file name: a plan writes outside the run folder
- Where: `crates/mzlab-core/src/flow/parse.rs:52` (no character rule on `id`, none in `check.rs`); sinks `r.work.join(format!("{}.json", job.instance.id))` at `crates/mzlab-cli/src/flow.rs:269`, `:309`, `crates/mzlab-cli/src/flow_base.rs:55-56`, `:476-480` and the other steps (`.csv`, `.mgf`, `.md`, `.mzfeat`, `.html`, `.svg`).
- What: `Path::join` with an absolute path replaces the base, so an `id` that is an absolute path makes the step write there (the relative `../` form works when the folders on the way exist). The extension is the one of the step; part of the content comes from the inputs of the plan (names of library entries). `flow check` says the plan is valid. The same engine runs the plans of the Flow page in the app (`crates/mzlab-desktop/src/flow_run.rs`).
- Reproduction: a folder with `lib.msp` (one record) and `q.mgf` (one spectrum) and a `plan.json` `{"format":"mzflow/2","inputs":[{"name":"q.mgf","type":"spectra"},{"name":"lib.msp","type":"library"}],"steps":[{"id":"/ABSOLUTE/victim/notes","step":"spectra.match@1","in":{"query":"file:q.mgf","library":"file:lib.msp"}}]}`; `mzlab-cli flow run plan.json --out run` ends with `done` and `/ABSOLUTE/victim/notes.json` is replaced by the answer of the step.
- Fix: accept only `[A-Za-z0-9_.-]` (no leading dot) for an instance id in `parse.rs`, with an error key.

### C3 - medium - an Agilent `.D` folder stops the program: `AcquiredTime` cut inside a character
- Where: `crates/mzlab-core/src/agilent_ext/mod.rs:265` and the twin `crates/mzlab-core/src/agilent/mod.rs:225`: `t.split_at(19)` on the text of `Contents/AcquiredTime`.
- What: when byte 19 of the value is inside a multi-byte character the slice panics. The profiles `release`, `desktop` and `cli` have `panic = "abort"` (`Cargo.toml`): the app and the CLI end at once, the `.wasm` module traps. Reached from the header of the converted mzML (`mzml_header`), so on opening or converting the folder.
- Reproduction: `python3 tools/agilent_sintetico.py OUT synt.D 12`, then in `OUT/synt.D/AcqData/Contents.xml` set `<AcquiredTime>2025-05-15T16:00:4é</AcquiredTime>` (UTF-8); `mzlab-cli convert OUT/synt.D --out X` prints `end byte index 19 is not a char boundary` (debug build: the job fails; release build: abort).
- Fix: take the head with `t.get(..19)` (or `char_indices`) and return `None` when it is not a boundary.

### C4 - low - Python packages without a version on the path of the installers
- Where: `.github/workflows/desktop.yml:49-50` (`pip install numpy`, `pip install playwright`), called by `release.yml:294`; the same pattern in the test workflows.
- What: a compromised release of one of those packages runs in the job that builds the installers. The job has `contents: read` only and the draft is made by a separate job behind the `release` environment, which limits the damage to the built files.
- Fix: install from a file with pinned versions and hashes (`pip install --require-hashes -r ...`) in the jobs whose output is published.

### C5 - low - `cargo-cyclonedx` installed without a version
- Where: `.github/workflows/sbom.yml:20` (`cargo install cargo-cyclonedx --locked`), called by `release.yml:299`; every other `cargo install` of the workflows names a version.
- Fix: add `--version X.Y.Z`.

### C6 - low - the installers are not signed
- Where: `.github/workflows/desktop.yml:2` states it; `release.yml` publishes `SHA256SUMS` in the same release as the files.
- What: the sums prove integrity against a damaged download, not origin: whoever can replace a file of the release can replace the sums.
- Fix: sign the installers (or at least `SHA256SUMS`, with a key kept in the `release` environment) and say in the release notes how to verify.

## SUSPECTED

| # | Severity | Where | What | Fix |
|---|---|---|---|---|
| S1 | high | `crates/mzlab-desktop/src/main.rs:117-120` (`register_paths`), read back through `mzlab://localhost/file?id&off&len` (`protocol.rs:136`, `engine.rs:257`) | the command registers any path given by the page as a picked file: script in the window reads any file of the user (keys, the stored assistant keys). Read in the code, not run (needs the window build). Sending the bytes out is limited by the CSP to navigation and to the assistant request | register only the paths the native drop event delivered, never strings from the page |
| S2 | medium | `crates/mzlab-desktop/src/startup.rs:40-64`, `main.rs:483` | the window has no navigation rule and every `mzlab://` answer carries `Access-Control-Allow-Origin: *` with no check of the request origin: a foreign page reached by navigation could call the engine protocol, C1 included | allow only the app origin in `on_navigation` and answer with the app origin, not `*` |
| S3 | medium | `crates/mzlab-desktop/src/update.rs:28-30`, `:328-366` | the updater of the web part trusts TLS only: the manifest and its SHA-256 list come from the same host as the payload, with no signature; a compromised Pages deploy or repo write ships a web part that reaches C1 and S1. `update_apply` (`main.rs:246`) is also a plain command any window script can call | sign the manifest with a key embedded in the binary and verify it before apply |
| S4 | low | `crates/mzlab-desktop/src/store.rs:273-278`, `idxbuild.rs:201-211` | these refuse `/ \ NUL . ..` in a stored name but not `:`; on Windows `lr/upload?name=C:x` (`rpc.rs:46-66`) becomes a drive-relative path or an NTFS alternate stream. `libraries.rs:335-348` already refuses `:` | refuse `:` in the name, as `libraries.rs` does |
| S5 | low | `crates/mzlab-core/src/agilent_ext/datams.rs:166`, `:265-266` | `MSScan.bin` / `MSPeak.bin` declare the scan and peak counts; a crafted header made the Agilent reader spike to a single ~63 MB allocation in the mutation harness. It stays under the site budget (192 MB) and the desktop budget (1024 MB), so it is a spike, not an out-of-memory, but the count drives the size before any point is read | check the declared counts against `Limits` (as the other readers do) before reserving |
| S6 | low | `crates/mzlab-desktop/src/llm.rs:84-101`, `download.rs:117-133` | the assistant request is built by the page and sent to the provider with the stored key (a confused deputy if a window script forges one); with the `ollama` provider a window script can POST to `http://localhost:11434` paths of its choosing. The key is never read back to the page (`has_key` only) | fix the request path per provider and keep the body the page may set to the message fields |

## Clean areas (checked, nothing found)

- `unsafe`: only two sites in the workspace outside tests. `crates/mzlab-desktop/src/mapped.rs:21` (read-only mmap; truncation under the map is a crash at worst, documented) and `crates/mzlab-lettori-wasm/src/reading.rs:435` (memory views read before any other call, documented). No `unsafe` in `vendor/opentfraw`.
- The mutation test of the readers (`crates/mzlab-lettori/tests/reader_mutations.rs`) at `MZLAB_MUTAZIONI=20` (mzML numpress, mzXML, gzip single and multi-member, ANDI, Shimadzu `.qgd`/`.lcd`, Thermo TSQ, Waters, Bruker TDF): no panic, no hang, no non-finite number passed on.
- The wider harness over every byte offset of the ANDI `.cdf` seeds and part of Bruker PASEF (`.tdf_bin`): no panic, no hang, no allocation over budget. The declared-count limits of `crates/mzlab-core/src/readers.rs:48-98` (`Limits::sito/desktop`, `ok_entries`, `profondita_ok`) are applied by the readers before allocating or recursing.
- Recursion bounds: `crates/mzlab-core/src/json.rs:102` (depth 64), `crates/mzlab-core/src/xmltree.rs:181` (depth 256).
- CSP of the site (`mzlab/web/index.html`) and the app (`crates/mzlab-desktop/tauri.conf.json`): `default-src 'self'`, `object-src 'none'`, `base-uri 'none'`, `form-action 'self'`, no `unsafe-inline`/`unsafe-eval` for scripts (hashes plus `wasm-unsafe-eval` and `blob:`); `connect-src 'self'` on the site. An injected inline handler or inline `<script>` is blocked. Two gaps: `style-src 'unsafe-inline'` (styles only), and `blob:` in `script-src` so a page-controlled `blob:` URL can run as script in the app window — the channel that would turn an HTML injection into C1/S1.
- Service worker (`mzlab/web/sw.js`): every cached file is SHA-256 checked against a signed-in-the-page manifest before it is served (`:103`, `:124`, `:241`, `:280`); only status-200 responses are cached (`:162`, `:302`); `skipWaiting` fires only on the page's own `sw-attiva` message (`:216`). The `message` listener acts on that one op alone.
- `BroadcastChannel("mzlab")` (`mzlab/web/archivio.js:21-24`) and the window `message` listeners (`draw.js:26`, `first-paint.js:47`) check `e.source`/`e.data.type`; the channel is same-origin, so another origin cannot post to it.
- Desktop: no process spawn, no OS-shell open, no opener, no native-library load on the app path; no `eval`/`execute_script`; `emit("mzlab-open", ())` carries no payload. Argv, single-instance and file-association opening (`open.rs`, `opening.rs`) filter options and take the extension from the readers registry. `folder::resolve` and `libraries::Folder::resolve` keep plain components only. `update_serve.rs` normalises paths and refuses symlinks and native files with a canonical `starts_with`.
- Downloads (`download.rs`, `download_sources.rs`) use an exact host allow-list, https only, re-check each redirect and verify a per-piece and whole SHA-256. The assistant key (`llm.rs`) is stored `0600` by temp+rename and never returned to the page (`has_key` only).
- Supply chain: every third-party `uses:` in `.github/workflows/` is pinned to a full commit SHA; the only tag references are the repo's own reusable workflows (`release.yml` → `desktop.yml`, `sbom.yml`), which is safe. Every workflow has a top-level `permissions:`; write grants are scoped to the job that needs them (`codeql` security-events, `pages` id-token, `metriche`/`stat-ci`/`release` contents). No `${{ github.event.* }}` or `head_ref` is interpolated into a `run:` shell; event values reach steps through `env:`.
- `metriche.yml` (a `workflow_run`) checks `head_repository.full_name == github.repository` before running. `deny.toml` denies unknown registries and git sources; the five ignored advisories are compile-time or desktop-only with a stated review date.

## How to regenerate the malformed inputs (none committed)

- C3: `python3 tools/agilent_sintetico.py OUT synt.D 12`, then edit `OUT/synt.D/AcqData/Contents.xml` so `AcquiredTime` has a multi-byte character at byte 19 (e.g. `2025-05-15T16:00:4é`); `mzlab-cli convert OUT/synt.D --out X`.
- C2: a folder with a one-record `lib.msp`, a one-spectrum `q.mgf` and the `plan.json` of C2 with an absolute-path `id`; `mzlab-cli flow run plan.json --out run`.
- C1, S1, S2, S3: an engine-level test in `crates/mzlab-desktop/tests/` built like `feat_attributio.rs` reproduces C1 without a window; S1/S2/S3 need the window build (`--features app`) and were read in the code, not run.
- Readers: the mutation harness is `MZLAB_MUTAZIONI=N cargo test -p mzlab-lettori --features tutte --test reader_mutations`; the seeds come from the test builders in `crates/mzlab-lettori/tests/common*` and from `tools/*_sintetico.py` and `tools/wiff_riferimento.py`.