# Architettura dell'assistente dati

## 1. Forma generale

```
file utente ──► Rust: lettori, mzidx, features.estimate, metodo/1, campioni/2
                     │
                     ▼
              ispezione/1  (deterministico, nessun testo libero non filtrato)
                     │
   ┌─────────────────┼──────────────────────────────────────────┐
   │  orchestratore (Sonnet 5.5, effort medium)                 │
   │   ├─ router (Haiku 5.5)          : a chi va il messaggio   │
   │   ├─ lettore dei metadati        : ispezione/1 → lettura   │
   │   ├─ controllo cromatografia/QC  : ispezione/1 + QC        │
   │   ├─ proposta dei parametri      : proposta-param/1        │
   │   ├─ interprete feature table    : tabcheck/1 → lettura/1  │
   │   └─ disegnatore di grafici      : → grafico/1             │
   └─────────────────┬──────────────────────────────────────────┘
                     ▼
     Rust valida: flow::check, limiti step/1, chart::validate, numeri citati
                     ▼
     pagina / CLI esegue: piano mzflow/2, disegno con TDATI, tabelle
```

Principio: Rust misura e controlla, il modello interpreta, domanda e spiega. Il modello non vede mai byte dei file e non esegue mai codice.

## 2. Dove gira il client

| Superficie | Trasporto | Chiave | Stato | Vie |
|---|---|---|---|---|
| CLI `mzlab-cli assist` | `curl` con intestazioni su stdin (come `tutor eval`), mai la chiave negli argomenti | file `--key-file` (0600) o stdin | da costruire | nessuna |
| App desktop | rotte `llm/send`, `llm/poll` (`ureq`, esistono) | `llm-chiavi.json` 0600, la legge solo Rust | esiste il trasporto | nessuna |
| Sito | iframe `llm/trasporto.html` (esiste) | in memoria dell'iframe o IndexedDB cifrata | oggi solo `localhost` e `replay` | V1 |

- Per il test con la chiave dedicata si usa la CLI: nessun cambio di CSP, nessun server, ripetibile da una sessione cloud o dal Mac di Federico.
- La chiave arriva solo con il flusso di richiesta della credenziale dell'ambiente (variabile o file fuori dal repository); mai in chat, in un commit, in un argomento di processo, in `llmlog/1`, in `prov/1`. `log::has_secret` e `tests/test_llm_chiavi.py` restano le guardie.
- Il sito (V1a) usa la chiave dell'utente: il tetto per persona coincide con la chiave. Una chiave condivisa nel sito non è mai ammessa (si leggerebbe dalla pagina); per una chiave condivisa serve V2a.
- Profilo `corso`: l'assistente non esiste (nessun byte scaricato), come oggi per `llm/`. L'assistente vive dove vive il Flow.

## 3. Cosa si manda al modello

Solo strutture costruite da Rust, con tetti per sezione e numeri a 4 cifre significative (regola di `contesto/1`).

**Sì**
- `ispezione/1` per file: alias (`f1`..) o nome ripulito, formato, produttore e modello se il file li dice, classe di analizzatore, risoluzione stimata, polarità, livelli MS, tipi di scansione e conteggi, tempo di ciclo, finestra di isolamento, transizioni MRM (solo numero e intervallo m/z), intervallo RT, TIC e BPC ridotti a ≤ 200 punti, gradiente da `metodo/1` (tempi e %B), flusso, colonna se scritta, tempo morto stimato, rumore, errore di massa, larghezza dei picchi con la prova riassunta, ruolo proposto da `campioni/2` e ordine di iniezione.
- `proposta-param/1` (`03-parametri.md`) e `tabcheck/1` (`04-feature-table.md`).
- Feature table ridotta: conteggi, distribuzioni per colonna, ≤ 50 righe scelte da Rust (le più intense, quelle segnalate dai controlli, quelle chieste dall'utente per id).
- Piano corrente `mzflow/2` canonico e problemi di `flow::check`; descrittori `step/1` su richiesta (`flow_describe_step`).
- Testo dell'utente.

**No**
- Byte dei file, spettri interi, profili, punti dei cromatogrammi oltre la riduzione, `.mzidx`.
- Percorsi del disco, nomi di cartelle, nome dell'utente o del computer (`metodo/1` li porta: `operatore`, campi del driver); `grezzo` del Log; `campi` non mappati; seriale dello strumento.
- Con «Hide file names» acceso (predefinito nel test) i nomi diventano alias; i ruoli restano.
- Nulla della Teoria con le soluzioni; nessun dato del repository privato nei casi pubblici.

Regola sui nomi: un nome di file o di campione è un dato privato finché l'utente non dice il contrario; parte solo come alias o, con il consenso della classe `names`, tagliato a 64 caratteri, senza cifre lunghe (≥ 6, possibili date o matricole) e senza caratteri di controllo.

## 4. Strumenti (tool use)

- Gli strumenti sono quelli di `strumenti/1` (15 meta-strumenti generati dal registro `flow::steps()`): il modello non ha uno strumento per passo, ha `flow_add_step`, `flow_set_param`, `flow_propose_plan` con `enum` degli `id@version` e dei nomi dei parametri. Un passo nuovo entra nel catalogo da solo.
- Strumenti nuovi, tutti di classe `read` o `propose` (nessun `run` nuovo):
  - `data_inspect {files?: [alias]}` → `ispezione/1` (sostituisce la parte file di `data_summary`).
  - `params_propose {steps: [id@v]}` → `proposta-param/1` dalle regole Rust.
  - `table_check {checks?: [nome]}` → `tabcheck/1`.
  - `table_rows {ids?: [id], where?: filtro/1, limit ≤ 50}` → righe ridotte.
  - `chart_make {spec: grafico/1}` (esiste in `strumenti/1`; A5 lo estende con le aggiunte del §6) → valida e disegna; risponde con i problemi o `ok` e l'id del grafico.
  - `ask_user {questions: [{id, text, options?}]}` → domande chiuse mostrate come carte; la risposta torna come `tool_result`.
- `strict: true` su ogni strumento, `additionalProperties: false`, `tool_choice` `auto` (il forzato dà 400 su Sonnet 5.5). Se il modello doveva chiamare uno strumento e non l'ha fatto, il ciclo Rust lo dice in un messaggio di sistema a metà conversazione (non cambia il prefisso in cache).
- `policy::decide` resta l'unica decisione: livelli L0/L1/L2, 20 chiamate, conferma dal 4° `run`. Nel test si usa L1 (ogni modifica chiede conferma, simulata dallo script).

## 5. Il piano come JSON validato

- Il modello propone con `flow_propose_plan` un `mzflow/2` intero o con `flow_add_step`/`flow_set_param` modifiche singole. `plan::problems` (`flow::check` sul registro reale) risponde; al più 3 tentativi (`MAX_PLAN_TRIES`), poi l'assistente spiega cosa resta.
- Ogni valore di parametro deve stare nei limiti del descrittore; un valore che coincide con la proposta Rust porta `source: rule:<id>`, uno diverso porta `source: model` e la motivazione, che la pagina mostra.
- Modelli di partenza: l'orchestratore non inventa catene; sceglie fra modelli Rust (`F.templatePlan` e i piani d'esempio di `flow-pacchetto-tabui.md`) per tipo di acquisizione (`03-parametri.md` §4) e poi li adatta.

## 6. Grafici come specifica

- `grafico/1` dello studio LLM (§7 di `docs/studi/llm/README.md`), con tre aggiunte: `mark` anche `box`, `hist`, `heat`; `data.ref` può puntare a una vista nominata di Rust (`tic:f1`, `features:area`, `qc:cv`, `drift:rt`); `facet` facoltativo (≤ 12 pannelli).
- Il modello non manda mai righe di dati: manda il riferimento e i campi. `chart::validate` controlla campi esistenti, tipi, tetti, assenza di colori e stili; i colori sono Okabe-Ito dalle variabili CSS; ogni grafico nasce con la sua tabella (`TDATI`).
- Grafici predefiniti senza modello (degradazione): TIC sovrapposti, distribuzione delle larghezze, deriva RT dei QC, CV dei QC, rapporto campione/bianco.

## 7. Subagenti e divisione del contesto

Ogni subagente è una chiamata singola (o due con un solo ritentativo) con il suo prompt, la sua fetta di dati e un'uscita a schema. L'orchestratore vede solo il riassunto restituito (≤ 1 500 token), non l'ingresso del subagente.

| Subagente | Modello, effort | Ingresso | Uscita | Quando |
|---|---|---|---|---|
| router | Haiku 5.5, low | ultimo messaggio + stato (fase, file aperti, piano sì/no) | `{route, need_data[], language}` | ogni messaggio dell'utente |
| lettore dei metadati | Sonnet 5.5, medium | `ispezione/1` di tutti i file | `lettura/1` tipo `acquisition` + domande | apertura dei file |
| controllo cromatografia e QC | Sonnet 5.5, medium | TIC/BPC ridotti, gradiente, ruoli, ordine, segnali QC | `lettura/1` tipo `chromatography` | dopo il lettore, se ci sono ≥ 2 file |
| proposta dei parametri | Sonnet 5.5, medium | `proposta-param/1` + risposte dell'utente + piano | modifiche `flow_set_param` motivate | prima di una corsa |
| interprete feature table | Sonnet 5.5, medium | `tabcheck/1` + righe ridotte | `lettura/1` tipo `table` | dopo una corsa |
| disegnatore di grafici | Haiku 5.5, low | richiesta + elenco delle viste e dei campi | `grafico/1` | a richiesta |

- Nessun subagente chiama strumenti di classe `edit` o `run`: solo l'orchestratore, sotto la politica.
- Ordine di fiducia: Rust > risposte dell'utente > lettura del subagente > testo dell'orchestratore. Un subagente che contraddice un numero di Rust è un errore misurato dalla valutazione.
- Haiku basta dove l'uscita è una scelta fra opzioni chiuse (rotta, tipo di grafico): lo dice la valutazione (`06-valutazione.md`, confronto Haiku/Sonnet sugli stessi casi); se Haiku sbaglia più del 5 % in più, quel ruolo passa a Sonnet a effort `low`.

## 8. Formato delle letture (`lettura/1`)

```json
{"schema":"lettura/1","kind":"acquisition|chromatography|table","findings":[
  {"id":"F1","topic":"polarity","statement":"f1–f7 are ESI+ full scan at unit resolution","evidence":["ispezione.f1.polarity","ispezione.f1.resolution"],
   "confidence":"high|medium|low","action":{"type":"none|ask|set_param|add_step|split_series|chart","ref":"Q1"}}],
 "questions":[{"id":"Q1","text":"Which file is the procedural blank?","options":["f1","f8","none"]}],
 "numbers":["0.3","14.3"]}
```
- `evidence` cita solo chiavi presenti nell'ingresso: Rust controlla che esistano; `numbers` elenca i numeri usati e ognuno deve comparire nell'ingresso (controllo alla `checks::numbers`).
- Le frasi restano candidati: mai «è il composto X», sempre «m/z 305.3 a 14.7 min, candidato del prodotto …» e solo in HR con le prove; in LR nessun verdetto sui prodotti (principi 1 e 4).

## 9. Flusso di una sessione

1. L'utente apre i file; Rust costruisce `ispezione/1` (secondi, nessun costo).
2. Lettore dei metadati e controllo cromatografia: una lettura con 2–5 domande chiuse (matrice, bianchi, QC, composti attesi, obiettivo: screening, TP, quantificazione).
3. Risposte → orchestratore sceglie il modello di piano, chiama `params_propose`, propone il piano; `flow::check`; l'utente applica (L1).
4. Corsa (esecutore esistente); `table_check`; interprete; grafici a richiesta.
5. Ogni turno aggiorna il contatore di spesa; all'80 % del tetto l'assistente avvisa, al 100 % si ferma e lascia le parti Rust attive.
