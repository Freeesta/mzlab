# Studio: assistente dati con l'API di Claude

Obiettivo: una chat che apre i file caricati, capisce che cosa ha davanti (strumento, polarità, risoluzione, cromatografia, gradiente, tipo di acquisizione, QC, bianchi), fa domande all'utente (matrice, composti attesi, obiettivo), propone il workflow di mzFlow e i suoi parametri fondamentali con motivo e confidenza, legge la feature table e disegna grafici.

Uso pubblico (richiesta di Federico): l'assistente presenta mzFlow a chiunque visiti il sito, senza chiave personale, senza account e senza registro delle persone, attraverso un proxy anonimo con tetti per sessione, per IP, per giorno e per mese; all'inizio solo per Federico (fase 0). Budget API: 200 $.

Questo studio non contiene codice di prodotto e non ha fatto chiamate all'API. Costruisce sopra ciò che esiste (`docs/agenti/llm.md`, `llm-subagenti.md`, `llm-lettori.md`: `mzlab-llm`, agente, politica, `contesto/1`, `ispezione/1`, `proposta-param/1`, `tabcheck/1`, `grafico/1`, lettori, trasporti, `spend`) e dice che cosa manca (`11-stato.md`).

## Documenti

| File | Contenuto |
|---|---|
| `01-architettura.md` | dove gira il client, chiave, cosa parte e cosa no, strumenti = passi, piano validato in Rust, `grafico/1`, subagenti |
| `02-costo.md` | prezzi verificati, token per fase, caching, tetti (10 $ per le prove di Federico, 0,50 $ per la demo pubblica), piano del test |
| `03-parametri.md` | segnali dei file → parametri di mzFlow: regole deterministiche in Rust, spiegazione del modello, confidenza |
| `04-feature-table.md` | controlli sulla feature table (QC, bianchi, duplicati, addotti, isotopi, deriva RT), domande, formato dell'uscita |
| `05-sicurezza.md` | iniezione dai nomi e dai metadati, limiti, registri (livello b spento), funzionamento senza rete |
| `06-valutazione.md` | compiti d'oro sui file sintetici, metriche, soglie di accettazione |
| `07-mappa-codice.md` | cosa riusare, cosa inventare, tetti `.wasm`, piano di PR |
| `08-prompt-dev.md` | prompt DEV di A1–A8 (A1–A6 fatte) e rimando ai prompt P1–P5 |
| `09-worker.md` | proxy pubblico anonimo: host, fasi 0/1, tetti, corpo chiuso, conteggio per IP senza conservarlo, cosa non registra, cosa parte, abusi, passi di Federico |
| `10-opzioni.md` | Copilot, server MCP, sandbox (D6), proxy pubblico e corse in cloud come opzioni: uso, vantaggi, rischi, alternativa locale, scheda per chiedere il sì di Federico |
| `11-stato.md` | A1–A8 su `main` con le prove; che cosa gira oggi nel sito |
| `12-demo.md` | copione della demo di mzFlow, token e costo per corsa, prove con le registrazioni, quante sessioni compra il budget |
| `13-piano-dev.md` | PR P1–P5 in ordine, zone, modello, `.wasm`, accettazione |
| `14-decision-card.md` | scheda delle decisioni di Federico (in inglese) |
| `prompt/` | i prompt per l'API (sistema, router, subagenti, valutazione), `00_esecuzione-test.md` e i prompt DEV `dev-P1…P5` (inglese) |

## Cosa farebbe un professionista (e non era chiesto)

1. **Il modello non misura, interpreta.** Ogni numero (larghezza dei picchi, rumore, errore di massa, deriva RT, CV dei QC) lo calcola Rust in modo deterministico; il modello riceve i numeri con la prova e li spiega o fa domande. Così l'errore numerico del modello è zero per costruzione e il test è ripetibile.
2. **Compiti d'oro prima dei prompt.** Il set con risposta nota (file sintetici di `tools/dati_sintetici.py`, `06-valutazione.md`) si scrive prima di spendere un dollaro; ogni prompt ha un criterio di accettazione misurabile.
3. **Costo per compito completato**, non per richiesta: una sessione economica che richiede tre giri in più non è economica (`02-costo.md`).
4. **Prefisso stabile e cache**: testo di sistema e catalogo `strumenti/1` congelati (byte stabili, già garantiti dal test `llm_tools`), dati variabili in coda; si verifica `cache_read_input_tokens` > 0 al secondo turno, altrimenti il test si ferma.
5. **Uscite strutturate ovunque**: i subagenti rispondono con `output_config.format` (schema JSON), gli strumenti hanno `strict: true` (`additionalProperties: false`); Sonnet 5.5 rifiuta `tool_choice` forzato, quindi `auto` + istruzione + controllo che la chiamata ci sia.
6. **Validazione in Rust di tutto ciò che il modello produce**: piano (`flow::check`), parametri (limiti dello `step/1`), grafico (`chart::validate`), testo (ogni numero citato deve stare nei dati inviati: `text::numbers` in `reading::validate`).
7. **Tetto di spesa a più livelli**: contatore nel client (arresto morbido all'80 %), tetti del proxy per sessione, IP, giorno e mese con riserva del caso peggiore, limite di spesa del workspace dedicato nella Console, stima prima dell'invio sopra 50 000 token.
8. **Difesa dall'iniezione** dai metadati: nomi e campi liberi passano come stringhe JSON delimitate, tagliate e con alias; la politica guarda solo strumento e argomenti (`policy::decide`, già così); set di attacchi rossi nel test.
9. **Riproducibilità e audit**: ogni turno ha SHA-256 della richiesta, modello servito, `usage`, costo e versione dei prompt in `llmlog/1` (livello a: niente testo); ogni corsa lanciata dall'assistente porta il blocco `llm` in `prov/1` (esiste).
10. **Degradazione senza rete**: l'ispezione, le proposte dei parametri, i controlli della tabella e i grafici predefiniti sono Rust e funzionano senza modello; il modello aggiunge solo spiegazione e dialogo.
11. **Gestione delle risposte particolari**: `stop_reason` `refusal` (Sonnet 5.5 ha classificatori), `max_tokens`, 429 con e senza `retry-after`, 400 di limite di spesa (`You have reached your specified workspace API usage limits`): ognuno ha la sua chiave `err.llm.*` e nessun ritentativo dove non serve.
12. **Prompt versionati come dati**: ogni prompt è un file con SHA-256 registrato nel log; cambiare un prompt = nuova versione e nuova corsa del set d'oro.

## Materiale di partenza

- Documentazione ufficiale letta il 22/10/2026: prezzi (`platform.claude.com/docs/en/about-claude/pricing`), limiti di spesa e di frequenza (`/docs/en/api/rate-limits`), caching, uso degli strumenti, uscite strutturate, effort (guida `claude-api`). I numeri sono in `02-costo.md`.
- Codice: `crates/mzlab-llm` (fornitori, adattatori, `Replay`, `llmlog/1`, agente, politica, `contesto/1`, valutazioni), `spend` (tetti in micro-dollari), `crates/mzlab-feat/src/params.rs` (stima di rumore, tolleranza, larghezza, tempo morto con prova), `mzlab-core::metodo` (`metodo/1`: gradiente, sorgente, esperimenti), `campioni/2` (ruoli proposti dai nomi).
- Cataloghi: `docs/schemi/strumenti-1.json` (71 KB, i 15 meta-strumenti), `steps-1.json`, `steps-cli-1.json`, `steps-tabui-1.json`, `steps-attributio-1.json`.
- Impostazioni note e documentate di XCMS (centWave: `ppm`, `peakwidth`, `snthresh`, `prefilter`; IPO e AutoTuner per la stima dai dati) e di mzmine (rumore, tolleranza m/z, ADAP chromatogram builder, join aligner): servono come riferimento dei valori tipici in `03-parametri.md`, non come codice.
- File demo: `tools/dati_sintetici.py` (full scan LR ESI+ ed ESI−, DDA LR, MRM con standard, polarità alternata, DDA Orbitrap, un file rotto, MS2). Nessun dato reale entra nel repository pubblico.

## Decisioni principali (dettagli nei file)

- Client: app desktop (trasporto `llm/*`, chiave propria facoltativa) e sito; nel sito nessuna chiave personale: l'unico fornitore cloud è il proxy anonimo `mzlab-demo` (`09-worker.md`), raggiunto anche dall'app con lo stesso corpo (parità). La CLI `mzlab-cli assist` (A7) serve alle valutazioni.
- Modelli: `claude-sonnet-5-5` a effort `medium` per lettori e orchestratore; `claude-haiku-5-5` a effort `low` per router e grafici; Opus mai nel prodotto, solo come giudice facoltativo nel test.
- Niente codice libero: il modello propone piani `mzflow/2`, parametri e `grafico/1`, Rust valida, la pagina esegue e disegna.
- Demo pubblica: catalogo ridotto (prefisso ≈ 8 000 token), tetto 0,50 $ per sessione, costo tipico ≈ 0,28 $ (`12-demo.md`); prove con registrazioni `llmreg/1` in modo stretto, così la demo dal vivo può sempre ripiegare su «Play a recorded demo».
- Senza proxy, senza budget o senza consenso il sito funziona come oggi più le parti senza modello (ispezione, proposte, controlli, grafici predefiniti) dopo P1.
- Nessun account, deploy, segreto o spesa da una chat: i passi esterni sono di Federico (`09-worker.md` §9); le sue scelte sono in `14-decision-card.md`. Copilot, server MCP, sandbox e corse in cloud restano opzioni spente (`10-opzioni.md`).

## Vie

| Via | Stato |
|---|---|
| V1 sito | nessuna chiave dell'utente nel sito; il trasporto raggiunge `localhost`, `replay` e, dopo P4, solo l'host del proxy |
| V2 tetti con chiave condivisa | proxy anonimo con contatori per sessione, IP-hash, giorno e mese (`09-worker.md`), al posto degli inviti personali |
| V3 valutazioni | Batch API con la chiave di test (−50 %) dalla CLI (A7) |
| V4 livello b dei registri | spento finché non c'è il parere del responsabile privacy; il proxy non registra mai contenuti |
