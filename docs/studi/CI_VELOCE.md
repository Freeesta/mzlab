# Studio: CI veloce (workflow di GitHub)

Scopo: con fino a 15 chat in parallelo, ridurre la durata dei job e la coda dei runner senza togliere nessun controllo. Le chat non possono modificare `.github/workflows/`: i file proposti, interi e pronti da incollare, sono in `docs/studi/ci_veloce/`; li applica Federico dall'editor di GitHub.

## 1. Misure

Fonte: API di GitHub (`actions/runs`, `actions/runs/{id}/jobs`, log dei job), 1500 run dal 13/10 21:50 al 15/10 07:24 UTC (33 h, prima delle chiavi di cache con `github.sha`). Gli script stanno nella cartella di lavoro della sessione, non nel repository.

- **Volume**: 20 536 minuti-runner in 33 h (in media ~10 runner sempre occupati); 2892 minuti (14 %) in job annullati da un push più nuovo. Per workflow: `test` 6529, `sito-rust` 4555, `desktop` 4523, `rust` 3032, `pages` 1112, `notte` 520, `release` 258.
- **Coda** (Linux, da creazione a inizio del job): mediana 0 la notte; nelle ore di punta (11-15 UTC) mediana fino a 6,4 min, p90 18 min, massimo 29 min. I runner gratuiti di un repository pubblico sono 20 Linux/Windows e 5 macOS insieme: ogni minuto-runner tolto accorcia la coda.
- **Tempo di una PR** (dal primo run all'ultimo job verde, stesso commit): mediana 21,4 min (p90 38) se il job `rust` gira, 14,3 min (p90 27) se è saltato. Push per PR: mediana 2. Pages, da push a sito pubblicato: mediana 6,1 min, p90 26 min.
- **Controlli obbligatori di «main protetto»** (letti da `branches/main/protection` e dal ruleset): `pytest (ubuntu-latest, 3.11)`, `pytest (ubuntu-latest, 3.14)`, `browser (ubuntu-latest, chromium)`, `javascript`, `tipi (tsc)`, `rust-ok`. `sito` non è obbligatorio. Nessun file proposto cambia questi nomi.

| Job | run | mediana | p90 | passo più lento (mediana) |
|---|---|---|---|---|
| `rust` / `rust` | 135 | 18,7 | 20,2 | `cargo test --release` 13,3 (di cui compilazione 10,5) |
| `sito-rust` / `sito-e2e` | 255 | 11,6 | 13,3 | e2e 7,9; costruzione dei .wasm 2,8 |
| `test` / `pytest` 3.11 e 3.14 | 287+288 | 8,5 | 8,9 | pytest 8,0 (in serie, 806 test) |
| `desktop` / build Windows, macOS, Linux | 71-77 | 20,1 / 16,7 / 11,9 | 25,7 / 21,7 / 16,5 | `cargo install tauri-cli --force` 9,6 / 7,8 / 5,9 |
| `sito-rust` / `sito-unit` | 287 | 4,1 | 4,5 | pytest (lo stesso di `test`) 3,8 |
| `pages` / `build` | 56 | 4,5 | 25,5 | download dei sospetti 7,4 (fino a 53); MassBank 7,6 (fino a 35) |
| `test` / `browser` | 323 | 2,3 | 2,6 | e2e di fumo 1,4 |

Misure locali (container a 4 CPU, come i runner): `cargo test --locked --release --no-run` dopo una modifica a `mzlab-core`, `mzlab-lib`, `mzlab-feat` = 582 s col profilo `release` di `Cargo.toml` (`lto = true`, `codegen-units = 1`, `opt-level = "z"`); 189 s con `lto = false`, `codegen-units = 16`, `opt-level = 3` (217 s anche ricompilando tutte le dipendenze). Con il secondo profilo: 629 test passati, 0 falliti, esecuzione 218 s.
Nel log CI il job `rust` ricompila solo le 14 crate dello spazio di lavoro (cache delle dipendenze presente), eppure impiega 10 min 28 s: il tempo è il link LTO a una unità di ~150 binari di test, non una cache mancata.

### 1.1 Le cache: il tetto di 10 GB le sfratta

Il repository ha 10 GB di cache; oltre, GitHub cancella le meno usate. L'elenco delle cache non è leggibile con il token della sessione (403); dai log:
- dimensioni: `desktop` macOS 1,7 GB, Linux 2,1 GB, Windows 1,2 GB, `desktop-shim` 0,7 GB (5,7 GB per un giro completo); `rust` 0,52; `sito-rust` 0,41; `test` browser 0,28; `pytest-wasm` 0,17; `pages-rust` 0,15 GB.
- `desktop` salva anche dalle PR (chiave su `Cargo.lock`, che cambia spesso): 76 run del desktop su PR in 33 h; un run Windows di PR non ha trovato nemmeno una cache con il prefisso `desktop-windows-latest-`.
- **Prova dello sfratto**: in `pages`, con `tools/librerie_pacchetti.py`, `crates/mzlab-lib/src`, `tools/genera_sospetti.py` identici (stessi hash git da `d0ee0f3` a `c368993`), la cache `librerie-…` è stata salvata alle 03:14 e non c'era più alle 03:55: 6 ricostruzioni di fila (3,6-35 min, quasi tutto download da Zenodo), e la cache `sospetti-…` 3 volte (7-24 min). Alle 02:57 non esisteva più nessuna `pages-rust-*`.
- Le chiavi con `github.sha` di stamattina (`pages`, `rust`, `sito-rust`) funzionano (hit sul primo run dopo il cambio), ma salvano ~1,1 GB a ogni push su main, anche quando il Rust non è cambiato: aggiunte allo sfratto del desktop, nessuna cache resta più di un'ora. L'effetto sulla durata dei run dopo la modifica non è ancora misurabile (i primi run erano in corso al momento della misura).

## 2. Colli di bottiglia e proposte (ordine di applicazione)

Guadagno in minuti-runner su 33 h di attività come quella misurata (20 536 in tutto). «Obbl.» = il file contiene un controllo obbligatorio di main (il nome resta identico).

| # | Collo di bottiglia | Misura prima | Proposta | Guadagno atteso | Rischio | File |
|---|---|---|---|---|---|---|
| 1 | Cache sfrattate dal tetto di 10 GB | `librerie` e `sospetti` ricostruite 6 e 3 volte di fila; `pages` p90 26 min | Cache del desktop salvate solo da main e una volta per versione di `Cargo.lock`; cache di `test.yml` salvate solo da main; chiave delle cache Rust sul contenuto di `crates/`, `vendor/`, `.cargo/` invece di `github.sha` (main salva solo se il Rust è cambiato) | `pages` p90 da 26 a ~6 min; niente compilazioni a freddo; meno traffico | basso: cambiano solo chiavi e condizioni di salvataggio | `desktop.yml`, `test.yml` (obbl.), `rust.yml` (obbl.), `sito-rust.yml`, `pages.yml` |
| 2 | Tauri CLI ricompilata a ogni run (`--force`) | 9,6 + 7,8 + 5,9 min per giro; wasm-bindgen ricompilato su Windows (2,9 min: manca `wasm-bindgen.exe` nel percorso della cache) | `tauri-cli` fissata (`TAURI_CLI: 2.12.3`) e presa dalla cache se c'è; `wasm-bindgen.exe` in cache; l'installer con `-- --profile desktop` | ~26 min-runner per giro, ~1950 in 33 h (−9 %); desktop da 20 a ~9 min | basso; una versione nuova della CLI va scritta a mano in `TAURI_CLI` | `desktop.yml` |
| 3 | Test Rust compilati con LTO a una unità | `cargo test` 13,3 min, compilazione 582 s | Solo nel passo dei test: `CARGO_PROFILE_RELEASE_LTO=false`, `…CODEGEN_UNITS=16`, `…OPT_LEVEL=3`; i .wasm restano sul profilo `release` | −6,5 min a run, ~880 in 33 h; PR con Rust da 21 a ~15 min | basso: stessi test, stesso codice; i .wasm e le guardie di memoria non cambiano | `rust.yml` (obbl.: `rust-ok`) |
| 4 | pytest ripetuto in `sito-unit` | 4,1 min × 287 run | `sito-unit` tolto: i controlli rapidi di `verifica.py` (sintassi JS con Teoria e shell, i18n, mappa, soglie; 14 s) passano in testa a `sito-e2e`; pytest resta in `test.yml` (3.11 e 3.14, con il toolchain wasm32, quindi anche i test che `sito-unit` saltava) | ~1100 in 33 h (−5,5 %) e un job in meno in coda per run | basso: nessun controllo perso; `sito` riassume solo `sito-e2e` | `sito-rust.yml` |
| 5 | pytest in serie nei 2 job obbligatori | 8,0 min ciascuno | `pytest -n auto --dist loadfile` (pytest-xdist, già usato da `sito-unit`) e cache aggiornata da main | vedi §3 | medio-basso: i test che costruiscono i .wasm girano in processi diversi (cargo li serializza col suo lock) | `test.yml` (obbl.) |
| 6 | Download ripetuti in `pages` e `release` | MassBank 240 MB ≈ 11 min; fonti dei sospetti 330 MB ≈ 24 min | Cache dei download con chiave sullo SHA-256 fissato negli script; lo zip è verificato prima dell'uso; `genera_sospetti.py --fonte` scarica solo ciò che manca | una ricostruzione da 35 a ~2 min | basso: stessi file, stessi SHA-256 | `pages.yml`, `release.yml` |

Ordine: 1-4 insieme (desktop, sito-rust, rust, pages), poi `test.yml` (5) da solo, perché contiene 4 controlli obbligatori; `release.yml` quando si vuole (gira solo ai rilasci).

## 3. Valutati e non proposti (o rimandati)

- **Un solo .wasm per `rust`, `sito-rust`, `pages`** (artefatto condiviso): i tre workflow girano in run diversi; passare l'artefatto da `sito-rust` a `pages` richiede `workflow_run` e lega la pubblicazione al successo degli e2e. La costruzione costa 2,8 min con cache calda: guadagno piccolo, rischio sulla consegna. Non proposto.
- **sccache** (backend GitHub Actions): scrive migliaia di voci nella stessa quota di 10 GB che oggi è il problema. **mold/lld**: il tempo è nel link LTO, che il linker non accelera. Non proposti.
- **cargo-nextest**: l'esecuzione dei test Rust è 246 s in serie su ~150 binari; in parallelo scenderebbe verso ~90 s (il binario più lungo, `rois_parity`, 73 s). Ma più binari scriverebbero insieme gli stessi file sintetici (`tools/golden.py`) e i doctest vanno lanciati a parte: prima va controllato test per test. Rimandato a un pacchetto con prova.
- **e2e di `sito-e2e` in serie**: i test con `"sito"` in `NEEDS` (`tools/verifica.py`) usano porte fisse e girano uno alla volta; i tre più lenti (`e2e_tpmine_lr_rust` 227 s, `e2e_tpmine_ingresso` 205 s, `e2e_tpmine_transf` 194 s) dominano. È codice dei test, non dei workflow: pacchetto a parte.
- **wasm-opt in serie** (`tools/wasm_opt.sh`, ~90 s su 13 file): parallelizzarlo è una modifica di `tools/`, ~1 min a run.
- **Filtri sui percorsi per `test.yml`**: no, i 4 job sono obbligatori e anche una PR di soli documenti è controllata (nomi vietati, dimensioni dei file d'area).
- **Annullare i run di main superati** (`cancel-in-progress` anche su main): su main ogni commit tiene il suo risultato per scelta; GitHub già scarta i run in attesa superati. Lasciato com'è.
- **Desktop solo su main per le PR che cambiano solo `Cargo.lock`**: toglierebbe ~40-60 run di PR, ma sposterebbe un controllo dopo l'unione. Decisione di Federico; non nei file.
- **Runner più grandi** (8 core, a pagamento al minuto, prezzi su docs.github.com «billing for GitHub Actions»): utili solo a `rust` e ai due `pytest`; con le proposte 3 e 5 il guadagno residuo è piccolo. Opzione, non proposta.
- `advisories`, `sbom`, `notte`: fuori dalle PR, nessun collo; `notte` beneficia da sola delle cache più fresche.

## 4. Istruzioni per incollare (una riga per file)

Per ognuno: GitHub → `.github/workflows/<nome>` → matita → seleziona tutto → incolla il file di `docs/studi/ci_veloce/` → «Commit changes» su `main`.

1. `desktop.yml`: cache solo da main con chiave per sistema/`Cargo.lock`/versione della CLI, Tauri CLI dalla cache, `wasm-bindgen.exe` in cache, installer con `-- --profile desktop` e bundle in `target/[triple/]desktop/`.
2. `sito-rust.yml`: `sito-unit` tolto, controlli rapidi in testa a `sito-e2e`, cache con chiave sul contenuto del Rust.
3. `rust.yml` (contiene `rust-ok`, obbligatorio): passo dei test senza LTO, cache con chiave sul contenuto del Rust.
4. `pages.yml`: cache Rust sul contenuto, cache dei download MassBank e delle fonti dei sospetti.
5. `test.yml` (4 controlli obbligatori, nomi invariati): pytest con `-n auto --dist loadfile`, cache salvate solo da main.
6. `release.yml`: le stesse cache dei download di `pages.yml`.

Dopo ciascuno: il primo run su main salva le cache nuove (può durare quanto oggi), dal secondo valgono i tempi della tabella. Se un run diventa rosso, si torna al file precedente dalla cronologia del file su GitHub.
