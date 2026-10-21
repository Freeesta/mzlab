# Studio: agenti esterni che usano `mzlab-cli`

Caso voluto da Federico: un agente che gira sul computer dell'utente (Claude Cowork, Claude Code, altri) prepara un piano mzFlow, imposta i parametri con `--set`, lo esegue e legge i risultati usando solo `mzlab-cli`, senza passare dalla pagina. L'LLM è dell'utente e gira fuori da mzLab: la pagina non c'entra, non ci sono consenso né chiavi da gestire (le ha l'agente). Contratto comune con lo studio principale (`README.md`): stesso registro (`mzlab_cli::flow::steps()`), stessi piani `mzflow/2`, stessi `prov/1` e `corsa/1`. Prompt DEV: D0 e D7 nel §6; D0 viene prima di D1.

## 1. Che cosa c'è oggi (dal codice)

- **Comandi**: `flow run PIANO [--in …] --out CARTELLA [--set ISTANZA.NOME=VALORE…] [--workers N] [--budget-mb N] [--dry-run] [--fresh]` (`crates/mzlab-cli/src/flow_run.rs`), `flow report RUNDIR [--out FILE]` (`flow_report.rs`, scrive `corsa/1` in JSON), `flow command PIANO` (lo script copiabile), `convert`, `validate --json`, `features`, `msn-graph`, `msn-lib`. Instradamento in `lib.rs::execute` (`lib.rs:342`) e `flow_cmd` (`lib.rs:332`).
- **Codici d'uscita** (già stabili e documentati in `cli.md`): 0 corsa completa, 1 istanze in errore (le altre uscite scritte), 2 uso, 3 piano non valido, 4 ingresso mancante o SHA-256 diverso. Buoni per un agente.
- **Uscite di una corsa**: `--out` contiene un file per porta (`<istanza>.<ext>`), `plan.json` (piano canonico con i `--set`, rilanciabile), `prov.json` (`prov/1`) e `corsa.json` (`corsa/1`): già leggibili da macchina.
- **Riproducibilità e cache**: stessi byte con 1 o N worker; seconda corsa nella stessa cartella riusa i lavori (`cached`); niente rete, niente file fuori da `--in`/`--out` (regola «Cluster» di `cli.md`). Già il comportamento giusto per un agente.

## 2. Che cosa manca a un agente

| Bisogno | Oggi | Manca |
|---|---|---|
| Sapere quali passi e parametri esistono | i file `docs/schemi/steps-*-1.json` del sorgente, non installati con il binario | `mzlab-cli steps --json [--id ID]`: il registro `step/1` di `flow::steps()` più `executable_native` (`flow::executable`), dal binario stesso, così la versione coincide |
| Controllare un piano scritto da sé, senza file | `--dry-run` vuole anche gli ingressi | `mzlab-cli flow check PIANO [--set …] --json`: solo `flow_compare::check` + `flow::check` + passi senza esecutore (`err.flow.step.native`) |
| Leggere i problemi | stderr `error: …` con `istanza.porta:chiave` (`problem_text`, `flow_run.rs:268`) senza `params` | `--json`: `{instance, port, error_key, params}` per problema, la stessa forma di `flow::Problem` |
| Leggere il `--dry-run` | testo per persone (`dry_text`, `flow_run.rs:335`) | `--json`: `flowdry/1` = ingressi trovati (nome, percorso, byte) e istanze in ordine con parametri canonici e `params_sha256` |
| Sapere com'è finita una corsa | riassunto su stderr (`summary`, `flow_out.rs:73`), poi leggere `prov.json` | `--json`: `flowrun/1` su stdout a fine corsa: codice, cartella, `plan_sha256`, per istanza stato (`ok|cached|error|skipped`), `ms`, uscite (porta, file, SHA-256), errore con chiave e parametri |
| Seguire una corsa lunga | righe `[3/9] …` su stderr | `--progress jsonl`: una riga JSON per lavoro su stderr (`{"job":3,"of":9,"instance","item","status","ms"}`), stesso momento delle righe di oggi |
| Errori d'uso leggibili | `error: …` + `hint: …` (`fail`, `flow_run.rs:326`) | con `--json`: `errore/1` `{code, error_key, params, hint}` su stdout, codice invariato |
| Sapere chi ha lanciato | `prov/1` dice il programma | `--by NOME` facoltativo: `launched_by{name, via:"cli"}` in `prov/1` (stessa chiave facoltativa `llm` dello studio principale, `prov_json_llm` di D3) |

Regola per tutti: `--json` non cambia codici d'uscita, file di `--out` né byte di `prov.json`/`corsa.json`; stdout porta un solo documento JSON, stderr resta per persone (o JSONL con `--progress jsonl`). Le chiavi `err.flow.*` restano le stesse.

## 3. Pagina per agenti (`docs/agenti/per-agenti.md`)

Pagina breve nel repository (scritta in questa PR con ciò che esiste oggi; D0 e D7 la aggiornano): sequenza consigliata, esempi di piani, regole di sicurezza. È il testo che un agente legge per primo; un `mzlab-cli --help` la nomina (D7). Sequenza: (1) passi e parametri; (2) scrivere il piano; (3) `--dry-run`; (4) `flow run`; (5) codice d'uscita; (6) `flow report` e i file di `--out`; (7) correggere con `--set` e rilanciare nella stessa cartella (la cache rifà solo ciò che cambia).

## 4. Server MCP

| | (a) nessun server: l'agente chiama la CLI dalla shell (scelta ora) | (b) `mzlab-cli mcp` in Rust, stdio JSON-RPC (dietro «via») | (c) involucro in Python/Node |
|---|---|---|---|
| Componenti nuovi | nessuno | un sottocomando, nessuna dipendenza (JSON di `mzlab-core`) | runtime e pacchetti nuovi |
| Agenti serviti | quelli con la shell (Claude Code, Cowork con terminale) | anche quelli che parlano solo MCP | come (b) |
| Sicurezza | quella della CLI (niente rete, solo `--in`/`--out`) | radice di lavoro fissata all'avvio (`--root CARTELLA`), nessuno strumento che cancella o scrive fuori, stessi codici | come (b), più codice da tenere |
| Regola Tutto in Rust | rispettata | rispettata | violata (logica fuori da Rust) |

Raccomandazione: (a) subito (D0 basta); (b) solo con il «via» di Federico perché è un componente nuovo da mantenere: strumenti = `steps`, `flow_check`, `flow_dry_run`, `flow_run`, `flow_report`, `read_output` (≤ 4 MB, solo dentro `--root`), con gli stessi schemi di `strumenti/1` (D1) per i parametri. (c) scartata.

## 5. Installare il binario senza compilare

| | Come | Costo | Stato |
|---|---|---|---|
| (1) Binari nei rilasci | job nuovo in `release.yml`: `mzlab-cli` per macOS universale, Windows x64, Linux x64, con SHA-256 e SBOM | nuovo job di CI e binari pubblicati: fuori perimetro senza «via» (`cli.md`) | richiesta a Federico |
| (2) Dentro l'app desktop | il binario `mzlab` dell'app già contiene `mzlab-cli` come libreria (`crates/mzlab-desktop/Cargo.toml`); `mzlab cli <argomenti>` chiama `mzlab_cli::execute` senza aprire finestre | poche righe in `crates/mzlab-desktop/src/main.rs`, nessuna CI nuova | D7, raccomandato: chi ha l'app ha la CLI |
| (3) Build WASI | `wasm32-wasip1` dello stesso sorgente, già provata (`tools/wasi_run.mjs`, `tests/test_flow_determinismo.py`) | serve `node`; più lenta; nessun accesso fuori dalle cartelle concesse | utile per agenti in sandbox; da pubblicare insieme a (1) |
| (4) `cargo install --path crates/mzlab-cli` | oggi | serve Rust | resta per gli sviluppatori |

## 6. Prompt DEV

### D0 — `mzlab-cli steps --json`, `flow check` e `--json` di `flow run` (chat DEV LLM-core o chat CLI se aperta; prima di D1)
Scrivi SOLO al Coordinatore (`send_message`, sessione indicata da Federico); mai a Federico; UN messaggio a fine PR (max 5 righe: PR, SHA, CI, pages, cosa resta) o su un blocco vero.
Studio: `docs/studi/llm/agenti-esterni.md` (§1-§3). Vincoli: solo Rust in `crates/mzlab-cli`; nessuna modifica a `Cargo.toml` o `Cargo.lock`, nessuna dipendenza, nessun crate nuovo; codice e commenti in inglese; messaggi della CLI in inglese; niente dati veri. Subagenti Haiku: `cercatore`, `verificatore`, `controllo-github`; mai a loro codice, asserzioni, commit, merge. Sopra 250k token: blocco di consegna e fermati.
Prerequisiti: nessuno. · Tocca: nuovi `crates/mzlab-cli/src/steps_cmd.rs` (comando `steps`), `crates/mzlab-cli/src/flow_json.rs` (serializzatori `flowdry/1`, `flowrun/1`, `errore/1`, problemi), `crates/mzlab-cli/src/flow_json_tests.rs`; `lib.rs` (una riga `Some("steps")` in `execute`, che resta ≤ 80 righe, una riga `Some("check")` in `flow_cmd`, righe di `HELP`, `mod`); `flow_run.rs` (solo lettura di `--json`, `--progress jsonl` e la chiamata a `flow_json`: il file resta ≤ 500 righe, oggi 477; il testo di `dry_text` e `fail` non cambia); `flow_out.rs` (`summary` e la riga di avanzamento passano dal formato scelto; oggi 133 righe); `flow_report.rs` solo se serve `--json` uniforme (`corsa/1` è già JSON: allora `--json` = stesso testo su stdout); `docs/schemi/{flowdry-1.json, flowrun-1.json, errore-1.json}`; `docs/agenti/cli.md` (righe dei comandi nuovi, nella sezione «Comandi», niente riscritture); `docs/agenti/per-agenti.md` (sequenza con `--json`). · NON tocca: `crates/mzlab-cli/src/flow.rs` (499/500), `mzlab-core`, `mzlab/web/`, `flow/`, desktop.
Leggi: `docs/agenti/cli.md` («Regole della CLI»), `docs/agenti/rust-flow.md` (Problem, prov/1, corsa/1).
PR «CLI: risposte JSON per gli agenti»:
1. `mzlab-cli steps [--json] [--id ID@V]`: senza `--json` una riga per passo (`id@version  native|page  title`); con `--json` l'array `step/1` di `step::json::registry_json(&flow::steps())` con, per passo, `"executable_native": flow::executable(id)` aggiunto in fondo (stesse chiavi e ordine degli `steps-*-1.json` per il resto); `--id` un solo passo; passo ignoto = uscita 2 con `err.flow.step.unknown`.
2. `mzlab-cli flow check PIANO [--set …] [--json]`: la funzione che oggi prepara il piano pronto (`flow_run.rs:240-264`) senza cercare gli ingressi; uscita 0 valido, 3 problemi, 2 uso; con `--json` `{"schema":"flowcheck/1","valid":bool,"problems":[{instance,port,error_key,params}]}`.
3. `--json` su `flow run` (anche con `--dry-run`): un solo documento su stdout (`flowdry/1`, `flowrun/1` o `errore/1`), stessi codici d'uscita, stderr invariato; `flowrun/1` legge `Run` e la cartella di corsa (uscite con SHA-256 come in `prov/1`). `--progress jsonl`: stesse righe di oggi in JSON, una per lavoro.
4. `params` dei problemi: gli stessi di `flow::Problem::params`, valori come in `prov/1`.
Accettazione: `cargo test -p mzlab-cli --lib flow_json` (sul piano sintetico di `flow_run_tests.rs`, `BASE`): `steps --json` ha gli stessi id di `steps-1.json`, `steps-attributio-1.json`, `steps-tabui-1.json`, `steps-cli-1.json` uniti e `executable_native` = `flow::executable`; `flow check` di un piano con un parametro fuori limite dà `err.flow.param.range` con i parametri e codice 3; `flow run --json` dà `flowrun/1` con gli SHA-256 dei file di `--out`; con e senza `--json` i file di `--out`, `prov.json` (tempi a parte) e i codici sono uguali; `--dry-run --json` non scrive nulla; `errore/1` per un `--set` malformato (codice 2) e per un ingresso mancante (codice 4); stessi byte di stdout due volte (tempi a parte). `cargo test -p mzlab-cli` intero verde, `flow_rule_tests.rs` verde. Nuovo `tests/test_cli_json.py`: i tre schemi sono JSON Schema validi e un `flowrun/1` prodotto dal test Rust (scritto in una cartella temporanea) li rispetta.
Tetti: nessun `.wasm` toccato; il binario WASI (`examples/flow.rs`, `tools/wasi_run.mjs`) compila ancora e `tests/test_flow_determinismo.py` dà gli stessi SHA-256. Conflitti caldi: `lib.rs::execute` ≤ 80 righe, `flow.rs` non toccato, `flow_run.rs` ≤ 500, `cli.md` solo righe nuove.

### D7 — Pagina per agenti, CLI dentro l'app desktop e server MCP dietro «via» (chat DEV LLM-core, dopo D0 e D3)
Intestazione, vincoli, subagenti e consegna come D0; in più: niente job di CI nuovi né binari pubblicati (richiesta (1) del §5 a Federico); il punto 3 solo con il «via» di Federico, altrimenti si salta e lo dici nel messaggio.
Prerequisiti: D0 unita; D3 unita per `--by` (usa `flow::prov_json_llm`). · Tocca: `docs/agenti/per-agenti.md` (versione completa), `crates/mzlab-cli/src/lib.rs` (riga di `HELP` che nomina la pagina), `crates/mzlab-cli/src/flow_run.rs` (`--by NOME` → `launched_by` in `prov/1` via `prov_json_llm`; resta ≤ 500 righe), `crates/mzlab-desktop/src/main.rs` (`mzlab cli <argomenti>`: prima di avviare Tauri, se il primo argomento è `cli`, chiama `mzlab_cli::execute`, scrive stdout/stderr e esce con il codice; nessuna finestra), `crates/mzlab-desktop/tests/` (prova del passaggio), `docs/agenti/rust-desktop.md` (una riga), `docs/agenti/cli.md` (righe); solo con «via»: nuovi `crates/mzlab-cli/src/mcp.rs` + `mcp_tests.rs` (`mzlab-cli mcp --root CARTELLA`), `docs/schemi/mcp-strumenti-1.json`. · NON tocca: `flow.rs`, `mzlab/web/`, `release.yml`, `Cargo.toml` (nessuna dipendenza).
Leggi: `docs/studi/llm/agenti-esterni.md` §3-§5, `docs/agenti/rust-desktop.md`, la specifica pubblica di MCP (versione stabile corrente: `initialize`, `tools/list`, `tools/call` su stdio JSON-RPC 2.0; scrivi nella pagina la versione del protocollo che implementi).
PR «mzLab per gli agenti: pagina, CLI nell'app e (con via) MCP»:
1. `per-agenti.md` completa: installazione (§5: app desktop `mzlab cli`, `cargo install`, WASI con `node tools/wasi_run.mjs`), sequenza `steps --json` → piano → `flow check --json` → `flow run --dry-run --json` → `flow run --json` → `flow report`, tabella dei codici, due piani d'esempio provati dai test (la catena BASE di `flow_run_tests.rs` e la stessa con `--set det.min_height=…`), regole di sicurezza (niente rete, solo `--in`/`--out`, nessuna cancellazione fuori dalla cache della cartella di corsa), avviso didattico (a risoluzione unitaria un m/z è un candidato).
2. `mzlab cli …` nel binario dell'app: stessi byte e codici di `mzlab-cli` (prova su `flow run --dry-run --json` e `steps --json`).
3. (Con «via») `mzlab-cli mcp --root CARTELLA`: server stdio; strumenti `steps`, `flow_check`, `flow_dry_run`, `flow_run`, `flow_report`, `read_output` (≤ 4 MB, percorsi solo dentro `--root`, `..` rifiutato) che chiamano le stesse funzioni di D0; schemi degli argomenti dal registro come `strumenti/1`; nessuno strumento che cancella o scrive fuori da `--root`; ogni corsa porta `launched_by{name:<clientInfo.name>, via:"mcp"}` in `prov/1`.
Accettazione: `cargo test -p mzlab-cli` verde con: `--by` scrive `launched_by` e senza `--by` `prov.json` ha i byte di prima; (punto 3) `mcp_tests.rs` manda `initialize`, `tools/list`, `tools/call flow_run` sul piano sintetico e confronta con `flow run --json`, un percorso fuori da `--root` è rifiutato; `cargo test -p mzlab-desktop` con la prova di `mzlab cli`; ogni comando di `per-agenti.md` è eseguito da un test (`tests/test_per_agenti.py` estrae i blocchi ```sh e li lancia sul piano sintetico con il binario costruito, saltando se manca). `python3 tools/verifica.py --cambiati` verde.
Tetti: nessun `.wasm` toccato. Conflitti caldi: `lib.rs::execute` ≤ 80 righe (una riga `Some("mcp")` se c'è il via), `flow_run.rs` ≤ 500, `main.rs` del desktop solo il ramo `cli` prima di Tauri.

## 7. Richieste a Federico (aggiunte al §10 dello studio principale)
8. Binari di `mzlab-cli` (e WASI) nei rilasci: job nuovo in `release.yml`.
9. Server MCP `mzlab-cli mcp` (componente nuovo da mantenere, nessuna dipendenza).
10. `mzlab cli` dentro l'app desktop (nessuna CI nuova: va bene come strada predefinita?).
