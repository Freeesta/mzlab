# Studio: assistente dati con l'API di Claude

Obiettivo (Federico, 22/10): una chat che apre i file caricati, capisce che cosa ha davanti (strumento, polarità, risoluzione, cromatografia, gradiente, tipo di acquisizione, QC, bianchi), fa domande all'utente (matrice, composti attesi, obiettivo), propone il workflow di mzFlow e i suoi parametri fondamentali con motivo e confidenza, legge la feature table e disegna grafici.

Questo studio non contiene codice di prodotto e non ha fatto chiamate all'API. Costruisce sopra ciò che esiste (`docs/agenti/llm.md`: `mzlab-llm`, agente, politica, `contesto/1`, trasporti; `docs/agenti/tutor.md`: tetti in micro-dollari, valutazioni, controllo dei numeri) e dice che cosa manca.

## Documenti

| File | Contenuto |
|---|---|
| `01-architettura.md` | dove gira il client, chiave, cosa parte e cosa no, strumenti = passi, piano validato in Rust, `grafico/1`, subagenti |
| `02-costo.md` | prezzi verificati, token per fase, caching, tetto $10 con arresto morbido, piano del test da $200 |
| `03-parametri.md` | segnali dei file → parametri di mzFlow: regole deterministiche in Rust, spiegazione del modello, confidenza |
| `04-feature-table.md` | controlli sulla feature table (QC, bianchi, duplicati, addotti, isotopi, deriva RT), domande, formato dell'uscita |
| `05-sicurezza.md` | iniezione dai nomi e dai metadati, limiti, registri (livello b spento), funzionamento senza rete |
| `06-valutazione.md` | compiti d'oro sui file sintetici, metriche, soglie di accettazione |
| `07-mappa-codice.md` | cosa riusare, cosa inventare, tetti `.wasm`, piano di PR |
| `08-prompt-dev.md` | un prompt per chat DEV, in ordine |
| `prompt/` | i prompt per l'API (sistema, router, subagenti, valutazione) e `00_esecuzione-test.md` |

## Cosa farebbe un professionista (e non era chiesto)

1. **Il modello non misura, interpreta.** Ogni numero (larghezza dei picchi, rumore, errore di massa, deriva RT, CV dei QC) lo calcola Rust in modo deterministico; il modello riceve i numeri con la prova e li spiega o fa domande. Così l'errore numerico del modello è zero per costruzione e il test è ripetibile.
2. **Compiti d'oro prima dei prompt.** Il set con risposta nota (file sintetici di `tools/dati_sintetici.py`, `06-valutazione.md`) si scrive prima di spendere un dollaro; ogni prompt ha un criterio di accettazione misurabile.
3. **Costo per compito completato**, non per richiesta: una sessione economica che richiede tre giri in più non è economica (`02-costo.md`).
4. **Prefisso stabile e cache**: testo di sistema e catalogo `strumenti/1` congelati (byte stabili, già garantiti dal test `llm_tools`), dati variabili in coda; si verifica `cache_read_input_tokens` > 0 al secondo turno, altrimenti il test si ferma.
5. **Uscite strutturate ovunque**: i subagenti rispondono con `output_config.format` (schema JSON), gli strumenti hanno `strict: true` (`additionalProperties: false`); Sonnet 5.5 rifiuta `tool_choice` forzato, quindi `auto` + istruzione + controllo che la chiamata ci sia.
6. **Validazione in Rust di tutto ciò che il modello produce**: piano (`flow::check`), parametri (limiti dello `step/1`), grafico (`chart::validate`), testo (ogni numero citato deve stare nei dati inviati, come `mzlab-tutor::checks`).
7. **Tetto di spesa a tre livelli**: contatore nel client (arresto morbido all'80 %, duro a $10), limite di spesa del workspace dedicato nella Console ($200), stima prima dell'invio sopra 50 000 token.
8. **Difesa dall'iniezione** dai metadati: nomi e campi liberi passano come stringhe JSON delimitate, tagliate e con alias; la politica guarda solo strumento e argomenti (`policy::decide`, già così); set di attacchi rossi nel test.
9. **Riproducibilità e audit**: ogni turno ha SHA-256 della richiesta, modello servito, `usage`, costo e versione dei prompt in `llmlog/1` (livello a: niente testo); ogni corsa lanciata dall'assistente porta il blocco `llm` in `prov/1` (esiste).
10. **Degradazione senza rete**: l'ispezione, le proposte dei parametri, i controlli della tabella e i grafici predefiniti sono Rust e funzionano senza modello; il modello aggiunge solo spiegazione e dialogo.
11. **Gestione delle risposte particolari**: `stop_reason` `refusal` (Sonnet 5.5 ha classificatori), `max_tokens`, 429 con e senza `retry-after`, 400 di limite di spesa (`You have reached your specified workspace API usage limits`): ognuno ha la sua chiave `err.llm.*` e nessun ritentativo dove non serve.
12. **Prompt versionati come dati**: ogni prompt è un file con SHA-256 registrato nel log; cambiare un prompt = nuova versione e nuova corsa del set d'oro.

## Materiale di partenza

- Documentazione ufficiale letta il 22/10/2026: prezzi (`platform.claude.com/docs/en/about-claude/pricing`), limiti di spesa e di frequenza (`/docs/en/api/rate-limits`), caching, uso degli strumenti, uscite strutturate, effort (guida `claude-api`). I numeri sono in `02-costo.md`.
- Codice: `crates/mzlab-llm` (fornitori, adattatori, `Replay`, `llmlog/1`, agente, politica, `contesto/1`, valutazioni), `crates/mzlab-tutor` (`quota` in micro-dollari, `checks::numbers`, `eval` con trasporto `curl`), `crates/mzlab-feat/src/params.rs` (stima di rumore, tolleranza, larghezza, tempo morto con prova), `mzlab-core::metodo` (`metodo/1`: gradiente, sorgente, esperimenti), `campioni/2` (ruoli proposti dai nomi).
- Cataloghi: `docs/schemi/strumenti-1.json` (71 KB, i 15 meta-strumenti), `steps-1.json`, `steps-cli-1.json`, `steps-tabui-1.json`, `steps-attributio-1.json`.
- Impostazioni note e documentate di XCMS (centWave: `ppm`, `peakwidth`, `snthresh`, `prefilter`; IPO e AutoTuner per la stima dai dati) e di mzmine (rumore, tolleranza m/z, ADAP chromatogram builder, join aligner): servono come riferimento dei valori tipici in `03-parametri.md`, non come codice.
- File demo: `tools/dati_sintetici.py` (full scan LR ESI+ ed ESI−, DDA LR, MRM con standard, polarità alternata, DDA Orbitrap, un file rotto, MS2). Nessun dato reale entra nel repository pubblico.

## Decisioni principali (dettagli nei file)

- Client: CLI (`mzlab-cli assist`, chiave da file, come `tutor eval`) e app desktop (trasporto `llm/*` esistente) subito; sito solo dopo la via V1.
- Modelli: `claude-sonnet-5-5` a effort `medium` per analisi e dialogo; `claude-haiku-5-5` per instradamento e specifiche dei grafici; Opus mai nel prodotto, solo come giudice facoltativo nel test.
- Niente codice libero: il modello propone piani `mzflow/2`, parametri e `grafico/1`, Rust valida, la pagina esegue e disegna.
- Nessuna modifica a `Cargo.toml`, workflow, `pages.yml`: i moduli nuovi stanno in `mzlab-llm` e `mzlab-cli`.

## Vie da chiedere a Federico

| Via | Opzione a | Opzione b |
|---|---|---|
| V1 sito | `api.anthropic.com` nella CSP di `llm/trasporto.html`, chiave dell'utente nel browser (intestazione `anthropic-dangerous-direct-browser-access`) | nessun host: l'assistente solo in desktop e CLI |
| V2 tetto per persona con chiave condivisa | Worker con contatori per persona (Cloudflare: non dato) | una chiave o un workspace per persona con limite di spesa della Console; nessun server |
| V3 valutazioni | Batch API con la chiave di test (−50 %, risultati entro 24 h) | solo chiamate dirette |
| V4 livello b dei registri | dopo il parere del responsabile privacy | resta spento (stato attuale) |

Il test con la chiave dedicata (via già data) non ha bisogno di V1, V2 o V3: gira dalla CLI con contatore locale e limite di spesa del workspace.
