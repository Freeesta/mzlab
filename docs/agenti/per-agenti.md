# mzLab per agenti esterni (`mzlab-cli` dalla shell)

Per un agente che gira sul computer dell'utente (Claude Code, Claude Cowork o altri) e usa mzLab senza la pagina: prepara un piano `mzflow/2`, lo controlla, lo esegue e legge i risultati con `mzlab-cli`. Regole complete dei comandi: `cli.md`; piani e `prov/1`: `rust-flow.md`; passi `step/1`: `rust-step.md`. Studio: `docs/studi/llm/agenti-esterni.md`.

## Installazione
- Oggi solo da sorgente: `cargo install --path crates/mzlab-cli` (serve Rust), poi `mzlab-cli` è nel PATH. La stessa CLI compila per `wasm32-wasip1` ed esegue un piano sotto node con `tools/wasi_run.mjs`.

## Sequenza consigliata
1. **Passi e parametri**: `mzlab-cli steps --json [--id ID]` stampa i descrittori `step/1` del binario che userai, con `executable_native`; gli stessi stanno in `step/1` stanno in `docs/schemi/steps-1.json`, `steps-attributio-1.json`, `steps-tabui-1.json`, `steps-cli-1.json` (una riga JSON per passo; `params` è un JSON Schema con `default`, `minimum`, `maximum`, `enum`). Un passo senza esecutore nativo (elenco `BUCHI` in `cli.md`, «Copertura») fa rifiutare il piano con `err.flow.step.native`.
2. **Piano**: un file `mzflow/2` con `inputs` (nomi dei file) e `steps` (`id` dell'istanza, `step` = `id@versione`, `in` = `file:NOME` o `istanza.porta`, facoltativi `algorithm`, `preset`, `params` con le sole differenze dal predefinito).
3. **Controllo senza scrivere**: `mzlab-cli flow check PIANO.json [--set ...] --json` controlla il piano senza gli ingressi (`flowcheck/1`, codice 3 con i problemi); `mzlab-cli flow run PIANO.json --in CARTELLA --dry-run` controlla piano, `--set` e ingressi e stampa gli ingressi trovati e le istanze in ordine con i parametri canonici; codice 3 = piano non valido (una riga `istanza.porta:chiave` per problema su stderr), 4 = ingresso mancante; con `--json` su stdout `flowdry/1`.
4. **Corsa**: `mzlab-cli flow run PIANO.json --in CARTELLA --out CORSA [--set ISTANZA.NOME=VALORE ...] [--workers N]`; avanzamento su stderr, una riga per lavoro (`--progress jsonl`: una riga JSON per lavoro).
5. **Esito**: con `--json` su stdout `flowrun/1` (stato, tempi e SHA-256 dei file di ogni istanza) o `errore/1` (`error_key`, `params`, `hint`); codice d'uscita 0 tutto fatto, 1 qualche istanza in errore (le altre uscite sono scritte), 2 uso sbagliato, 3 piano non valido, 4 ingresso mancante o SHA-256 diverso.
6. **Risultati**: in `CORSA` un file per porta (`<istanza>.<ext>`), `plan.json` (piano canonico con i `--set`), `prov.json` (`prov/1`) e `corsa.json` (`corsa/1`: stato, tempi e parametri di ogni istanza); `mzlab-cli flow report CORSA` riscrive il `corsa/1`.
7. **Correggere**: rilanciare nella stessa `CORSA` con altri `--set`: la cache rifà solo i passi che cambiano (`cached` in `prov/1`); `--fresh` rifà tutto.

## Esempio (la catena di base sui file mzML)
```json
{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML","file:s_t030.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}]}
```
È il piano `BASE` di `crates/mzlab-cli/src/flow_run_tests.rs`. Un parametro si cambia senza toccare il file, per esempio `--set det.polarity=-1` o `--set det.min_height=5000`; `--set det.algorithm=centwave` sceglie il modello a onde (parametri `ppm`, `peak_width_s`, `snr` solo con quell'algoritmo); un algoritmo `planned` del descrittore (oggi `threshold` di `features.detect`) dà `err.flow.algorithm.unavailable`; il nome di un'istanza unica può essere l'id del passo.

Tabella per famiglie e tabella per ioni: due `features.align` sullo stesso `det.peaks`, il secondo con `"params":{"mode":"ion"}` (o `--set ion.mode=ion`); il detect è condiviso. In modo famiglie un ione assorbito nella famiglia del genitore (ISF, addotto) non ha riga e compare solo fra i membri; in modo ioni ha la sua riga con le aree per campione.

## Regole per l'agente
- La CLI non usa la rete e legge e scrive solo in `--in` e `--out`; non cancellare file dell'utente: la cartella di corsa è l'unico posto di scrittura.
- A risoluzione unitaria un m/z è un candidato, non un'identificazione; i passi di identificazione richiedono dati `hr`.
- Ogni corsa è riproducibile: stesso piano e stessi ingressi danno gli stessi byte con 1 o N `--workers`.
