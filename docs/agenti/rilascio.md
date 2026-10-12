# Rilascio (tag `vX.Y.Z`)

- **Versione unica**: `Cargo.toml` (`[workspace.package]`), `crates/mzlab-desktop/tauri.conf.json` e `CITATION.cff` portano la stessa `X.Y.Z` (`.zenodo.json` solo se ha il campo `version`). `python3 tools/versione.py` lo controlla; `tests/test_versione.py` lo fa girare nel job pytest obbligatorio, quindi una versione scritta a metà non entra in `main`. `--tag vX.Y.Z` confronta con il tag; `--nota vX.Y.Z` stampa la sezione `## [X.Y.Z]` di `CHANGELOG.md` (errore se manca o è vuota).
- **`CHANGELOG.md`**: formato Keep a Changelog, in italiano con un riassunto in inglese; le voci nuove vanno in «Non rilasciato» e passano alla sezione della versione al momento del rilascio.
- **`.github/workflows/release.yml`** (tag `v*`; `workflow_dispatch` = prova a vuoto senza release):
  `versione` (controllo di versione e note) → `sito` (gli stessi passi di `pages.yml`, zip del sito + `tools/sbom.py`) in parallelo a
  `desktop` (richiama `desktop.yml`: `.dmg` macOS universale, `.exe` Windows, AppImage e `.deb` Linux, non firmati) e `sbom` (richiama `sbom.yml`: CycloneDX di ogni crate) → `bozza` (ambiente `release`): raccoglie i file, scrive `SHA256SUMS` e crea una **release in bozza** con le note del changelog. Nulla è pubblicato da solo.
- Se si cambiano i passi di build in `pages.yml`, cambiano anche nel job `sito` di `release.yml`.
- **`tools/sbom.py`**: CycloneDX 1.5 dei componenti JS e WASM del sito (Ketcher, OpenChemLib, Pyodide, numpy); le versioni sono lette da `mzlab/web/vendor/README.md` e `tools/build_site.py`.
- **Passi di Federico** (il codice non li fa):
  1) una volta, in Settings → Environments crea l'ambiente `release` e, se vuole, aggiungi tu come revisore obbligatorio
  2) scrivi la sezione del changelog, alza la versione nei tre file, unisci in `main`
  3) `git tag vX.Y.Z && git push origin vX.Y.Z`
  4) a workflow finito apri la bozza in Releases, controlla file e note, **Publish release** (Zenodo, se attivato come in `docs/zenodo.md`, archivia da solo e crea il DOI)
  5) Software Heritage: su <https://archive.softwareheritage.org/save/> «Save code now» con l'indirizzo del repository (un clic, serve una volta e dopo ogni release se vuoi lo SWHID aggiornato)
  6) compila `doi` e `date-released` in `CITATION.cff` come in `docs/zenodo.md`.
- Non incluso: firma di Windows (SignPath) e di macOS, dominio proprio, `update.rs` con la nuova origine.
