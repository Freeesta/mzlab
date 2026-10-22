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
| `09-worker.md` | Worker portabile per la chiave condivisa: invito, riserva e saldo, contatori per persona, cosa non registra |
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

- Client: CLI (`mzlab-cli assist`, chiave da file, come `tutor eval`) e app desktop (trasporto `llm/*` esistente) subito; sito con la chiave dell'utente (V1a) o con un codice d'invito verso il Worker (V2a, `09-worker.md`).
- Modelli: `claude-sonnet-5-5` a effort `medium` per analisi e dialogo; `claude-haiku-5-5` per instradamento e specifiche dei grafici; Opus mai nel prodotto, solo come giudice facoltativo nel test.
- Niente codice libero: il modello propone piani `mzflow/2`, parametri e `grafico/1`, Rust valida, la pagina esegue e disegna.
- Nessuna modifica a `Cargo.toml`, workflow, `pages.yml` in A1–A8: i moduli nuovi stanno in `mzlab-llm` e `mzlab-cli`. Worker e CSP in PR separate (W0–W2) con via del Coordinatore.

## Host per le prove del Worker (gratuiti, letti il 22/10/2026)

| Host | Richieste gratuite | Flusso SSE di 60–120 s | Contatori atomici | Limiti d'uso | Segreto |
|---|---|---|---|---|---|
| Cloudflare Workers Free | 100 000/giorno | sì, finché il client è collegato; CPU 10 ms (l'attesa non conta) | Durable Objects SQLite: 100 000 righe/giorno; KV no (1 000 scritture/giorno) | nessuno | `npx wrangler secret put ANTHROPIC_KEY` o cruscotto |
| Deno Deploy Free | 1 000 000/mese, CPU 10 h/mese | durata non scritta nelle pagine lette | Deno KV con transazioni atomiche: 500 000 unità di scrittura/mese | «for personal use and smaller projects» | variabili d'ambiente del progetto (comando non verificato) |
| Vercel Hobby | 1 000 000 invocazioni/mese | sì: 300 s per funzione | troppo piccolo (Blob 2 000 scritture/mese) senza un servizio esterno | solo uso personale non commerciale: escluso per un corso | variabili del progetto |
| Netlify Free | a crediti (300) | no: le funzioni in flusso si fermano a 60 s | Blobs senza controllo di concorrenza | nessuno trovato | variabili del sito |

Fonti: pagine ufficiali di limiti e prezzi di ciascun host (Cloudflare: workers/platform/limits, durable-objects/platform/pricing, workers/configuration/secrets; Deno: deploy/pricing; Vercel: docs/limits, pricing; Netlify: functions/api, netlify-blobs). Non verificati: regione dei dati e scelta UE (nessuno dei quattro la dice nelle pagine lette per il piano gratuito); da controllare prima di dare inviti a studenti.

**Consiglio: Cloudflare Workers Free con un Durable Object per persona.** È l'unico dei quattro con flusso lungo, contatori fortemente coerenti (un oggetto per persona serializza riserva e saldo) e nessun limite d'uso. Una richiesta scrive 2–3 righe: 100 000 righe/giorno bastano per ≈ 30 000 richieste.

Rischio da misurare in W1: i 10 ms di CPU per richiesta con la lettura dell'`usage` dal flusso (si cercano solo gli eventi `message_start` e `message_delta`, senza decodificare il resto). Il passaggio a un piano a pagamento (Cloudflare Workers Paid o un altro host con il suo adattatore) non cambia la logica. Seconda scelta: Deno Deploy (KV atomico, ma durata del flusso da verificare).

Passi di Federico (nessuno li fa una chat):
1. Creare un account Cloudflare (piano gratuito) e installare `wrangler` sul Mac.
2. Creare nella Console Anthropic un workspace «assistente» con limite di spesa (190 $) e una chiave solo per quel workspace.
3. Dopo l'unione di W1: `npx wrangler secret put ANTHROPIC_KEY` (incolla la chiave quando la chiede) e `npx wrangler deploy` dalla cartella del Worker.
4. Mandare al Coordinatore solo l'indirizzo pubblico del Worker (serve alla CSP di W2), mai la chiave.
5. Creare gli inviti con `mzlab-cli assist invito nuovo` e darli alle persone a voce o per posta; revocarli a fine prova.

## Vie (decise da Federico il 22/10/2026: V1a, V2a, V3a; V4 aperta)

| Via | Scelta (a) | Alternativa scartata (b) |
|---|---|---|
| V1 sito | `api.anthropic.com` nella CSP di `llm/trasporto.html`, chiave dell'utente nel browser (intestazione `anthropic-dangerous-direct-browser-access`) | nessun host: l'assistente solo in desktop e CLI |
| V2 tetto per persona con chiave condivisa | Worker portabile con contatori per persona (`09-worker.md`), prima su un host gratuito per le prove, poi a pagamento | una chiave o un workspace per persona con limite di spesa della Console; nessun server |
| V3 valutazioni | Batch API con la chiave di test (−50 %, risultati entro 24 h) | solo chiamate dirette |
| V4 livello b dei registri | dopo il parere del responsabile privacy | resta spento (stato attuale) |

Il test con la chiave dedicata gira dalla CLI con contatore locale e limite di spesa del workspace; le valutazioni a chiamata singola (T1–T8) passano dalla Batch API (V3a). Il Worker non serve al test: serve quando più persone usano la chiave condivisa.
