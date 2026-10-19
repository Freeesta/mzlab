# Pubblicazione delle librerie (solo analisi)

Parte di `docs/studi/WORKFLOW_MODULARI.md` (§12). Nessuna pubblicazione: ogni azione verso l'esterno (crates.io, npm, nomi riservati, DOI) richiede il «via» di Federico. Segni: **V** verificato nel repository, **I** ipotesi.

## 1. Candidati e ordine
| Ordine | Crate | Perché | Cosa manca |
|---|---|---|---|
| 1 | codec numpress, staccato da `mzlab-core` (`src/numpress.rs`, 321 righe, V) | piccolo, nessuna dipendenza, specifica esterna (MS-Numpress, Teleman 2014) e file di riferimento pubblici della specifica: verità esterna pronta | crate a sé, nomi in inglese, prova contro i vettori della specifica |
| 2 | `mzlab-lib` (formato `.mzlib`, parser delle librerie, punteggi; nessuna dipendenza interna, V) | utile da solo, API già raccolta in un crate | API stabile, documentazione, prova contro MassBank/MoNA pubbliche |
| 3 | `mzlab-lettori` + parte di lettura di `mzlab-core` (trait `Lettore`, `apri`, scrittura mzML) | il pezzo più raro (lettura nativa senza librerie del produttore) | dipende dalla parte grande di `mzlab-core`: va prima separata; mzML da validare con il validatore PSI (prova 2 del §7) |
| 4 | npm: `mzlab-lettori-wasm` | lo stesso del 3 nel browser | solo dopo il 3; pacchetto con `.wasm` + glue, tetti di byte invariati |

**Non candidati**: `mzlab-feat` (≈ 68 mila righe, contratti ancora in movimento), `mzlab-desktop`, `mzlab-cli`, i `*-wasm` di pagina (legati all'interfaccia). `opentfraw` (in `vendor/`, Apache-2.0 con le nostre modifiche in `vendor/opentfraw/MODIFICHE-MZLAB.md`, V): non si ripubblica, si propongono le modifiche al progetto originale.

## 2. Licenza e dipendenze
- `LICENSE`: MIT, © 2026 Federico Cristaudo (V); i crate la ereditano con `license.workspace = true` (V in `mzlab-lib`).
- Le licenze delle dipendenze sono già controllate a ogni PR dal job `deny` di `.github/workflows/rust.yml` con `deny.toml` (V): ammesse MIT, Apache-2.0, BSD-2/3, MPL-2.0, ISC, Unicode, Zlib, OFL-1.1, CDLA-Permissive-2.0 (`webpki-roots`, solo dati dei certificati), sorgenti solo da crates.io. Tutte compatibili con MIT per la distribuzione; MPL-2.0 chiede di pubblicare le modifiche ai SUOI file, non al nostro codice.
- Doppia licenza: dove una dipendenza è «MIT OR Apache-2.0» (es. `r-efi`) si sceglie MIT.
- `opentfraw` (Apache-2.0): chi lo ridistribuisce deve portare licenza e NOTICE delle modifiche; oggi vale per il sito e l'app (I: controllo del pacchetto dell'app non fatto in questo studio).
- patRoon è GPL-3: nessun codice copiato (§9 del principale), quindi nessun vincolo GPL sui crate.
- Nessun crate ha oggi `publish = false` (0 su 18, V): un `cargo publish` per errore partirebbe. Proposta: aggiungerlo a tutti finché Federico non dà il via, e toglierlo solo dal crate scelto.

## 3. Cosa serve a una libreria pubblica
- **API stabile con semver**: oggi i nomi pubblici sono in italiano (`apri`, `scrivi`, `Limiti`, `Lettore`): serve una facciata in inglese, poi `1.0` solo quando non cambia più; prima `0.x`.
- **Documentazione**: rustdoc su ogni elemento pubblico (`#![deny(missing_docs)]`), un README con un esempio che si compila (doctest), MSRV dichiarata.
- **Test con verità esterna**: vettori di riferimento della specifica (numpress), file pubblici con valori noti (serie `verita/1` del banco di ROSINA.md F), mzML validato dal validatore PSI; niente test che chiedono il repository privato.
- **Citazione**: `CITATION.cff` esiste già nella radice (V); per ogni crate un DOI (Zenodo) solo con il via di Federico.
- **Niente dati privati**: il pacchetto si controlla con `cargo package --list` (nessun file di `mzlab-dati`, nessun cognome di `nomi_vietati.txt`: lo stesso controllo di `tests/test_esempi_anonimi.py`).
- **Nome libero** su crates.io e npm: non verificato (I).
- **Manutenzione**: CHANGELOG per crate, `cargo semver-checks` in CI prima di ogni versione (I: non installato).
