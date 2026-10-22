# Worker portabile: chiave condivisa e tetto per persona (via V2a)

Scelta di Federico: V1a (sito con chiave dell'utente) e V2a (Worker con contatori per persona) insieme. Due modi d'uso nel sito e nel desktop:
- **chiave propria**: il trasporto chiama `api.anthropic.com` diretto; tetto della persona = limite della sua chiave nella Console + contatore locale `mzlab-llm::spend` (10 $ per sessione);
- **invito**: il trasporto chiama il Worker con un codice d'invito personale; la chiave condivisa sta SOLO nel Worker come segreto; il Worker tiene i contatori per persona e per sessione.

Nessun account, deploy o segreto si crea da una chat: i passi esterni li fa Federico (README, «Host per le prove»). Il codice del Worker e la modifica della CSP vanno in PR DEV separate (W1, W2 in `07-mappa-codice.md` §5), dopo questa PR e con via esplicita del Coordinatore.

## 1. Forma

```
browser (trasporto.html) ──HTTPS + Bearer invito──▶ Worker ──x-api-key (segreto)──▶ api.anthropic.com
                          ◀── SSE invariato + x-mzlab-spend ──      ◀── SSE ──
                                   │
                                   └── archivio dei contatori (per persona, per sessione)
```
- Un solo modulo `assist-proxy.js` con la logica (Request/Response standard del web, nessuna API di un host); un adattatore piccolo per host (`adapters/<host>.js`) che dà due cose: `secret(name)` e `store` (contatori atomici). Spostarsi da un host gratuito a uno a pagamento = cambiare adattatore e rifare i passi manuali, non la logica.
- Calcolo del costo e regola d'ammissione: le stesse di `mzlab-llm::spend` (`02-costo.md` §4). Due modi (decisione del Coordinatore, via W0): **(a, consigliato)** `spend` compilato in un `.wasm` minuscolo caricato dal Worker (regola «Tutto in Rust», un solo codice per CLI, desktop, sito e Worker; serve un bersaglio di build nuovo, quindi Cargo e workflow); **(b)** aritmetica in JS nel Worker con un test di parità contro `spend` sugli stessi casi (nessun cambio di Cargo, ma un'eccezione alla regola «JS solo interfaccia» da scrivere in AGENTS.md).

## 2. Cosa fa a ogni richiesta

1. Solo `POST /v1/messages` e `POST /v1/messages/count_tokens`; ogni altro percorso 404. CORS: `Access-Control-Allow-Origin` solo per l'origine del sito (`https://freeesta.github.io`) e per l'app desktop; preflight `OPTIONS` con le sole intestazioni necessarie.
2. Invito: `Authorization: Bearer <codice>`; il Worker guarda l'SHA-256 del codice nell'archivio (mai il codice in chiaro); codice sconosciuto o revocato → 401 `err.llm.invite`.
3. Corpo: ≤ 400 KB; `model` in lista (`claude-sonnet-5-5`, `claude-haiku-5-5`); `max_tokens` ≤ 6 000; nessuno strumento lato server (`web_search`, `web_fetch`, `code_execution`, MCP: costano e aprono la rete); niente `service_tier` diverso da quello predefinito; niente Batch (le valutazioni Batch passano dalla CLI con la chiave di test, V3a). Violazione → 400 `err.llm.proxy.body` con il campo.
4. Ammissione: costo massimo = ingresso stimato (`ceil(byte/3)` × prezzo pieno) + `max_tokens` × prezzo d'uscita. Se speso + riservato + massimo supera il tetto della persona (10 $) o della sessione (10 $) → 402 `err.llm.limit.spend` senza chiamare il fornitore. Altrimenti **riserva** il massimo in modo atomico.
5. Inoltro con la chiave condivisa e `anthropic-version`; i byte SSE tornano al browser invariati mentre il Worker legge `message_start.usage` e `message_delta.usage` al volo.
6. Fine (o errore, o chiusura del browser): **saldo** = costo vero da `usage` (ingresso, uscita, lettura e scrittura della cache) al posto della riserva; se `usage` manca (flusso tagliato) resta la riserva intera, mai zero.
7. Intestazione `x-mzlab-spend: <speso>/<tetto>` in micro-dollari e `x-mzlab-soft: 1` sopra l'80 %: il pannello lo dice una volta (`02-costo.md` §4, arresto morbido).

## 3. Contatori

| Chiave | Valore | Durata |
|---|---|---|
| `inv:<sha256>` | persona (id breve casuale), tetto in micro-dollari, stato (attivo/revocato), scadenza | fino alla revoca o alla scadenza |
| `p:<persona>` | speso, riservato | per tutta la durata dei crediti (fino al 16/11/2026), poi azzerabile da Federico |
| `s:<persona>:<sessione>` | speso, riservato | 24 h dall'ultima richiesta |
| `g:totale` | speso di tutte le persone; tetto globale (190 $, sotto il limite del workspace) | come `p:` |

- La sessione è un id casuale che il client crea all'apertura del pannello; nessun dato della persona dentro.
- Le operazioni riserva e saldo devono essere atomiche per persona: due schede aperte non possono superare il tetto. Un archivio solo «eventualmente coerente» non basta; l'adattatore usa l'oggetto con stato o la transazione dell'host (README, tabella degli host).
- Il tetto globale e il limite di spesa del workspace nella Console restano la rete di sicurezza finale: 400 «specified workspace API usage limits» → `err.llm.limit.workspace` per tutti, senza ritentare.

## 4. Cosa il Worker non fa mai

- Non scrive né registra corpi di richiesta o di risposta, nomi di file, testo dell'utente; nessun `console.log` del contenuto; i registri dell'host con i corpi spenti.
- Registro minimo (livello a, `05-sicurezza.md` §3): ora, id della persona, modello, `usage`, costo, stato HTTP, durata. Nient'altro.
- Non conserva dati grezzi: di nessun file passa mai il contenuto (il client manda solo `ispezione/1`, proposte, controlli, come in `01-architettura.md` §3).
- Non rivela la chiave: la chiave non compare in nessuna risposta, intestazione o errore; gli errori del fornitore si inoltrano con `type` e `message` ma senza intestazioni del fornitore tranne `retry-after` e `request-id`.
- Non ritenta da solo: i ritentativi li decide il client con le regole di `05-sicurezza.md` §2.

## 5. Inviti e revoca (passi di Federico)

- Un comando locale (W1: `mzlab-cli assist invito nuovo|revoca|elenco`, nessuna rete: stampa i comandi dell'host da eseguire o il JSON da caricare) crea il codice (128 bit casuali, base32), ne stampa una volta il valore da dare alla persona e scrive nell'archivio solo l'SHA-256.
- Revoca: stato `revocato`; effetto alla richiesta successiva. Interruttore generale: segreto `ASSIST_OFF=1` → 503 `err.llm.proxy.off` per tutti.
- Rotazione della chiave condivisa: nuovo segreto, revoca della vecchia chiave nella Console; i contatori restano.

## 6. Prove (W1, senza rete)

- Il modulo gira con `fetch` finto del fornitore (registrazioni SSE di `llmreg/1`) e un adattatore `store` in memoria: tetto mai superato con 2 richieste in parallelo; riserva saldata con `usage`; flusso tagliato = riserva intera; CORS rifiutato da un'altra origine; corpo con `web_search` rifiutato; chiave assente da ogni uscita (stesso controllo di `log::has_secret`).
- Parità del costo con `mzlab-llm::spend` sui casi di `02-costo.md` (con la via W0 a è lo stesso codice).

## 7. Cosa cambia nel sito

- `llm/trasporto.html`: la CSP `connect-src` aggiunge `https://api.anthropic.com` (chiave propria) e l'host del Worker (invito); nessun altro host. Modifica di CSP = PR W2 con via esplicita.
- Pannello: scelta «My API key» o «Invitation code» (testi in `lang/`), codice salvato solo nell'IndexedDB dell'iframe del trasporto, mai nella pagina principale; contatore di spesa dalla risposta del Worker.
