# Export all my data (inventory of what the app saves in the browser + ZIP writer)

All of the user's work lives in THEIR browser; the export gathers it into a single `mzlab-dati-<date>.zip` downloaded locally (nothing leaves the computer). Storage names: `docs/agenti/dati.md` ("Key inventory"), `mzlab/web/nomi.js`.

## ZIP writer (Rust)
- `crates/mzlab-core/src/zip.rs`: `zip_stored(&[(name, bytes)])` writes a "stored" ZIP (no compression, no dependency, no ZIP64: at most 65535 entries and 4 GiB), UTF-8 names (bit 11), fixed DOS date 1980-01-01 (deterministic archive), CRC-32 IEEE. `zip_packed(packed)` builds it from a single buffer: per entry `u16` name length, `u32` data length (little endian), name, data.
- In the engine (`crates/mzlab-wasm/src/zip.rs`): `zip(packed) -> Vec<u8>` (a single read; the worker passes the returned buffer, a new copy of the exact length, without copying it again).
- Reading: `unzip_stored(zip)` rereads a "stored" ZIP (CRC checked, unsafe names and entries sharing bytes rejected: empty, leading `/`, `\`, NUL, `..`, two entries on the same bytes (`zip: entries overlap`: a "stored" archive keeps each byte once, so expansion cannot exceed the file); it also applies to archives from other writers, but it reads neither compressed entries nor ZIP64); `unzip_packed` returns it in the buffer of `unpack`. In the engine: `unzip(zip) -> Vec<u8>`. Tests in the Rust tests `zip/tests.rs` (it also rereads `py.zip` written by Python).
- Tests: `cargo test -p mzlab-core zip::` (known CRC, empty archive, file fingerprint) and `tests/test_zip_stored.py` (the Rust test with `MZLAB_ZIP_OUT=<folder>` writes 0 files, an empty file, 4 files with 70 000 bytes and a non-ASCII name; Python rereads them with `zipfile` byte by byte and with `unzip -t`).

## Button and collection
- **Button**: none. The "Data and privacy" section of Settings has been removed from the interface: `esporta-dati.js`, `importa-dati.js` and the worker remain as code with no entry point (no other file loads them) and no e2e test; the `export.*` and `import.*` keys left in `lang/*/settings.js` are those the modules use.
- `mzlab/web/esporta-dati.js`: `collect()` (localStorage, IndexedDB `mzlab_lab` in the stores `notebook`, `kv`, `liste_utente`, `molecole` as JSON with binaries in base64, OPFS `librerie/`), `manifest()`, `pack()`, `run()`, `start()`. `esporta-dati-worker.js` calls `zip`/`unzip` of the engine. The file is `mzlab-dati-<YYYY-MM-DD>.zip` with `manifesto.json` (format `mzlab-dati/1`, version, date, `files` with bytes and meaning, `excluded` with reason), `localStorage.json`, `indexeddb/<store>.json`, `opfs/librerie/…`.

## Import my data
- `importa-dati.js` (`begin(d, file)`) reads a ZIP of this format. The worker `esporta-dati-worker.js` (op `unzip`) has Rust read the ZIP; `plan()` accepts only `manifesto.json` with format `mzlab-dati/1` and keeps a localStorage key only if it belongs to mzLab and is not secret, run-time or development (same rules as the export: a tampered file cannot write anything else).
- Preview `#uip-import-box` (`role=group`): number of entries and how many differ, records per IndexedDB store, library files; focus goes to "Restore" (`#uip-import-go`), "Cancel" writes nothing. Only on confirmation `apply()` writes localStorage, IndexedDB (through `lab()`) and the libraries in OPFS (op `opfs`; files in use are counted in `import.partial`), then the page reloads. An invalid file gives `import.bad` without a preview. Local secrets are not touched.

## Collection rules
- **Included**: everything the user creates: preferences and views (localStorage `mzlab.*`, `mzlab-theme`, `mzlab-labels`; the old `qqq.*` only if the matching `mzlab.*` is absent), notebook, user lists, molecules, spectral libraries, session. mzML/raw files and the indexes (`idx/`, `tiles/`) are NOT included unless already saved by the app as user data: they can be rebuilt from the original files.
- **Excluded, listed in the manifest with the reason**: secrets (every key whose name contains `token`, `key` or `secret`; the assistant key is never saved, it lives only in the `llm/trasporto.html` iframe); run state (`mzlab.run`, `mzlab.avvii`, `mzlab.migrato`, `mzlab.memProbe`, `mzlab.memBudget*`, `mzlab.memLevel`, `mzlab.kprefetch`, sessionStorage); app caches (Cache API `mzlab-app-*`, `mzlab-big-*`, service worker); development and admin flags (`mzlab.admin`, `mzlab.dev.*`).

## Inventory
| Where | Key / name | Meaning | Format |
|---|---|---|---|
| localStorage | `mzlab.prefs` | general preferences | JSON object |
| localStorage | `mzlab.lang`, `mzlab-theme`, `mzlab-labels` | language, theme, chart labels | string |
| localStorage | `mzlab.a11y` | reading aids and accessibility settings | JSON |
| localStorage | `mzlab.sessione` | open files and panels (session) | JSON |
| localStorage | `mzlab.picchi`, `mzlab.calc`, `mzlab.mappa`, `mzlab.ordine`, `mzlab.pca`, `mzlab.pfas`, `mzlab.profilo`, `mzlab.fair`, `mzlab.flgrp`, `mzlab.tunit`, `mzlab.kmd`, `mzlab.qaqc`, `mzlab.riferimento`, `mzlab.bianco`, `mzlab.comp`, `mzlab.infg.frag` | choices of the Data views | JSON or string |
| localStorage | `mzlab.barra.lato`, `mzlab.barra.lr`, `mzlab.barra.hr`, `mzlab.barra.hint`, `mzlab.banco.info` | sidebar layout and notices already seen | string / JSON |
| localStorage | `mzlab.lib.soglie`, `mzlab.lib.cartella`, `mzlab.liste.attive`, `mzlab.contaminanti`, `.esclusi`, `.on` | chosen libraries, lists and contaminants | JSON |
| localStorage | `mzlab.disegno.font`, `.riquadri`, `.strumenti`, `mzlab.pal.recent` | Draw page: font, panels, tools, recent colours | JSON |
| localStorage | `mzlab.tour.lr`, `mzlab.upd.last`, `mzlab.upd.seen`, `mzlab.rel.last`, `mzlab.rel.seen`, `mzlab.reset.note`, `mzlab.logo.gold` | tour, update notices already seen | string / JSON |
| localStorage | `mzlab.llm`, `mzlab.llm.consenso` | assistant: settings and consents per provider (no keys) | JSON |
| localStorage | `mzlab.attributio.reviewer`, `mzlab.tabui.*`, `mzlab.pipeline.draft` | admin: reviewer and decisions (not exported from the students' interface) | JSON |
| IndexedDB `mzlab_lab` | `notebook`, `kv` | notebook; key-value pairs | objects |
| IndexedDB `mzlab_lab` | `liste_utente`, `molecole`, `manifest` | user lists, molecule store, list of files in OPFS | objects |
| IndexedDB `mzlab_lab` | `files` | files saved by the old engine (fallback only) | blob |
| IndexedDB `mzlab-libs`, `mzlab-libdir` | `lib`, `h` | spectral libraries and folder pointed to without OPFS (fallback) | blob / handle |
| OPFS | `librerie/` | saved spectral libraries (`<id>.mzlib`) | binary |
| OPFS | `mzlab/` (engine root), `idx/`, `tiles/`, `conv/`, `export/` | imported files, indexes, maps and conversions: rebuildable | binary |
| Cache API | `mzlab-app-*`, `mzlab-big-*`, `mzlab-libs` | offline copy of the app | HTTP responses |
| sessionStorage | `mzlab.run`, `mzlab.telefono*`, `mzlab.unopen`, `mzlab.convopen`, `mzlab.tour.step` | state of this run | string |

Note: keys and folders are read BY NAME from the browser (like `reset.js`, no fixed list): the table explains the families, the collection takes everything that starts with `mzlab`/`qqq` minus the exclusions.

## Chunked exports (`export-pezzi.js`, `export-worker.js`)
- All text exports go through `dl(name, text, type, prov)` (`explore.js`); `text` is a string, a `Blob`/`Uint8Array` or an iterable (possibly async) of chunks (strings, `Uint8Array`, `Blob`).
- A string or `Blob` under 8 MB and an iterable that ends under 8 MB are saved with `saveBlob` as a whole `Blob`; beyond that, `EXPORT_PEZZI.save` (`export-pezzi.js`, ES module loaded with `carica()` at the first large or iterable export, outside the start-up list) writes the chunks with `export-worker.js` into OPFS `mzlab/export/<id>` (`createSyncAccessHandle`, chunks of about 1 MB, one acknowledgement message per chunk) and downloads the `File` read from there. Same bytes: strings are UTF-8 and a surrogate pair is never split between two messages.
- Without OPFS or `Worker` the chunks are collected in memory and a `Blob` is saved (`save` returns `false`). Write error (space, storage): notice `export.stream.fail`, no file.
- Provenance (`provenienza/1`): CSV/TSV carry the `# mzLab …` line first; on an iterable the line is prepended to the first chunk, identical; `prov === false` does not write it.
- Cleanup: the file in `export/` is removed 10 minutes after the download starts (`drop`); at the first load of the module `clean` removes files older than 1 hour. The download URL stays valid for the same 10 minutes.
- Whoever produces a lot of text passes a generator: MGF/MSP of all MS2 (`piecesMgf`, `piecesMsp` in `workflow.js`, same bytes as `formatMgf`/`formatMsp`).
- Test: `tests_e2e/e2e_esporta_pezzi.py` (byte by byte on small sources, 9 MB with a cut surrogate pair, 210 MB in chunks with SHA-256).
- mzFinder (`TP_Mine/js/`): the exports go through the same `dl()` (`window.dl`): `G.dl` in `28_`, `30_`, `38_`, `42_` and the `dl` helper of `19_tpmine_results.js` (also used by `21_tpmine_hr.js`, with `prov === false`: no provenance line). A large string (full CSV, MGF, SIRIUS) beyond 8 MB goes to OPFS; the report ZIP (`zipStore`) and the engine strings stay in memory until the engine hands them over in chunks.
