# Proxy pubblico anonimo per l'API di Claude

Richiesta di Federico: l'assistente del sito presenta mzFlow a CHIUNQUE visiti il sito, senza chiave personale, senza account e senza registro delle persone, con limiti per IP; all'inizio solo per lui. La chiave Anthropic sta solo in un proxy lato server come segreto. Nessun account, deploy o segreto si crea da una chat: i passi esterni sono di Federico (§9).

## 1. Forma

```
browser: pagina ─▶ iframe llm/trasporto.html ──HTTPS (Origin, x-mzlab-session[, x-mzlab-owner])──▶ proxy
                                              ◀── SSE invariato + x-mzlab-spend ──                     │
                                                                                                      ├─▶ api.anthropic.com (x-api-key segreto)
app desktop: Rust llm/send (stesso corpo, stesse intestazioni) ─────────────────────────────────────────┘
proxy ── Ledger (un Durable Object): fase, interruttore, tetti globali, contatori per IP-hash e per sessione (24 h)
```

- Un modulo `assist-proxy.js` con la logica (Request/Response standard del web), un adattatore per host (`adapters/cloudflare.js`: `secret(name)`, `ledger`; `adapters/memoria.js` per le prove). Spostarsi di host = cambiare adattatore.
- Il costo e l'ammissione sono gli stessi di `mzlab-llm::spend` (`Meter::admit`/`charge`), compilati in un `.wasm` minuscolo (`crates/mzlab-spend-wasm`, P2 di `13-piano-dev.md`): regola «Tutto in Rust», un solo codice per CLI, desktop, sito e proxy.
- Parità sito/app: l'app desktop usa il proxy con lo stesso fornitore `mzlab-demo` di `fornitori-1.json` (url del proxy, nessuna chiave); il corpo è lo stesso byte per byte (test di parità in P4).

## 2. Host

| Host | Flusso lungo | Contatori atomici | Limite di frequenza | Note |
|---|---|---|---|---|
| **Cloudflare Workers (Free, poi Paid 5 $/mese)** | sì; la CPU conta solo il lavoro, non l'attesa | Durable Objects con SQLite (anche nel piano gratuito) | binding `ratelimit` integrato (finestra 10 s o 60 s, per località) | consigliato |
| Deno Deploy | da verificare | Deno KV atomico | da scrivere a mano | seconda scelta |
| Supabase Edge Functions | durata massima per richiesta da verificare | Postgres | a mano | un database in più da gestire |
| Vercel Hobby | sì | serve un servizio esterno | a mano | solo uso personale non commerciale: escluso |
| Netlify | no (60 s) | no | a mano | escluso |
| Cloudflare AI Gateway | sì | no (contatori del gateway, non per IP-hash con tetto in dollari) | per gateway | utile come registro di spesa, ma non chiude il prompt di sistema: non sostituisce il proxy |

Da verificare in P2 sulle pagine ufficiali prima del deploy: la CPU per richiesta nel piano gratuito con la lettura dell'`usage` dal flusso (si analizzano solo `message_start` e `message_delta`), la scelta della giurisdizione UE dei Durable Objects, il numero di richieste gratuite al giorno.

## 3. Fasi

| Fase | Chi passa | Come | Tetti |
|---|---|---|---|
| 0 solo Federico | chi manda `x-mzlab-owner: <frase segreta>` | il proxy confronta l'SHA-256 della frase con il segreto `OWNER_SHA256`; senza → 403 `err.llm.proxy.closed` | sessione 2 $, giorno 10 $ |
| 1 pubblico | ogni visitatore del sito | variabile `PHASE=1` (Federico la cambia); il proprietario resta riconosciuto e ha i suoi tetti | tetti di §4 |
| spento | nessuno | segreto o variabile `ASSIST_OFF=1` → 503 `err.llm.proxy.off`; oppure tetto globale raggiunto → 503 `err.llm.proxy.budget` | – |

La frase segreta si scrive una volta in un campo del pannello (solo nell'IndexedDB dell'iframe del trasporto, mai nella pagina principale) e si cancella con «Forget». Non è un account: è un interruttore che solo Federico conosce.

## 4. Tetti (fase 1, valori proposti; decisione D3)

| Tetto | Valore | Dove |
|---|---|---|
| `max_tokens` per richiesta | router 200, lettori 2 000, grafici 800, orchestratore 1 500 (piano intero 3 000) | proxy rifiuta oltre il tetto del ruolo (`err.llm.proxy.body`) |
| corpo della richiesta | ≤ 120 KB, ≤ 40 messaggi, ultimo messaggio dell'utente ≤ 2 000 caratteri | proxy |
| sessione | 0,50 $ (avviso all'80 %, 0,40 $) | proxy (`s:`) + `Meter` nel pannello |
| IP al giorno | 1,00 $ (≈ 2–4 sessioni), 3 sessioni nuove | proxy (`ip:`) |
| frequenza per IP | 12 richieste / 60 s, 1 richiesta in corso alla volta per sessione | binding `ratelimit` + ledger |
| globale al giorno | 6 $ | proxy (`g:giorno`) |
| globale nel mese | 150 $ | proxy (`g:mese`) |
| workspace della Console | 180 $ al mese, chiave solo per quel workspace | Anthropic Console (rete di sicurezza finale: 400 «workspace API usage limits» → `err.llm.limit.workspace`) |

Il pannello ha già `spend::Meter` con un tetto: in modalità demo il tetto del client è 0,50 $ e non si può alzare; il tetto vero è quello del proxy.

## 5. A ogni richiesta

1. Percorsi: solo `POST /v1/messages` e `OPTIONS`; altro → 404. CORS: `Access-Control-Allow-Origin` solo `https://freeesta.github.io` (e l'origine dell'app desktop, `mzlab://localhost` / `http://mzlab.localhost`); un'altra `Origin` → 403 senza chiamare il fornitore. Nessun cookie.
2. Interruttore e fase (§3).
3. **Corpo chiuso** (il proxy non è un chatbot gratuito):
   - `model` ∈ {`claude-sonnet-5-5`, `claude-haiku-5-5`}, con il ruolo che lo consente (`assist-ruoli-1.json`);
   - il blocco `system` deve avere un SHA-256 nella lista dei prompt di mzLab (`sistema-1.md`, `data/assist/0N_*.md`, generata al build del proxy dai file del repository); i messaggi `role: "system"` a metà conversazione idem (`06_piano.md`);
   - `tools` deve avere l'SHA-256 del catalogo demo (`strumenti/1` ridotto, P3) o mancare (lettori con `output_config.format`, schema anch'esso in lista);
   - niente strumenti lato server (`web_search`, `web_fetch`, `code_execution`, MCP), niente `service_tier`, niente `temperature/top_p/top_k`, niente immagini o documenti;
   - violazione → 400 `err.llm.proxy.body` con il nome del campo.
4. **Conteggio senza conservare l'IP**: `h = HMAC-SHA256(sale_del_giorno, IP)` troncato a 16 byte; il sale è casuale, nasce al primo uso del giorno UTC dentro il ledger, non esce mai dal ledger e si butta a mezzanotte insieme a tutti i contatori `ip:` e `s:` (allarme del Durable Object). L'IP in chiaro vive solo nella memoria della richiesta.
5. **Ammissione**: costo massimo = ingresso stimato (`ceil(byte/3)` × prezzo pieno) + `max_tokens` × prezzo d'uscita. Se uno qualsiasi fra sessione, IP, giorno, mese supererebbe il tetto con speso + riservato + massimo → 429 `err.llm.limit.spend` con `x-mzlab-limit: session|ip|day|month`, senza chiamare il fornitore. Altrimenti **riserva** il massimo in modo atomico (un solo ledger serializza).
6. Inoltro con `x-api-key` segreto e `anthropic-version: 2023-06-01`; i byte SSE tornano invariati mentre il proxy legge `usage` da `message_start` e `message_delta`.
7. **Saldo** a fine flusso: costo vero (ingresso, uscita, lettura e scrittura della cache) al posto della riserva; flusso tagliato o `usage` assente → resta la riserva intera.
8. Intestazioni di risposta: `x-mzlab-spend: <sessione>/<tetto>` in micro-dollari, `x-mzlab-soft: 1` sopra l'80 %.

## 6. Cosa il proxy non fa mai

- Non registra corpi di richiesta o risposta, testo dell'utente, nomi, IP, intestazioni `User-Agent`/`Referer`: nessun `console.log` di contenuto; nel `wrangler.toml` `observability` spento (niente Workers Logs, niente Logpush); un test rifiuta ogni `console.` nel modulo tranne `console.error(codice)` con una chiave fissa.
- Unico dato conservato: contatori (`ip:<hash>`, `s:<id sessione>`, `g:giorno`, `g:mese`) in micro-dollari; `ip:` e `s:` durano al più 24 h; `g:` porta solo somme. Nessun elenco di sessioni, nessun orario per richiesta.
- Non ritenta da solo, non rivela la chiave (nessuna intestazione del fornitore tranne `retry-after` e `request-id`; un messaggio d'errore che somiglia a una chiave si scarta come in `provider::http_error`).

## 7. Cosa parte dal browser

- Solo ciò che Rust ha già ridotto: `ispezione/1`, `contesto/1`, `proposta-param/1`, `tabcheck/1` (≤ 20 righe per controllo), viste di `grafico/1`, il piano `mzflow/2` e il testo che la persona scrive nel pannello. Mai file grezzi, spettri interi, percorsi, operatore, seriale, nome del computer.
- In modalità demo «Hide file names» è sempre acceso e non si spegne: i file sono `f1…fn`, i campioni con i ruoli (`QC`, `blank`, `sample`) e un numero.
- **Avviso di consenso** (`llm-consenso.js`, classe nuova `inspection`) prima della prima chiamata di ogni visita, consenso salvato in `sessionStorage` (vale per la scheda aperta, si richiede alla visita seguente). Testo proposto in `14-decision-card.md` D4: che cosa parte (i fatti ridotti, mostrati con «What will be sent»), a chi (proxy di mzLab su Cloudflare, poi Anthropic come fornitore del modello), che mzLab non conserva nulla, che Anthropic tratta i dati secondo le sue condizioni commerciali API, che le parti senza modello funzionano senza consenso.
- Prima della pubblicazione (fase 1) il testo va mostrato al responsabile della protezione dei dati di UniTO: l'IP è un dato personale anche se trattato solo in memoria.

## 8. Abusi e difese

| Attacco | Difesa |
|---|---|
| iniezione dai metadati del file (nome del campione, commento del metodo, celle) | già nel prodotto: alias, stringhe ≤ 64 caratteri, dati in blocchi JSON, politica che non legge testo (`05-sicurezza.md` §1); nessuno strumento apre la rete o legge file |
| usare il proxy come chatbot gratuito | prompt di sistema e catalogo in lista (§5.3), testo dell'utente ≤ 2 000 caratteri, `max_tokens` piccoli, il prompt `01_sistema.md` rifiuta ciò che non riguarda mzLab e i dati aperti, tetti per IP |
| amplificazione del costo (contesti enormi, molti turni) | corpo ≤ 120 KB, ≤ 40 messaggi, riserva del caso peggiore prima dell'inoltro, tetto di sessione |
| bot e IP che ruotano | tetto globale giornaliero (6 $): il danno massimo è un giorno di demo; Turnstile solo se succede (aggiunge un host alla CSP: decisione D6) |
| richieste da altri siti | CORS + controllo di `Origin` nel proxy (un client non browser può mentire sull'Origin: per questo contano i tetti, non il CORS) |
| furto della chiave | la chiave esiste solo come segreto del proxy e nella Console; workspace dedicato con limite di spesa; rotazione = nuovo segreto + revoca |

## 9. Passi di Federico (nessuna chat li fa)

1. Console Anthropic: workspace «mzlab-demo», limite di spesa mensile 180 $, una chiave solo per quel workspace.
2. Cloudflare (piano gratuito): account; dopo l'unione di P2 dalla cartella del proxy `npx wrangler secret put ANTHROPIC_KEY`, `npx wrangler secret put OWNER_SHA256` (SHA-256 della frase segreta, calcolato in locale), `npx wrangler deploy`.
3. Mandare al Coordinatore solo l'indirizzo pubblico del proxy (serve alla CSP di P4), mai chiave o frase.
4. Fase 1: `PHASE=1` in `wrangler.toml` o nel cruscotto, dopo la demo provata e il parere privacy. Spegnere: `ASSIST_OFF=1`, o revocare la chiave nella Console.

## 10. Quando il proxy manca

Budget finito, proxy spento o irraggiungibile, consenso negato: il sito funziona come oggi. La carta dell'assistente dice «Assistant not available» (chiave `lang/`), una volta; restano l'ispezione, le proposte con «Apply», i controlli della tabella e i grafici predefiniti (dopo P1), senza alcuna richiesta di rete.
