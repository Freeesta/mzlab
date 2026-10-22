# Processo come macchina: dove perdiamo tempo e come smettere

Studio del flusso PR → CI → unione → sito. Fonti: `stat-ci` (217 eventi, 4,5 h di lavoro, 14 PR unite), log dei job su `main`, ultimi 52 merge di `main`, workflow in `.github/workflows/`. Stime al giorno su ~50 PR unite e ~1,6 run di `test` per PR.

## (a) Dati misurati, in ordine di minuti persi

| # | Collo di bottiglia | Misura | Minuti persi stimati al giorno |
|---|---|---|---|
| 1 | `pytest (3.11)` è il percorso critico di ogni run | Unit tests 19 min 06 s su `main` (gli altri job ≤ 3 min, 3.14 70 s). Causa misurata in (e): le build cargo dei test si cancellano a vicenda nello stesso `target/` (i 4 file più lenti durano 7-25 s da soli, 535-968 s insieme) | ~13 min per run × ~80 run delle PR = **~17 h di attesa delle PR**, più ~13 min per ogni unione in fila del Coordinatore |
| 2 | Push di rilavorazione | 2,2 push per PR, 1,6 run di `test` per PR; 160 min di runner annullati in 4,5 h. Cause lette nei commit: «accorcia cli.md sotto il limite», «funzioni più corte», fmt/clippy (`rust-lint` 2), soglie (`javascript` 2), tipi 1 | 0,6 run in più × 20 min × 50 = **~10 h** |
| 3 | Conflitti veri sulle liste | 3 in 4,5 h, sempre gli stessi file: `crates/mzlab-cli/src/lib.rs` (17 dei 52 merge), `docs/agenti/cli.md` (16, a 25 563/25 600 B), `docs/schemi/strumenti-1.json` (13), `flow-copertura-tabui.md` (10), `flow.rs` (10, 477/500 righe), `flow-pacchetto-tabui.md` (9), `AGENTS.md` (8, 25 570/25 600 B), `flow_rule_tests.rs` (7), `steps-cli-1.json` (4). Le PR del Flow restano aperte 5-6 h, le altre ~1 h | ~8 conflitti × 15 min + 0,4 merge di main per PR × 5 min × 50 = **~4 h**, più un giro di CI ciascuno |
| 4 | Lavoro manuale del Coordinatore | Per PR: CI sullo SHA di testa, combine locale, `controlla.py`, controllo del diff (Cargo, workflow, `.wasm`, CSP, desktop), squash con `expectedHeadSha`, notifica, archivio | ~5-8 min e ~15-30k token per PR = **~5 h e ~1M token** |
| 5 | PR di soli documenti con la CI intera | 5 dei 52 merge toccano solo `docs/`, `*.md`, `.claude/`: 20 min di `test` ciascuna | **~1,5 h** |
| 6 | Coda di `pages` | 12 attese, 17 min in tutto; `build` 8 min | trascurabile (la coda tiene solo l'ultimo run) |
| 7 | Misura falsata | 11 «conflitti» su 14 non hanno file in comune e tornano risolti entro 2 min: stato `dirty` passeggero di GitHub dopo un push su `main`. Gonfiano la classifica di `ci_metrics.py` di 2 h 45 | 0, ma nasconde le cause vere |

Quello che NON è un problema: la coda dei runner (0 s), `rust` (job più lungo `rust-wasm` 6 min, in parallelo a `test`), il verde → unione (immediato).

## (b) Ipotesi per ciascun collo di bottiglia

1. **pytest 3.11.** La causa e la correzione (una target-dir per famiglia di feature) sono in (e); tagli e shard non servono più.
2. **Rilavorazione.** Hook `pre-push` che lancia `pre_pr.py --veloce` (30 s) e rifiuta il push se è rosso; avviso di `controlla.py` dal 90 % del tetto sui file caldi; i file caldi divisi prima che sforino (sotto). Regola: un push per PR dopo il verde locale; i commit intermedi restano locali.
3. **Conflitti sulle liste.** Le liste in cui ogni PR aggiunge una riga diventano file in cui l'unione è automatica: (i) i `mod flow_*;` di `lib.rs` in `flow_mods.rs` (incluso con `include!`), con i test dentro il modulo (`#[cfg(test)] #[path = "flow_x_tests.rs"] mod tests;`) così ogni passo aggiunge una riga sola; (ii) `.gitattributes` con `merge=union` (driver interno di git, nessuna configurazione) solo per file che sono liste pure di righe indipendenti; (iii) la tabella dei passi di `cli.md` in un file d'area a parte, una riga per passo in ordine alfabetico; (iv) `tools/fondi_main.py`: `git merge origin/main`, i file generati in conflitto si prendono da main e si rigenerano con il loro strumento, poi `pre_pr.py --veloce`. GitHub non applica `merge=union` lato server, ma il merge locale del DEV e il combine del Coordinatore sì: il «dirty» sparisce prima del push.
4. **Coordinatore.** `tools/coo_unisci.py <PR> <sha>`: legge da `stat-ci` (`eventi.jsonl`) l'esito dei run su quello SHA (niente API, niente token), fa il combine in un worktree, lancia `controlla.py` e `indice.py --controlla`, segnala i percorsi a rischio (`Cargo.*`, `.github/workflows/`, `*.wasm`, meta CSP di `index.html`, `crates/mzlab-desktop/`) e stampa una riga di verdetto con i parametri della chiamata di squash. Notifica e archivio restano al Coordinatore, ma da un modello di 3 righe; lo stato delle chat lo legge `stato-sessioni`.
5. **Soli documenti.** Come per la Teoria (`tools/ci_solo_teoria.py`): se ogni file cambiato è un documento, `pytest` esegue solo i test dei documenti (mappa, soglie, nomi vietati, i18n dei documenti), `browser` salta i passi con esito verde. I job restano presenti per il ruleset.
6. **Misura.** `ci_metrics.py registra-conflitti` scrive un conflitto solo se i file in comune sono > 0 oppure se il `dirty` resta alla seconda lettura dopo 60 s; il riepilogo aggiunge «push per PR per causa».
7. **Idee valutate e lasciate.** Merge queue: GitHub la offre ai repository di organizzazione, non di un account personale (da verificare; altrimenti serve spostare il repository). Branch «sempre aggiornata»: un giro di CI in più a ogni merge, il contrario di quello che serve. Stack di PR dipendenti: utile solo per il Flow, superfluo dopo il punto 3.

## (c) Piano a ondate

| Ondata | Intervento | Prompt | Via | Guadagno al giorno |
|---|---|---|---|---|
| 1 (subito, in parallelo) | pytest 3.11 a ≤ 6 min | P8 (sostituisce P1) | già dato (`test.yml`) | ~17 h di attesa delle PR, ~2 h di fila del Coordinatore |
| 1 | Misura senza falsi conflitti | P6 | già dato (`stat-ci.yml`) | misura giusta per le ondate dopo |
| 1 | Percorso dei soli documenti | P2 | già dato (`test.yml`) | ~1,5 h |
| 2 (a Flow fermo: nessuna PR del Flow aperta) | Liste senza conflitti | P3 | no | ~4 h + ~8 run di CI |
| 2 | `fondi_main.py` | P4 | no | ~1 h e i file generati sempre giusti |
| 2 | `coo_unisci.py` | P5 | no | ~4 h e ~0,8M token del Coordinatore |
| 3 | Hook pre-push + modello di PR | P7 | no | ~6 h (metà della rilavorazione) |
| 3 | Unione automatica dopo `coo_unisci.py` | R1 | **sì** | il Coordinatore non aspetta il verde |

## (d) Prompt per le chat DEV (Sonnet)

Regole comuni a tutti: ramo `claude/<argomento>` da `origin/main`, nessun dato reale, nessun nome di composto, nessun percorso personale, nessun segreto; prima del push `git merge origin/main` e `python3 tools/pre_pr.py`; una PR aperta e un messaggio di ≤5 righe al Coordinatore (COO 11).

**P1 · sostituito da P8** (la causa misurata in (e) non è la durata dei test).

**P2 · CI: PR di soli documenti** (via già dato per `test.yml`). Estendi `tools/ci_solo_teoria.py` (o un `tools/ci_solo_doc.py` gemello) con il caso «ogni file cambiato è sotto `docs/` o è un `*.md` o è sotto `.claude/`»: `pytest` esegue solo `test_esempi_anonimi.py`, i test della mappa e delle soglie e i controlli i18n dei documenti; `browser` salta con esito verde. Un file qualunque fuori lista = tutto. File: `tools/ci_solo_*.py`, `.github/workflows/test.yml`, test in `tests/`, una riga in `ci-veloce.md`. Accettazione: test pytest con elenchi di file (solo doc → vero; doc + un `.py` → falso; `AGENTS.md` → vero); PR di prova docs-only con `test` verde in ≤ 3 min.

**P3 · Flow: liste senza conflitti** (no via; lancialo quando nessuna PR del Flow è aperta). (1) Sposta i `mod flow_*;` e i loro `#[cfg(test)] mod flow_*_tests;` da `crates/mzlab-cli/src/lib.rs` a `crates/mzlab-cli/src/flow_mods.rs`, incluso con `include!`, e porta ogni `flow_x_tests` dentro `flow_x.rs` con `#[cfg(test)] #[path = "flow_x_tests.rs"] mod tests;` (una riga per passo). (2) `.gitattributes`: `merge=union` solo per `flow_mods.rs` e per i file d'area che sono pure tabelle di righe indipendenti (decidi e scrivi quali). (3) La tabella dei passi di `docs/agenti/cli.md` va in `docs/agenti/cli-passi.md`, una riga per passo in ordine alfabetico; `cli.md` ha un rimando. Aggiorna la regola (b) di AGENTS.md §2 senza farlo crescere in byte. Accettazione: script di prova in `tests/` che crea un repository temporaneo, due rami che aggiungono due passi adiacenti, `git merge` senza conflitti; `cargo test -p mzlab-cli` verde; `flow_rule_tests.rs` verde; `controlla.py` verde.

**P4 · Strumento: `tools/fondi_main.py`** (no via). Fa `git fetch origin main` e `git merge origin/main`; se restano in conflitto solo file generati (lista in testa al file: `docs/schemi/strumenti-1.json`, `docs/schemi/steps-cli-1.json`, `tools/tipi/globali.d.ts`, altri che trovi con `grep` dei generatori), li prende da main, li rigenera con il loro strumento, li aggiunge e conclude il merge; per ogni altro conflitto si ferma e stampa i file. Poi lancia `pre_pr.py --veloce`. Uscita 0 / 1 (conflitti umani) / 2 (controlli rossi). File: `tools/fondi_main.py`, `tests/test_fondi_main.py` (repository temporaneo), una riga in `docs/agenti/test.md` e in AGENTS.md §2 al posto di «Prima: `git merge origin/main`» (stessi byte o meno). Accettazione: test con un conflitto su un JSON generato risolto da solo e un conflitto su un `.rs` che ferma.

**P5 · Strumento: `tools/coo_unisci.py`** (no via; sola lettura salvo il worktree temporaneo). Ingresso: numero di PR e SHA di testa. (1) `git fetch origin pull/<N>/head stat-ci main`; lo SHA deve coincidere. (2) Da `origin/stat-ci:eventi.jsonl` gli eventi `run` su quello SHA: `test` e `rust` `success` (o `rust` saltato da `changes`); altrimenti «CI non verde: <job>». (3) Worktree temporaneo con il merge su `origin/main`, `tools/controlla.py`, `tools/indice.py --controlla`, poi il worktree si toglie. (4) Percorsi a rischio nel diff: `Cargo.toml`, `Cargo.lock`, `.github/workflows/`, `*.wasm`, meta CSP di `mzlab/web/index.html`, `crates/mzlab-desktop/`: «VIA richiesto: …». (5) Ultima riga JSON: `{pr, sha, verde, combine, rischi, unire}`. File: `tools/coo_unisci.py`, `tests/test_coo_unisci.py` (eventi finti e repository temporaneo), una riga in `docs/agenti/ci-veloce.md`. Accettazione: casi verde, rosso, SHA diverso, conflitto, rischio Cargo.

**P6 · stat-ci: niente falsi conflitti** (via già dato per `stat-ci.yml`). In `tools/ci_metrics.py registra-conflitti` un PR `dirty` diventa evento `conflitto` solo se `n_file_comuni > 0` oppure se è ancora `dirty` a una seconda lettura dopo 60 s; in `riepilogo` aggiungi «push per PR» divisi per causa (titoli dei commit dopo il primo: limite, fmt/clippy, merge di main, altro). File: `tools/ci_metrics.py`, `.github/workflows/stat-ci.yml` se serve l'attesa, test in `tests/`. Accettazione: con gli eventi di prova i falsi spariscono e i 3 veri restano.

**P7 · Pre-push e modello di PR** (no via). (1) `tools/hook_pre_push.sh`: lancia `pre_pr.py --veloce` e rifiuta il push se esce 1; `tools/installa_hook.py` lo mette in `.git/hooks/`, e uno hook di avvio della sessione cloud lo installa (skill `session-start-hook`). (2) `controlla.py`: avviso dal 90 % del tetto per i file toccati da più di 5 degli ultimi 50 merge. (3) `.github/pull_request_template.md` con 4 caselle: area, `pre_pr.py` verde, file generati rigenerati, rischi (Cargo, workflow, CSP). File: quelli citati, `docs/agenti/test.md` (una riga). Accettazione: push di prova con `cli.md` oltre il tetto rifiutato in ≤ 60 s; test pytest dell'avviso al 90 %.

## Richieste del via a Federico (3 righe ciascuna)

**R1 · Unione automatica.** Attivare «Allow auto-merge» nelle impostazioni del repository. Il Coordinatore lancia `coo_unisci.py` e, se è pulito, attiva l'unione automatica squash invece di aspettare il verde. Cambia la regola «mai unita dalla chat» solo per il Coordinatore, sempre dopo il suo controllo.

**R2 · Merge queue.** Disponibile solo se il repository passa a un'organizzazione GitHub. Darebbe l'ordine automatico e il test sul main combinato senza conflitti di gara. Da valutare solo se R1 e P3 non bastano.

## (e) Studio profondo del tempo di CI (dati grezzi)

Fonti: API Actions, con 700 run di `test.yml` e 700 di `rust.yml` (20-21/10: 179 PR unite con run); log dei job con `--durations`; tre run locali di tutto `tests/` su `origin/main`, con 4 CPU come il runner e `-n auto --dist loadfile`.

**Misure**
- `pytest 3.11`, Unit tests: mediana 400-560 s il 20/10 e 925-1040 s dal 21/10 alle 00 UTC (arrivo di `test_msngrafo_wasm`).
- Push → verde di `test.yml` su PR: mediana 982 s, p90 1553 s. `rust.yml` sulle PR Rust: mediana 955 s, p90 1877 s. `sito-rust`: 490 s. `pages` (push → sito): 980 s.
- Run di `test` annullati: 33 % (233 su 700), pari a 2261 min di runner in 2 giorni. Le PR hanno in media 2,16 SHA e 0,07 run falliti: si rilancia per i push, quasi mai per un rosso.
- Coda dei job: mediana 30 s, p90 circa 10 min nelle ore piene. Una push di una PR Rust accende circa 20 job: test 5, rust 8, sito-rust 5, stat-ci 1-3. È circa il limite di job concorrenti del piano gratuito. stat-ci è il 46 % dei run del repository.
- Cache cargo di pytest: è salita da 272 MB a 1,3 GB e poi scesa a 1,1 GB. La salva il job 3.14, che dalla #1365 salta i test di `tools/pytest_solo_311.txt`. Non è la causa: un run con la cache piena è durato comunque 886 s.
- **Causa misurata**: lanciati da soli a target caldo, `test_msngrafo_wasm` dura 7 s, `test_valida_lr` 16 s, `test_valida_hr` 25 s e `test_cancello` 14 s. Insieme con `-n 4` durano 794, 968, 615 e 535 s: tutto pytest 18 min 33 s in locale, quanto in CI. Senza `wasm-bindgen` (i test wasm saltati) il run da zero dura 8 min 42 s.
  `mzlab-feat-wasm` è `cdylib`+`rlib`: la sua libreria ha un nome fisso e ogni famiglia di feature la sovrascrive. Le famiglie sono `-p mzlab-cli` (che lo usa senza default, con detect, proofs, finder e untargeted), `-p mzlab-feat-wasm` (default) e `--features untargeted`. Ogni alternanza ricompila: cli 65 s → nativo_mrm 12 s → cli 66 s, a target caldo, in fila dietro il lock di cargo.
  Rischio in più: un test può eseguire un binario compilato da un altro worker con feature diverse.
  Si sovrascrivono solo i prodotti finali con nome fisso (binari e `cdylib`): le `rlib` di ogni insieme di feature restano accanto, con il loro hash. Prova a target caldo: `cargo test --no-run -p mzlab-feat` 70 s, `--features record` 47 s, `record,interp` 51 s, di nuovo senza feature 0 s.
- `rust.yml` non ha questo difetto: ogni job chiama cargo in fila, mai due processi insieme, e i suoi cambi di feature toccano solo `rlib` o la foglia `mzlab-feat-wasm` (wasm32: detect 48 s, ancora detect 0 s, proofs 3 s, di nuovo detect 3 s). Ogni insieme di feature costa una compilazione sola, e resta nella cache.
- Doppioni: `test_guardie_memoria.py`, `test_feat_wasm.py` e `test_flow_determinismo.py` girano sia nel 3.11 sia in `rust-wasm` (`rust.yml`, righe 137-146).
- Controlli obbligatori del ruleset: `javascript`, `pytest (ubuntu-latest, 3.11)`, `pytest (ubuntu-latest, 3.14)`, `browser (ubuntu-latest, chromium)`, `rust-ok`, `tipi (tsc)`. «Strict» è spento: la PR non deve essere aggiornata a main.

**Simulazione sulle 179 PR** (tempi del log del 21/10, calibrati sul run misurato; script nel blocco di consegna).
- Selezione per percorso dei 19 file lenti (crate cambiati e loro dipendenti): 3.11 da 18,6 a 14,0 min di mediana.
- Più 2 shard: 13,1 min. Più i 3 file di convalida solo di notte: 13,1 min.
- Nessuna variante scende sotto 8 min finché le build si cancellano a vicenda: il limite è il singolo file che aspetta il lock.
- Con la correzione delle target-dir il tempo atteso è la somma dei tempi da soli diviso 4 worker, cioè 3-4 min anche senza selezione.

**Misura dopo P8 e la chiave della cache con l'hash dello shim** (job `pytest 3.11`, passo Unit tests / job intero).
- Prima: 925-1040 s / 19-20 min.
- Cache calda: PR di misura 186 s / 4 min 32 s (run intero 5 min 36 s); push di `main` che salva la cache 293 s / 7 min 05 s.
- Cartelle `target/fam/*` fredde (run partiti prima che `main` salvasse la cache): 416-592 s / 8,5-10,5 min.
- Il salvataggio della cache parte solo se la chiave manca: ogni modifica di ciò che entra nella cache cambia anche la chiave, altrimenti le PR restano fredde.
- Ora il più lento è `rust.yml` sulle PR Rust: 10 min 50 s dal push al verde (`rust-wasm` 9 min 12 s, `rust-misc` 8 min 13 s, `rust-core` 7 min 25 s). Sotto 8 min si scende solo con R3.
- In `rust.yml`, `rust-core` sarebbe superfluo in 92 delle 158 PR Rust: i crate cambiati non sono fra i dipendenti di core, lettori, lib, libro, mol e target.

| Intervento | Minuti risparmiati per run di PR | Rischio | Prova |
|---|---|---|---|
| P8 · target-dir per famiglia di feature | ~13 (19 → ≤ 6) | basso: cambiano i percorsi dei binari, non i test | pytest `-n 4` a target caldo < 5 min in locale; «passed» uguale; 3.11 in CI ≤ 6 min su 3 PR reali |
| P9 · niente doppioni fra 3.11 e `rust-wasm` | ~2 (400 s-worker) | basso: gli stessi test girano in `rust-wasm` quando cambia Rust | elenco dei test eseguiti dal 3.11 più `rust-wasm` uguale a prima (`--collect-only`) |
| P2 · PR di soli documenti | ~17 sulle PR di soli documenti (13 su 179) | basso | test della selezione; PR di prova verde in ≤ 3 min |
| Selezione per percorso dei file lenti | ~4 (dopo P8 quasi 0) | medio: un test saltato può nascondere un errore fino alla notte | solo se P8 non basta |
| R3 · `rust.yml` in tre interventi | ~4 sulle PR Rust (da 10,8 a ~6,5) | medio | simulazione sulle 158 PR Rust ed errore messo apposta in `mzlab-core` |
| stat-ci ogni 10 min invece che a ogni `workflow_run` | coda: −40 % dei run | basso | stessi eventi registrati |

**Prova esterna chiesta a ogni prompt**:
- (a) la simulazione sopra rifatta con i tempi nuovi;
- (b) unione delle selezioni o degli shard uguale a `pytest --collect-only -q tests`, con un test pytest;
- (c) tempi veri via API su 3 PR reali dopo l'unione: `pytest 3.11`, passo Unit tests, prima (mediana 925-1040 s) e dopo, con `tools/ci_metrics.py riepilogo`;
- (d) un errore messo apposta in un ramo di prova: una costante in `mzlab-feat` e una riga in `flow_*.rs`. Deve essere preso dalla CI veloce o da `notte.yml`, e il ramo non si unisce.

**P8 · CI: una target-dir per famiglia di feature** (via già dato per `test.yml`; niente Cargo).
1. Misura prima, a target caldo: tutto pytest `-n 4`, poi i quattro file della tabella da soli.
2. Nello shim `tools/ci_cargo/cargo` (per `build` e `run`), ogni invocazione riceve `--target-dir target/fam-<nome>` secondo la sua famiglia: `cli` (`-p mzlab-cli`), `feat` (`-p mzlab-feat-wasm` di default), `untargeted`, e `wasm32-<pacchetto>-<feature>`. Le altre restano su `target/`.
3. I percorsi scritti a mano (`tools/valida/*.py`, `tests/*.py`: `target/release/examples/…`, `target/wasm32-unknown-unknown/release/…`) passano per una funzione `tools/cargo_uscita.py` che chiede a cargo il percorso dell'eseguibile (`--message-format=json`). Vale anche in locale e in `verifica.py`.
4. La cache di pytest salva tutte le `target/fam-*` dal job 3.11.

File: `tools/ci_cargo/cargo`, `tools/cargo_uscita.py`, `tools/valida/*.py`, i test che costruiscono, `.github/workflows/test.yml`, una riga in `docs/agenti/ci-veloce.md`.
Accettazione: le prove (a)-(d); in CI il 3.11 ≤ 6 min su PR con cache calda; nessun test saltato in più.

**P9 · CI: niente doppioni con `rust-wasm`** (via già dato per `test.yml`). Il 3.11 deseleziona `test_guardie_memoria.py`, `test_feat_wasm.py` e `test_flow_determinismo.py` quando la PR cambia un percorso Rust (la stessa regola di `changes` in `rust.yml`, letta da uno script comune `tools/ci_percorsi_rust.py`); altrimenti li esegue. Accettazione: test della regola con elenchi di file; prova (b).

**R3 · `rust.yml` in tre interventi** (via di Federico chiesto). (a) Il determinismo esce da `rust-wasm` in un job suo: da 9,2 a circa 5,5 min. (b) `rust-core` gira solo se cambiano `crates/mzlab-{core,lettori,lib,libro,mol,target}/` (crate senza altre dipendenze del progetto), `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` o `tests/golden/`: saltato in 92 PR Rust su 158. (c) `rust-misc` in 2 job per crate: da 8,2 a circa 4,5 min. Una PR Rust passa da 10,8 a circa 6,5 min; `rust-ok` resta il solo nome richiesto e aspetta anche i job nuovi.
Rischio: +2 job per push, un po' più di coda nelle ore piene; ogni notte `notte.yml` esegue tutto. Prova: simulazione delle regole sulle 158 PR Rust passate, un errore messo apposta in `mzlab-core` preso dalla PR, tempi veri via API su 3 PR.
