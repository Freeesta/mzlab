# Come si parlano i moduli: porte, compatibilità, ordine, parallelismo, preset

Parte di `docs/studi/WORKFLOW_MODULARI.md` (§4b). È il cuore del contratto: la futura sezione «Flow» (una tela con moduli trascinabili e collegati) è solo un modo di disegnare quello che sta qui. La grafica non è in questo studio. Segni: **V** verificato nel codice o misurato, **I** ipotesi. Lo schema è `contratto.schema.json` (campi citati tra apici inversi).

## 1. Cosa viaggia su un filo
- **Un filo porta un riferimento, mai i byte.** Il dato vive nel deposito (OPFS nel browser, una cartella nell'app desktop) e il filo porta il suo **descrittore**: `type`, `sha256`, `bytes`, versione del formato e poche **faccette** lette dal dato (risoluzione `hr`/`lr`, polarità, livelli MS presenti, numero di campioni). Motivo: un `.mzidx` o un mzML pesa da megabyte a gigabyte; oggi mzML e `.mzidx` stanno già in OPFS e i moduli li leggono a fette (`ReadAt`, `Index<R>`; V).
- **Il dato è immutabile e indirizzato dal contenuto**: stesso SHA-256 = stesso dato. Un passo non riscrive mai il suo ingresso (correzione 7 di `PROVE_SU_CARTA.md`). Due fili che portano lo stesso dato non lo copiano.
- **Versione del formato nel descrittore**: un dato di formato più nuovo del programma si apre in sola lettura con `error.newerFormat` (regola già in uso per `.mzidx`, `.mzfeat`, `.mzfeatf`: `docs/agenti/formato-nuovo.md`, V).

## 2. Regola di compatibilità (quando un'uscita si può collegare a un ingresso)
Un filo da `A.out` a `B.in` è valido se valgono tutte:
1. **Tipo**: `out.type` = `in.type`, oppure `out.type` è nella lista `in.accepts` (es. un passo che legge sia `mzidx` sia `table`).
2. **Cardinalità**: `many → many` sì (stesso ordine); `one → many` sì (lista di uno); più fili verso una porta `many` si **raccolgono** nell'ordine di `ingressi` del file, mai nell'ordine in cui l'utente ha tirato i fili (il risultato non deve dipendere dal disegno); `many → one` solo se il passo ha `per_item: true`, cioè gira una volta per elemento (§3).
3. **Faccette**: `in.requires` (es. `{"resolution": ["hr"]}` per `tp.link` e `annot.*`: Principio 1, nel mondo LR niente TP né formule come verità; `{"ms_levels": [2]}` per `features.ms2_link`). Se la faccetta è già nota (dopo l'importazione) il controllo è immediato; se no, il passo lo rifà quando parte e risponde con una chiave d'errore.
4. **Grafo aciclico**: nessun filo chiude un ciclo.
Ogni violazione è una voce `{instance, port, error_key, params}` con chiavi del tipo `err.flow.type`, `err.flow.cardinality`, `err.flow.facet`, `err.flow.cycle`: la tela colora il filo e mostra il testo dal catalogo, nessun testo scritto a mano.

**Il passo mancante si propone, non si inserisce da solo.** I passi di conversione dichiarano `converts: {from, to}`. Se l'utente collega `raw` a una porta `mzidx`, il programma cerca il cammino più breve nel grafo delle conversioni registrate (ricerca in ampiezza; a parità vince quello con `cost` minore, poi l'ordine del registro) e propone «Insert: Build index» con un clic.
Esempio vero: `raw → mzidx` esiste diretto (`upload::indicizza_lettore`) e passando per l'mzML (`lettori_mzml::scrivi` + `build_mzml_spill`), e le due vie danno le stesse feature (V, §6.1 del principale): il diretto è più corto e vince. Entrano nel grafo SOLO le conversioni registrate: il secondo scrittore di mzML (`raw::to_mzml`, aree diverse a valle, V) non c'è.

## 3. Ordine di esecuzione e parallelismo (ricavati, mai scritti dall'utente)
- **Ordine**: ordinamento topologico del grafo (algoritmo di Kahn); a parità, l'ordine di `steps` nel file. Lo stesso file dà sempre lo stesso piano.
- **Parallelo fra rami**: un'istanza parte quando tutti i suoi antenati sono finiti; due istanze senza dipendenza reciproca girano insieme (es. i due rami `tp.generate` di `PROVE_SU_CARTA.md` D).
- **Parallelo per file**: un passo `per_item: true` su una porta `many` gira una volta per elemento, in parallelo; le uscite si raccolgono per INDICE, non per ordine di arrivo. Oggi il rilevamento è già per file (`run::detect` per file, poi `read_files` e `align`: V in `mzlab-cli`).
- **Dove**: nel browser un worker per istanza attiva, che carica il modulo `.wasm` del passo (`runs_in.wasm`; oggi ci sono già `rust-worker.js`, `feat-worker.js`, `tpmine-worker.js`: V); nell'app desktop fili nativi (`rayon` è già una dipendenza di `mzlab-desktop`: V).
- **Memoria come unica risorsa condivisa**: nessun conflitto di scrittura (dati immutabili), quindi l'unico limite è la memoria. Il pianificatore ammette un passo solo se la somma dei `memory_hint_bytes` dei passi attivi sta nel bilancio del dispositivo (il contratto di memoria di `mzlab-core::budget`, `docs/agenti/memoria.md`: V che esiste; uso nel pianificatore: I). La stima viene dalle misure di `prov/1` (`heap_peak_bytes`) dei giri precedenti.
- **Determinismo con il parallelo**: un passo non legge l'orologio né il caso senza seme; il risultato non dipende dall'ordine di arrivo. Prova: lo stesso piano con 1 worker e con N worker dà gli stessi `generated.sha256` (da aggiungere alla prova 3 del §7).
- **Errori e annullamento**: un passo che fallisce ferma solo i suoi discendenti; i rami fratelli finiscono e restano in cache. Annullare un passo ferma lui e i discendenti.
- **Cache come `make`**: chiave = (SHA-256 degli ingressi, `id@versione`, `params_sha256`). Cambiare un parametro di X invalida X e i discendenti, nient'altro; un giro ripetuto senza cambi non ricalcola nulla e `prov/1` scrive `cached: true`.

## 4. Algoritmi, preset e confronto
- **Algoritmi** (`algorithms[]`, `default_algorithm`): il passo (es. `features.detect`) è la «casella» della tela; le **alternative** sono algoritmi dentro lo stesso passo, con le stesse porte (stessi nomi e tipi): si scambiano senza ricollegare fili. Ogni voce dichiara `id`, `version`, `title`, `doc`, `source` (DOI dell'articolo da cui si riscrive: mai codice copiato, §9 del principale), `applicability`, `params` propri e `status` (`available` o `planned`, che la tela mostra in grigio).
  I `params` del passo sono quelli comuni a tutti gli algoritmi. Esempi veri nei file `step.features.detect.json` (`masstrace` available; `centwave`, `threshold` planned) e `step.spectra.match.json` (`cosine`, `modified_cosine`, `spectral_entropy`, tutti in `mzlab-lib/src/scores.rs`, V).
- **Applicabilità e suggerimento**: `applicability` usa le faccette dell'ingresso (`resolution`, `polarity`, `ms_levels`, `spectrum_mode`) più ciò che il metodo chiede (`peak_fwhm_s`, `min_files`, `needs`: `internal_standards`, `ms2`, `blank`, `reference_sample`, `library`) e una `note`. La tela mette in testa gli algoritmi che tornano; gli altri restano selezionabili con il motivo (es. «needs internal standards»), il controllo del piano li rifiuta solo se manca un ingresso obbligatorio.
- **Predefinito**: `default_algorithm` è il migliore noto per il caso tipico e si giustifica con numeri del banco (`tools/bench/`, prova 1 del §7), non con un'opinione.
- **Preset** (`presets`, `default_preset`): insiemi di parametri con nome, ognuno per un `algorithm` (assente = il predefinito). Un preset può avere `when` sulle faccette dell'ingresso (es. `hr`, `lr`): la scelta «Auto» prende il primo che corrisponde. Esiste già nel codice: `run::recipe_for` sceglie la ricetta ad alta risoluzione o quella del profilo a risoluzione unitaria dal primo file, e la CLI ha `--preset auto|pos|neg` (V).
- **Nel file**: un'istanza scrive `step`, eventualmente `algorithm` e `preset`, e in `params` solo le differenze. La forma canonica (regola 1 del §4 del principale) risolve algoritmo + preset + differenze + stime in valori completi; `prov/1` scrive l'algoritmo (`id@version`), il preset e i valori risolti; la chiave della cache comprende l'algoritmo.
- **Scambio di algoritmo**: i parametri comuni restano; quelli propri prendono il preset del nuovo algoritmo; la tela mostra la differenza prima di confermare.
- **Confronto** (`meta.compare@1`, `step.meta.compare.json`): un meta-passo che esegue il passo nominato in `params.step` con 2 o più `variants` (`algorithm`, `preset`, `params`, `label`) sugli stessi ingressi. Le porte d'ingresso sono quelle del passo avvolto (`ports_from: wrapped`); le uscite sono una per variante (`cmp.A.peaks`, `cmp.B.peaks`) e un `comparison`. Varianti uguali sono un errore (`err.flow.compare.sameVariant`). Le varianti girano in parallelo con le regole del §3 (la memoria si somma).
- **Metriche del confronto**, per tipo d'uscita: `peaks`/`table`: abbinate entro `mz_tolerance_ppm` (o `mz_tolerance_da` a risoluzione unitaria) e `rt_tolerance_min`, solo in A, solo in B, mediana di Δm/z in ppm e di ΔRT, mediana di |log2| del rapporto delle aree, Spearman delle aree abbinate; `rtmap`: scarto di RT per ancora; `annot` (punteggi MS2): correlazione di rango (Kendall), accordo del primo candidato, sovrapposizione dei primi k. Ogni metrica porta l'elenco delle righe che la spiegano, così la tela mostra le differenze, non solo i numeri.
- **Usi**: scegliere il predefinito su prove; validare una nuova implementazione contro la vecchia (stessi ingressi, stesso hash di partenza); metodologia e didattica (lo studente vede cosa cambia con il metodo). Nel mondo LR il confronto mostra picchi diversi, mai TP o rette (principio 1).

## 5. Il dialogo tra tela e motore (messaggi)
Solo JSON piccoli; i dati restano nel deposito.
| Messaggio | Da → a | Contenuto |
|---|---|---|
| `plan.check` | tela → controllo (Rust) | il `mzflow/2`; risposta: problemi `{instance, port, error_key, params}` e proposte di conversione |
| `plan.run` | tela → pianificatore | il piano controllato; il pianificatore manda le istanze pronte ai worker |
| `step.started` / `step.progress` | worker → tela | `{instance, done, total}`, la stessa forma della barra unica di oggi (`progresso.js`, `PROGRESSO`: V) |
| `step.done` | worker → tela | `{instance, generated: [descrittori], ms, heap_peak_bytes, cached}` |
| `step.failed` | worker → tela | `{instance, error, error_key, params}` (forma dell'API: V) |
| `plan.done` | pianificatore → tela | il `prov/1` completo |
La logica del controllo e della canonica sta in Rust (regola «Tutto in Rust»); il pianificatore nel browser smista soltanto, come fa già `feat-worker.js` con i passi della Pipeline (V).

## 6. Esempio: due rami, un preset, una conversione proposta
```json
{"format":"mzflow/2","program":{"name":"mzLab","version":"x"},
 "inputs":[{"name":"t0.raw","sha256":"…","type":"raw"},{"name":"t60.raw","sha256":"…","type":"raw"},{"name":"samples.json","sha256":"…","type":"samples"}],
 "steps":[
  {"id":"idx","step":"index.mzidx@1","in":{"raw":["file:t0.raw","file:t60.raw"]},"label":"proposed: Build index"},
  {"id":"det","step":"features.detect@1","algorithm":"masstrace","preset":"hr","in":{"index":"idx.index"},"params":{"min_height":100000}},
  {"id":"aln","step":"features.align@1","in":{"peaks":"det.peaks","index":"idx.index","samples":"file:samples.json"}},
  {"id":"ms2","step":"features.ms2_link@1","in":{"table":"aln.table","index":"idx.index"}},
  {"id":"tp","step":"tp.link@1","in":{"table":"aln.table","spectra":"ms2.spectra"}},
  {"id":"csv","step":"export.table_csv@1","in":{"table":"aln.table"}}]}
```
Piano ricavato: `idx` (per file, 2 in parallelo) → `det` (per file, 2 in parallelo) → `aln` → poi `ms2` e `csv` insieme → `tp`.

## 7. Cosa resta da provare
- Il controllo del piano in Rust e la ricerca delle conversioni: costo in byte del modulo (I) e tempo su piani di 50 istanze (I).
- Il pianificatore con N worker: picco di memoria reale nel browser e prova «1 worker = N worker» (I).
- La raccolta per indice nel rilevamento parallelo della Pipeline di oggi: da verificare che l'ordine delle uscite non dipenda dall'arrivo (I).
