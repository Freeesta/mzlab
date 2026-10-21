# Studio: tutor sulla Teoria (domande degli studenti, risposte dai paragrafi)

Obiettivo (decisione e consenso esplicito di Federico, 21/10): gli studenti fanno domande a una chat; la risposta nasce dai paragrafi della Teoria recuperati per la domanda (RAG, nessun addestramento) più un modello linguistico, con la citazione del paragrafo. Stile didattico: spiega il concetto, non risolve gli esercizi, rimanda al paragrafo e chiude con una domanda di ritorno. Il testo della Teoria può essere inviato al fornitore del modello; la Teoria non si modifica. Interruttore per corso, spento di default. I prompt DEV sono in `prompt-dev.md`.

Questo studio sostituisce le decisioni di `docs/agenti/assistente-teoria.md` (che rimanda qui) e riusa lo studio `docs/studi/llm/README.md` (contratti dei fornitori), senza duplicarlo.

## 1. Cosa esiste oggi (dal codice)

- **Teoria**: 23 capitoli + indice + 9 pagine di Pratica in `mzlab/web/teoria/*.html` (33 pagine vere, esclusi i rimandi `meta refresh`), ≈ 294 000 caratteri di testo visibile, 306 titoli `h2`/`h3`, nessun capitolo `data-esteso`. Le sorgenti Markdown del Libro stanno nel repository privato; il sito pubblica l'HTML.
- **Esercizi**: le domande di verifica sono `<details class="q"><summary>domanda</summary>risposta</details>` (`teoria/LEGGIMI.md:62`); la risposta è nascosta finché lo studente non clicca. Altri riquadri: `.box.gym`, `details.poe` («Prevedi, poi guarda»), `details.figdesc` (descrizioni delle figure).
- **Ancore**: `slug()` dei titoli h2/h3 (`crates/mzlab-libro/src/scan.rs:94`, `index.rs:62`), le stesse di `indice-ricerca.js` (generato da `tools/genera_indice_teoria.py`, porta Rust in `mzlab_libro::reindex`). Sinonimi `SINONIMI` in `crates/mzlab-libro/src/index.rs`.
- **CSP**: `index.html` ha `connect-src 'self' blob:`; le pagine `teoria/*.html` non hanno una CSP propria; le pagine amministratore `libro/editor.html` hanno una CSP propria con un solo host esterno (modello da riusare).
- **LLM**: lo studio `docs/studi/llm/README.md` definisce `llm-provider/1`, `llm-conv/1`, eventi normalizzati, `llmlog/1`, il profilo `mzlab.profilo` (`corso`|`ricerca`) e il crate `crates/mzlab-llm` (prompt D2, non ancora unito). Nessuna riga di codice LLM nel repository.
- **Build**: `pages.yml` ha Rust (cargo, wasm-bindgen) e `tools/build_site.py`; nessun servizio server-side.

## 2. Principi (vincolanti per i prompt)

1. **Non dà le risposte** (AGENTS.md §1.1): niente soluzioni delle domande di verifica, niente TP dei dati del laboratorio, niente rette di taratura o tabelle della relazione. Spiega il concetto, cita, rimanda, fa una domanda di ritorno.
2. **Solo la Teoria**: ogni affermazione porta la citazione di un blocco recuperato; se i blocchi non bastano, frase fissa «La Teoria non tratta questo punto» + i paragrafi più vicini. Nessuna conoscenza esterna presentata come Teoria.
3. **Teoria intatta**: nessun file di `mzlab/web/teoria/` cambia (contenuto, `comandi.js`, CSS). Il tutor è una pagina a parte che collega le ancore esistenti.
4. **Tutto in Rust**: blocchi, ricerca, costruzione della richiesta, controllo delle citazioni, conti dei tetti, valutazioni: Rust. Nel Worker e nella pagina il JS è solo colla (fetch, memoria, interfaccia).
5. **Spento di default**: nessuna richiesta al Worker finché il corso non è acceso e lo studente non ha letto l'avviso (§6).
6. **Blocco duro sui costi**: nessuno paga oltre i 200 $ di crediti API (ricarica automatica spenta, nessuna carta); a crediti finiti il tutor resta in modo «solo paragrafi» (§3.4).

## 3. Decisioni

### 3.1 Architettura: dove sta la chiave

| | (A) chiave dello studente | (B) Worker serverless con la chiave (scelta) | (C) macchina vera (crediti Student Pack) |
|---|---|---|---|
| Chiave | nel browser dello studente | segreto del Worker, mai nel browser | sul server |
| Costo per lo studente | account API e carta | 0 | 0 |
| Manutenzione | guidare ogni studente | un Worker, un Durable Object, zero server | sistema operativo, aggiornamenti, TLS, backup |
| Abuso | nessuno per Federico | limiti per studente, IP, corso, globale | come B, più superficie |
| Privacy | domanda al fornitore | domanda → Worker (non registra) → fornitore | come B, più registri di sistema da spegnere |
| Durata | finché lo studente paga | finché ci sono crediti | finché ci sono i crediti della macchina |

Raccomandazione: **(B) Cloudflare Worker** (piano gratuito) con la chiave Anthropic come segreto, un Durable Object (archivio SQLite) per i contatori. (A) non è realistica per il corso; (C) costa manutenzione senza vantaggi. I limiti del piano gratuito (richieste al giorno, CPU per richiesta, Durable Object con SQLite nel piano gratuito) si controllano al deploy: la CPU conta poco perché l'attesa del fornitore non è CPU.

### 3.2 Indice della Teoria per il recupero (`teoria-blocchi/1`)

| | (a) JS nel browser | (b) Python in `tools/` | (c) Rust offline al build (scelta) |
|---|---|---|---|
| Regole | viola Tutto in Rust | vietato: nessuna funzione nuova in Python | rispettate |
| Riuso | – | `genera_indice_teoria.py` | `mzlab-libro` (`slug`, `SINONIMI`, lettura HTML) |

- **Generatore**: modulo `crates/mzlab-libro/src/blocchi.rs` + binario `teoria-blocchi` (dipende solo da `mzlab-libro`, compila in secondi). Legge `mzlab/web/teoria/*.html`, scrive `blocchi-1.json`. Deterministico, chiavi ordinate.
- **Blocco**: paragrafi consecutivi sotto lo stesso h2/h3 fino a ~800 caratteri; voce di glossario = un blocco; equazioni come testo; figure solo con didascalia e `figdesc`; esclusi `script`, simulazioni, menu. Stima: 400-450 blocchi.
- **Esercizi**: `details.q` → blocco `kind: "esercizio"` con la sola domanda (`summary`); il testo della risposta NON entra nell'indice inviato al modello, ma il generatore ne scrive l'impronta (`hidden_sha` e le 5-grammi in `hidden_shingles`) per il controllo di non-soluzione (§3.6). `details.poe` uguale.
- **Id e citazioni stabili**: `id = <file senza .html>#<ancora>.<k>` (k = ordine nella sezione), `sha` = primi 12 esadecimali di SHA-256 del testo normalizzato. La citazione mostrata è «Cap. N, titolo della sezione» con link `teoria/<file>.html#<ancora>`. Un titolo cambiato cambia l'ancora: le conversazioni non si salvano, quindi nessun link vecchio da mantenere; i casi di eval citano ancore e un test segnala quelle sparite.
- **Ricerca**: BM25 in Rust (`mzlab-libro::cerca`, normalizzazione senza accenti e maiuscole come `cerca.js`, sinonimi di `SINONIMI`), più un peso alla sezione aperta dallo studente. Niente embedding (30-120 MB, fuori dal principio 2); si riconsiderano solo se il richiamo misurato è sotto l'obiettivo (§5).
- **Dimensione**: ≈ 330 KB JSON, ≈ 90 KB compressi da Pages. Non va nel sito come `.wasm` né nella pagina: lo legge il Worker.
- **Aggiornamento**: il file non si committa; `pages.yml` lo genera a ogni pubblicazione (passo dopo i `.wasm`) e lo copia in `site/static/tutor/blocchi-1.json`. Il Worker lo scarica da Pages con cache di 1 h: una Teoria cambiata arriva al tutor senza ridistribuire il Worker. Il test `tests/test_tutor_blocchi.py` lo rigenera e ne controlla schema, numero di blocchi e assenza dei testi delle risposte.

### 3.3 Interfaccia con lo studio LLM (riuso, niente doppioni)

- **Fornitore**: il Worker usa `llm-provider/1` (`crates/mzlab-llm/data/fornitori-1.json`, adattatore `anthropic`) e `mzlab_llm::provider::{request, Stream}` del prompt D2; la logica nuova sta in `crates/mzlab-tutor` (dipende da `mzlab-llm` e `mzlab-libro`). Nessun secondo adattatore.
- **Profilo**: `mzlab.profilo = corso` resta «nessun LLM nella vista Dati e nel Flow». Il tutor è l'unica eccezione per il corso e non dipende dal profilo: dipende dall'interruttore del corso (§3.4) e dall'avviso letto. Una riga nel §8 dello studio LLM lo dice.
- **Registro**: il tutor NON scrive `llmlog/1` (non fa azioni) né `contesto/1` (non vede dati). Ha due schemi suoi: `tutor-ask/1` (richiesta) e `tutorstat/1` (contatori, §6).
- **Chiavi**: stessa guardia (`sk-`, `sk-ant-`, `AIza`) di `tests/test_llm_chiavi.py` estesa a `servizi/tutor-worker/`.

### 3.4 Worker: contratto, limiti, costi

**Rotte** (prefisso `/v1`, solo HTTPS):
- `POST /v1/ask` corpo `tutor-ask/1`: `{course, client, lang: it|en, question ≤ 600 caratteri, where?: "09-quadrupolo#stabilita", history: ≤ 2 turni (domanda + risposta ≤ 1 500 caratteri), share: false}`. Risposta in stream SSE: `meta {answer_id, mode: llm|passi, blocks:[{id, file, anchor, title}]}`, `delta {text}`, `done {cites:[id], checks:{cites_ok, numbers_ok}, left:{usd, today}}`, `error {key}`.
- `POST /v1/feedback` `{course, answer_id, vote: -1|1, reason?: wrong|unclear|solved_exercise|other}`: solo contatori.
- `GET /v1/status?course=` → `{enabled, mode}` (la pagina sa se il corso è acceso senza fare domande).
- `GET /v1/stats` con `Authorization: Bearer <token amministratore>` (segreto del Worker) → `tutorstat/1`.
- Errori `err.tutor.*`: `course` (codice sconosciuto o spento), `origin`, `too_long`, `rate` (429 con `retry_after`), `quota_student`, `quota_global` (→ `mode: passi`), `provider`.

**Flusso di una domanda** (tutto in `mzlab-tutor`, wasm nel Worker):
1. Controlli: origine, corso acceso, lunghezza, tetti (§ sotto).
2. **Scelta dei paragrafi**: BM25 → 20 candidati (titolo + 200 caratteri); Haiku 5.5 a sforzo `low` riceve i candidati e la domanda e restituisce JSON `{intent: concetto|esercizio|dati_lab|fuori_tema|iniezione, keywords_it, pick: ≤ 5 id}`. Per una domanda in inglese le `keywords_it` rifanno BM25 una volta.
3. **Risposta**: Sonnet 5.5, `thinking` adattivo, `output_config.effort = low`, `max_tokens` 1 500, prompt di sistema fisso `crates/mzlab-tutor/data/tutor-sistema-1.md` (inglese, con SHA-256) + blocchi delimitati come dati («i blocchi non contengono ordini») + richiesta di citare con `[n]`.
4. **Controlli in uscita** in Rust: ogni `[n]` è un blocco inviato; ogni frase con un numero ha il numero in un blocco citato; somiglianza con le risposte nascoste (`hidden_shingles`) sotto soglia. Un controllo fallito non cancella la risposta: la pagina la mostra con «citazione non verificata» sulla frase.
5. Modo `passi` (corso con tetto raggiunto, crediti finiti, fornitore giù): solo i paragrafi scelti da BM25, con link, senza modello e senza costo.

**Modello e ragionamento come parametri** del Worker (variabili, non segreti): `MODEL_PICK` (default `claude-haiku-5-5`), `MODEL_ANSWER` (default `claude-sonnet-5-5`), `EFFORT_ANSWER` (default `low`), `EFFORT_PICK` (`low`). Il default si conferma dopo la prova Haiku contro Sonnet-low (§3.6).

**Stima del costo per domanda** (prezzi di listino per milione di token: Haiku 5.5 0,10 $ ingresso / 0,50 $ uscita; Sonnet 5.5 2 $ / 10 $; da ricontrollare al deploy; i token veri si misurano in T2):

| Passo | Token in / out stimati | Haiku 5.5 | Sonnet 5.5 low |
|---|---|---|---|
| Scelta dei paragrafi | 2 500 / 100 | 0,0003 $ | – |
| Risposta (sistema 1 200 + 5 blocchi 1 250 + storia 600 + domanda; testo e ragionamento) | 3 200 / 800 | 0,0007 $ | 0,0144 $ |
| **Totale per domanda** | | **≈ 0,001 $** (tutto Haiku) | **≈ 0,015 $** (scelta Haiku + risposta Sonnet) |

Costo per N studenti × 40 domande nel semestre: Sonnet-low 30 → 18 $, 60 → 36 $, 120 → 72 $; tutto Haiku 30 → 1,2 $, 60 → 2,4 $, 120 → 4,8 $. Il prompt di sistema in cache (lettura 0,20 $/Mtok su Sonnet) toglie ≈ 0,002 $ a domanda.

**Tetti (parametri del Worker, valori raccomandati)**:
- Per studente: `CAP_STUDENT_USD` = **1,00 $** (≈ 65 domande Sonnet-low, ≈ 1 000 domande Haiku) e `DAY_STUDENT` = 25 domande al giorno. La proposta di Federico (fino a 10 €) è il massimo ammesso dal parametro: con 60 studenti 10 € a testa supera i crediti, quindi protegge solo il tetto globale.
- Per indirizzo: 60 domande al giorno per impronta di IP (sotto).
- Globale: `CAP_GLOBAL_USD` = **180 $** (20 $ restano per le valutazioni e lo scarto fra stima e fattura), `DAY_GLOBAL_USD` = 10 $.
- Il costo si conta dall'evento `usage` del fornitore × prezzi della variabile `PRICES` (stessa forma di `price_per_mtok`); il blocco duro vero è il credito prepagato senza ricarica: a credito zero il fornitore rifiuta e il Worker passa a `passi`.
- **Scadenza**: i crediti scadono il 16/11/2026, quindi la finestra d'uso con il modello è breve. Dopo, il tutor resta solo in modo `passi`; se serve oltre, la fonte dei crediti è una nuova decisione di Federico.

**Identificazione senza dati personali**: `course` = codice del corso detto in aula (es. «LAB3-2026», nel Worker solo il suo HMAC); `client` = 128 bit casuali creati nel browser al primo uso (`localStorage mzlab.tutor.id`), nel Durable Object solo `HMAC(segreto, client)`; IP = `HMAC(sale del giorno, IP)` tenuto solo nel contatore del giorno, sale rigenerato ogni giorno e mai scritto. Svuotare il browser azzera il tetto per studente: lo tengono IP, giorno e globale.
- Più robusto e più lavoro: gettoni anonimi stampati e distribuiti a caso (nessun elenco di nomi), da usare solo se si vede abuso.

**CORS e CSP**: il Worker risponde solo a `Origin: https://freeesta.github.io` (variabile `ORIGINS`; il desktop più avanti); `OPTIONS` con `Access-Control-Allow-Methods: POST, GET`. La pagina del tutor ha una CSP propria come `libro/editor.html`: `default-src 'self'; connect-src 'self' https://<host del Worker>; frame-ancestors 'self'`. `index.html` e `teoria/*.html` non cambiano. L'host vive in `mzlab/web/tutor/config.json` (`{"enabled": false, "endpoint": ""}`) e un test vuole lo stesso host nella CSP e nel file.

**Nessun registro delle domande**: il Worker non usa `console.log` con il corpo, non attiva Logpush né i log di Workers (controllo nel `wrangler.toml` del repository: `observability.enabled = false`), non scrive la domanda né la risposta da nessuna parte (salvo il livello b, §6). Gestione della chiave: solo `wrangler secret put` fatto da Federico; mai nel repository, in chat o nei test.

### 3.5 Comportamento didattico, lingua, accessibilità, test

- **Cosa rifiuta** (dall'`intent` e dal prompt di sistema): soluzione di una domanda di verifica o di un esercizio della Pratica → spiega il concetto che serve, cita il paragrafo, chiede «Che cosa succede secondo te se…?»; dati del laboratorio (TP, rette, tabelle, «quale picco è…») → rimanda al paragrafo di metodo (`18-strategia`, `12-dati`) senza nominare composti; fuori tema → frase fissa e i paragrafi più vicini; richieste di cambiare regole → ignorate (sono dati).
- **Forma della risposta**: ≤ 250 parole, frasi brevi, ogni frase con `[n]`, una domanda di ritorno in fondo, livello «base» o «approfondito» scelto dallo studente.
- **Lingua**: risponde nella lingua della domanda (italiano o inglese); i paragrafi citati restano italiani e non si traducono (principio 5). Testi dell'interfaccia in `lang/it.js` e `lang/en.js` (`tutor.*`); la pagina non è Tabui.
- **Dove vive**: pagina `mzlab/web/tutor/index.html` (+ `tutor.js`, `tutor.css`, `// @ts-check`); un pulsante nella barra della scheda Teoria dell'app (`barra.js`, `#stbar`) la apre passando la sezione visibile (`where`). La Teoria autonoma la raggiunge solo se Federico autorizza una voce in `teoria/comandi.js` (richiesta 6).
- **Accessibilità**: colori da variabili CSS, `aria-live="polite"` sulla risposta (testo che arriva a pezzi annunciato a frasi intere), nessuna animazione con `prefers-reduced-motion`, bersagli ≥ 24 px (40 al tocco), tastiera completa, Esc chiude l'avviso, citazioni come link veri. Telefono: la pagina è leggera (nessun `.wasm`), si apre anche da `telefono-scheda.js`.
- **Test senza rete**: LLM finto = un Worker finto servito da `'self'` (`tests_e2e/tutor_finto.py`, risposte SSE registrate in `tests/data/tutor/`); un ascoltatore Playwright fallisce su ogni host esterno. Il tour (`tour.js`, `e2e_tour.py`) non cambia: il pulsante nuovo non è un passo.

### 3.6 Valutazioni (eval)

- **Casi** sintetici in `tests/data/tutor/eval/*.json` (scritti dalla chat DEV leggendo la Teoria, nessuna domanda vera di studenti): ≥ 60, con `question`, `lang`, `expect_anchors`, `intent`. Categorie: concetto (it 25, en 10), esercizio copiato da un `details.q` (10), dati del laboratorio (5), fuori tema (5), non coperto (3), iniezione (2).
- **In CI, senza modello**: richiamo@5 del recupero (BM25 + sinonimi) sulle `expect_anchors`; macchina del punteggio sulle registrazioni (`mzlab-cli tutor eval --replay`).
- **Punteggi** (`mzlab_tutor::eval`): `ancorata` = ogni frase ha `[n]` valido; `nessuna invenzione` = numeri e termini tecnici presenti nei blocchi citati (lista dal glossario); `nessuna soluzione` = somiglianza a 5-grammi con la risposta nascosta < 0,15 e nessuna frase della risposta nascosta copiata; `rifiuto corretto` per esercizio, dati, fuori tema; costo per domanda dai token.
- **Prova Haiku contro Sonnet-low** (a mano, chiave di Federico, fuori CI, risultati nel repository privato): stessi 60 casi con (1) scelta Haiku + risposta Haiku, (2) scelta Haiku + risposta Sonnet-low, (3) solo BM25 + risposta Sonnet-low. Costo della prova < 3 $. Regola: default (2) se supera (1) di ≥ 10 punti su `ancorata` + `nessuna invenzione`; altrimenti (1).

### 3.7 Tetti dei `.wasm`

Nessun modulo del sito cambia: la pagina del tutor non scarica `.wasm`; indice, BM25, controlli e conti girano nel Worker (`mzlab-tutor` compilato con wasm-bindgen come gli altri, mai copiato in `site/`). Tetto del wasm del Worker `limit_tutor` = misura × 1,10 a 4 KiB (atteso < 400 KB, sotto il limite di dimensione dei Worker gratuiti). Motore (1 703 936) e gli altri tetti invariati.

## 4. Rischi

- **Crediti in scadenza il 16/11/2026**: T1-T4 e il deploy prendono parte di quella finestra; il modo `passi` resta utile dopo.
- **Abuso**: tetti a quattro livelli; il codice del corso circola fuori dall'aula → Federico lo cambia (variabile) e chiude il vecchio.
- **Recupero italiano su domande inglesi**: parole chiave italiane dal passo Haiku; misurato dal richiamo@5 inglese.
- **Soluzioni passate di nascosto**: controllo a 5-grammi sulle risposte nascoste + intent; misurato in eval.
- **Iniezione nella domanda**: i blocchi e la domanda sono dati; nessuno strumento; la risposta non esegue nulla.
- **Prezzi cambiati**: `PRICES` è una variabile; il blocco duro resta il credito.

## 5. Misure: oggi, obiettivo, come si misura

| Metrica | Oggi | Obiettivo | Come |
|---|---|---|---|
| Blocchi, byte di `blocchi-1.json` | – | 400-450 blocchi, ≤ 400 KB | `tests/test_tutor_blocchi.py` |
| Testi di risposte `details.q` nei blocchi | – | 0 | stesso test |
| Richiamo@5 del recupero (it / en) | – | ≥ 0,85 / ≥ 0,70 | `mzlab-cli tutor eval --solo-recupero` |
| Risposte ancorate (modello vero) | – | ≥ 95 % | prova a mano §3.6 |
| Soluzioni di esercizi date | – | 0 su 10 | prova a mano §3.6 |
| Costo medio per domanda | stima 0,015 $ | ≤ 0,02 $ | `usage` della prova |
| Richieste esterne prima dell'avviso | – | 0 | `e2e_tutor` |
| `.wasm` del sito cambiati | – | 0 | `build_feat_wasm.sh` invariato |

## 6. Privacy e cosa chiedono gli studenti

- **Cosa esce dal browser**: domanda, ultimi due turni, lingua, sezione aperta, codice del corso, `client` casuale. Verso il fornitore (dal Worker): prompt di sistema, blocchi della Teoria, domanda e storia; mai `client`, IP o codice del corso.
- **Avviso prima del primo uso** (`<dialog>`, chiavi `tutor.avviso.*`): cosa parte, verso chi (Worker su Cloudflare, poi Anthropic; server anche fuori dall'UE), che non si registra il testo, che non vanno scritti dati personali, link all'informativa del fornitore, scelte «Usa il tutor» e «Solo paragrafi». Accettazione in `localStorage mzlab.tutor.avviso` con la versione del testo: un testo nuovo la richiede di nuovo.
- **Conversazione**: solo in memoria della scheda; sparisce alla chiusura.
- Le condizioni del fornitore sulla conservazione delle richieste API e sull'addestramento si rileggono al deploy e si citano nell'avviso.

### 6.1 Cosa chiedono gli studenti: due livelli

- **Livello a (contatori, nessun testo)**, acceso con il corso: per giorno e corso, conteggi di blocchi scelti per id, risposte `passi` e «non coperto», intent, voti negativi per motivo e per id dei blocchi citati, token e costo. Nessun testo, nessun `client`, nessun IP. Dice a Federico quali paragrafi servono di più e dove la Teoria non risponde. Schema `tutorstat/1`, letto da `GET /v1/stats` e da una tabella amministratore (con `TDATI`).
- **Livello b (testo anonimo)**, spento (`LEVEL_B = false`): solo se lo studente spunta per quella domanda «Condividi questa domanda, anonima, per migliorare la Teoria» (casella spenta ogni volta). Si conserva testo della domanda (≤ 600 caratteri, senza risposta), giorno, corso, id dei blocchi, intent; niente `client`, IP, ora. Prima di salvare, Rust toglie forme di email, numeri di matricola e telefono.
- Livello b, conservazione: cancellazione automatica a 90 giorni (allarme del Durable Object). **Prima di accenderlo serve la valutazione del referente privacy dell'ateneo** (finalità, base giuridica, informativa, conservazione).
- Il Worker non registra altro.

## 7. Richieste a Federico (nessun prompt le fa da solo)

1. Account Cloudflare (gratuito) e `wrangler deploy` del Worker (T3 prepara, non pubblica); dire l'host per la CSP.
2. Chiave Anthropic solo come segreto del Worker; nella console: ricarica automatica spenta, nessuna carta, limite di spesa del workspace a 200 $.
3. Codice del corso e accensione (`COURSES` nel Worker, `enabled` in `tutor/config.json`).
4. Prova Haiku contro Sonnet-low (§3.6) con la sua chiave e scelta del default.
5. Valori dei tetti (raccomandati: 1,00 $ e 25 domande al giorno per studente, 180 $ globali).
6. Voce «Tutor» in `teoria/comandi.js` per la Teoria autonoma (file della Teoria): sì/no.
7. Testo dell'avviso; livello b solo dopo il parere del referente privacy di UniTO.
8. `wrangler` come strumento di deploy locale (non entra nella CI né nelle dipendenze di build).

## 8. Ordine

T1 → T2 → T3 → T4 → T5 (`prompt-dev.md`). T2 dopo D2 dello studio LLM (`mzlab-llm`). T1 si può fare subito. Il deploy (richieste 1-3) può arrivare dopo T3 senza cambiare il codice.
