# CI veloce: cosa è condizionale

- **Concurrency**: `test`, `rust`, `sito-rust`, `desktop` annullano il run in corso solo sulle pull request (un push nuovo sostituisce il vecchio); su `main` ogni commit tiene il suo risultato.
- **Pages** (`pages.yml`): `build` e `deploy` hanno gruppi propri (`pages-build`, `pages-deploy`) senza annullamento: un run in corso finisce sempre, tra quelli in attesa GitHub tiene solo il più recente. Il sito riceve quindi sempre una versione nuova anche con molte unioni di fila.
- **`rust`**: job `changes` + `rust-ok` sempre presente; `rust` e `deny` girano solo se cambiano `crates/`, `Cargo.*`, `rust-toolchain.toml`, `deny.toml` o gli script del controllo Rust (altrimenti «skipped», `rust-ok` verde).
- **`sito-rust`** (`sito`, `sito-e2e`, `sito-unit`): parte solo se cambiano `crates/`, `Cargo.*`, `mzlab/`, `tools/build_site.py`, `tests_e2e/`, `tests/golden/`, `tools/dati_sintetici.py`, `tools/verifica.py` o il workflow; su una PR solo-documentazione NON compare (filtro dei percorsi, non «skipped»).
- **`desktop`** (job `shim` + 3 OS): solo se cambiano `crates/mzlab-desktop/**` (include `tauri.conf.json` e `prepare.py`), `Cargo.lock`, `mzlab/web/**` e `tools/build_site.py` (il bundle del frontend che finisce nell'installer) o il workflow, oppure a mano.
- **`test`** (`pytest 3.11/3.14`, `browser`, `javascript`, `tipi`): girano sempre (controlli obbligatori di «main protetto»).
- Cache: cargo (`registry`, `git`, `target`), `wasm-bindgen`, browser Playwright e pip, con chiave su `Cargo.lock` + `rust-toolchain.toml` (Playwright: `pyproject.toml`) e `restore-keys` sul prefisso, così un `Cargo.lock` nuovo parte dalla cache più recente invece che da zero. Le cache salvate da `main` sono visibili alle PR.
