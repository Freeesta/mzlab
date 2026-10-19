# Studio «workflow modulari»: il contratto dei passi e il formato dei workflow di mzLab

Obiettivo (Federico, 19/10/2026): mzLab legge i grezzi di ogni produttore, li converte in mzML e permette di COMPORRE workflow (pretrattamento → picchi → cromatogrammi → feature → annotazione/TP), riuniti in un'interfaccia, sul web o scaricato. Questo studio sceglie il contratto di un passo e il formato del workflow su prove (file veri di terzi, il nostro codice, un prototipo misurato), non su ideali. Base: `main` a `f072fcd`.

File dello studio:
- questo: verdetto, fonti, standard, contratto, prototipo, prove esterne, piano, vincoli, cosa non fare;
- `docs/studi/workflow/MATRICE.md`: passo → chi lo ha (mzmine, OpenMS, XCMS, patRoon, mzLab) → parametri; catalogo e buchi;
- `docs/studi/workflow/PROVE_SU_CARTA.md`: quattro workflow veri di terzi riscritti nel formato, cosa non si esprime e le correzioni nate;
- `docs/studi/workflow/contratto.schema.json`: la bozza (JSON Schema 2020-12) di `passo/1`, `mzflow/2` e `prov/1`;
- `docs/studi/workflow/esempio.mzflow.json`: un workflow grezzo → mzML → feature; `passo-features.detect.json`: il descrittore del rilevamento con i campi e i predefiniti veri di `trace::Params` (entrambi validano la bozza, V);
- `docs/studi/workflow/COMUNICAZIONE.md`: come si parlano i moduli della futura tela «Flow» (porte, compatibilità, passo mancante proposto, ordine e parallelismo ricavati, preset e alternative; §4b);
- `docs/studi/workflow/PUBBLICAZIONE.md`: quali crate pubblicare e in che ordine, licenze, cosa serve (§12, solo analisi);
- `crates/mzlab-cli/examples/flusso.rs`: il prototipo (§6).

**Segni**: **V** = verificato (letto in un file vero o nel codice, o misurato qui); **I** = ipotesi (non letto o non provato).

## 0. Verdetto
1. **Non serve un formato nuovo da zero**: mzLab ha già `mzflow/1` (`mzlab/web/workflow.js`: SHA-256 dei file, versione, commit, parametri, operazioni in ordine; V) e lo schema `feature_pipeline/1` (`docs/schemi/`, ricetta, prove delle stime, hash della tabella; V). Il formato proposto è **`mzflow/2`** = `mzflow/1` + un campo `passi` (il PIANO dichiarativo, un grafo), accanto a `operazioni` (il REGISTRO, invariato).
2. **Il pezzo che manca è il descrittore del passo** (`passo/1`): oggi i parametri sono strutture Rust (`Recipe`, `trace::Params`, V) e il pannello Pipeline li disegna a mano. Con `passo/1` (porte tipizzate, parametri in JSON Schema, versione legata al risultato, chiavi d'errore, modulo `.wasm`) il modulo si disegna da sé.
3. **Provenienza = sottoinsieme di PROV-JSON** (`prov/1`): entità con SHA-256, attività con parametri canonici, tempo e memoria. Il prototipo lo scrive già (V).
4. **Determinismo provato sul tratto grezzo → mzML → feature**: stessi byte su due giri, nativo e WASM (V, §6). Due difetti trovati: due scrittori di mzML con risultati diversi a valle, e l'impronta della ricetta calcolata sul testo `Debug` di Rust (§6.3).
5. **Prima catena da chiudere**: grezzo → mzML → indice → feature → tabella → TP del timololo (§8), con oracoli esterni (mzmine, OpenMS, ThermoRawFileParser) già cablati in `tools/bench/` (V).

## 1. Materiale grezzo letto
- **mzmine 4.0.2**: batch FBMN vero, `https://raw.githubusercontent.com/lfnothias/FBMN_annotation_fusion_visualization_Cytoscape/main/results_download/mzmine/mzmine_microbial_workshop.mzbatch.xml` (935 righe; V). Radice `<batch mzmine_version>`, passi piatti `<batchstep method parameter_version>`, parametri per etichetta umana, ingresso implicito `BATCH_LAST_FEATURELISTS`; nessun hash, nessun errore, nessun grafo (V).
- **OpenMS**: `.ini` di FeatureFinderMetabo 2.8.0, `https://raw.githubusercontent.com/PayamEmami/nf-core-metabolinden/5a0b165eff54b1f62e366545365ff61ee65c9d07/assets/parameters/FeatureFinderMetabo.ini` (V). `<PARAMETERS version>` + `<NODE>`/`<ITEM name value type restrictions advanced>`; un `.ini` per strumento, la catena sta fuori (Nextflow, KNIME). Nessun `.ini` letto per MapAligner e FeatureLinker (I).
- **XCMS**: vignetta `https://raw.githubusercontent.com/sneumann/xcms/devel/vignettes/xcms.Rmd` (V): `findChromPeaks(CentWaveParam)` → `refineChromPeaks` → `adjustRtime(ObiwarpParam)` → `groupChromPeaks(PeakDensityParam)` → `fillChromPeaks`. Script, nessun file di workflow; i parametri viaggiano nell'oggetto (I).
- **patRoon 3.1.0** (GPL-3): handbook `https://rickhelmus.github.io/patRoon/handbook_bd/` pagine `features`, `filtering`, `genTPs`, `TPsExamples` (V; solo nomi di funzioni e argomenti, nessun codice copiato). Studio dedicato: `docs/studi/patRoon_TP.md` (PR #1092, aperta).
- **PSI**: schema `mzML1.1.1.xsd` (58 KB) e esempio `1min.mzML` dal repository HUPO-PSI (V); `psi-ms.obo` `data-version 4.2.6` del 14/10/2026 (V); schema di mzQC 1.0.0 (V); mzTab-M 2.1-dev: pagine di dettaglio non lette (I).
- **Nostro codice** (V): trait `Lettore` e `apri` (`crates/mzlab-core/src/lettori.rs`, `crates/mzlab-lettori/src/lib.rs`), `lettori_mzml::scrivi`, `mzidx::build_mzml_spill`, `upload::indicizza_lettore`, `mzlab_feat_wasm::run::{recipe_for, detect, read_files, align}`, `pipeline::analyse`, `recipe::Recipe` (testo + impronta FNV), `mzlab-cli` (`docs/agenti/cli.md`), `TP_Mine/files/feat-worker.js` (orchestra già i passi in JS), le esportazioni di 10 moduli `.wasm` (da 2 a 32 funzioni ciascuno).
- **Dati con verità**: corpus e `verita/1` di Rosina (`docs/studi/ROSINA.md` §F; nel privato), serie D1 (MetaboLights, MAF) e D2 (sintetica) del banco (`docs/agenti/mzfinder-benchmark.md`, V), verità del timololo `timolol_truth.json` (privato). Per il prototipo: `small.RAW` (LTQ, 48 spettri, 1,5 MB) dei test di ThermoRawFileParser (Apache-2.0), usato solo in locale e mai salvato nel repository.

## 2. Standard prima di inventare
| Standard | Cosa riusiamo | Cosa no, e perché |
|---|---|---|
| mzML 1.1 (PSI) | il tipo `mzml` così com'è; `dataProcessingList/processingMethod` con un termine PSI-MS per OGNI passo che cambia gli spettri | come formato di workflow o di provenienza: solo una lista lineare per file, senza ingressi né parametri tipizzati (V) |
| PSI-MS (`psi-ms.obo` 4.2.6) | ~10 accessioni nel campo `cv` del passo (MS:1000544 conversione, MS:1000035 peak picking, MS:1000593 riduzione della linea di base, MS:1000594 rimozione dei punti deboli, MS:1000745 allineamento RT; V) | spedire l'OBO (1,9 MB) nel sito |
| mzQC 1.0 | il modello di `inputFiles` (nome, formato, proprietà con checksum) e `analysisSoftware`; un export `qc` futuro | come contenitore della provenienza: non ha grafo dei passi (V) |
| mzTab-M 2.x | un export della tabella in futuro, dopo aver letto le specifiche | come formato interno della tabella: pesante (MTD obbligatorio, studio/assay), orientato all'identificazione (I) |
| CWL v1.2 / Nextflow | la forma della porta (`type`, `doc`, `default`, `format`) e l'idea del passo che dichiara ingressi e uscite (V) | il motore: nessuno dei due gira nel browser; i file degli studenti non lasciano il computer |
| W3C PROV-JSON | `entity`/`activity`/`agent`/`used`/`wasGeneratedBy`: è `prov/1` | il resto (bundle, delega, qualificazioni): non serve. È una Member Submission, non una Recommendation (V) |
| RO-Crate 1.2 + Workflow Run Crate | più avanti un export «pacchetto» zip (`CreateAction` con `instrument`/`object`/`result`) | ora: JSON-LD verboso, nessun beneficio per lo studente (V) |
| JSON Schema 2020-12 | i parametri del passo e la generazione del modulo (`title`, `description`, `default`, `minimum`, `maximum`, `enum`; V); già usato in `docs/schemi/` | un validatore generico in Rust: dipendenze; basta un controllo ridotto (tipo, limiti, enum, `oneOf` con discriminante) |

## 3. Matrice e buchi (dettaglio: `workflow/MATRICE.md`)
- mzLab copre l'intero tratto grezzo → tabella → TP (V), ma come chiamate interne senza descrittore.
- Buchi veri: lisciatura come passo a sé, allineamento RT per deformazione continua (abbiamo solo ancore), export mzTab-M e mzQC, TP da struttura (esclusa, §9).
- Unici nostri: lettura nativa nel browser, stima dei parametri con prova (`params::estimate`), posizione della sostituzione, livelli con prove.

## 4. Il contratto (bozza in `workflow/contratto.schema.json`)
**Tipi dei dati** (`dataType`, 14): `raw`, `mzml`, `mzidx`, `xic`, `peaks`, `rtmap`, `table`, `samples`, `spectra`, `annot`, `tp`, `structure`, `qc`, `report`. `mzidx` è un tipo a sé perché è la moneta vera del motore: ogni passo dopo la lettura lavora sull'indice (V: `Index<R>` nelle firme di `pipeline::analyse`, `params::estimate`, `run::detect`).

**Il passo `passo/1`** (uno per tipo e versione, spedito con il modulo che lo esegue):
- `id` (`features.detect`), `version` intera: SALE quando gli stessi ingressi e parametri possono dare un'uscita diversa (come `parameter_version` di mzmine, ma legata al risultato);
- `title`, `doc`, `icon` (chiave di `QICON`), `cv` (termine PSI-MS se esiste), tutti in inglese;
- `in`/`out`: porte con `type`, `many` (una per file della serie), `optional`;
- `params`: JSON Schema con `x-unit`, `x-advanced` (dietro «More», come `advanced` di OpenMS), `x-group`, `x-estimate` (valore assente = lo stima il passo e ne scrive la prova, come `floor` oggi), `x-noresult` (non cambia il risultato: memoria, fili; fuori dall'impronta);
- `runs_in`: modulo `.wasm` con il suo tetto, e/o `native`; `deterministic`; `errors`: le chiavi che può dare, nella forma dell'API `{error, error_key, params}`.

**Il workflow `mzflow/2`**: `ingressi` (nome, SHA-256, byte, tipo) e `passi` (istanze: `id` unico, `step` = `id@versione`, `in` = riferimenti `file:NOME` o `istanza.uscita`, `params` solo diversi dai predefiniti). È un grafo aciclico; l'interfaccia mostra una catena finché il grafo ha un solo cammino. `operazioni` resta il registro di `mzflow/1`; un lettore di `mzflow/1` apre ancora i vecchi file.

**La provenienza `prov/1`**: `agent` (mzLab, versione, commit, `wasm` o `native`), `plan_sha256`, `entity` (SHA-256, byte, tipo, etichetta), `activity` (passo, parametri CANONICI con le stime, `params_sha256`, `used`, `generated`, `ms`, `heap_peak_bytes`, `error`).

**Regole**:
1. Parametri canonici = JSON con chiavi ordinate, ogni valore presente (predefiniti e stime compresi), numeri nella forma più corta che si rilegge uguale; l'impronta è lo SHA-256 di quel testo, senza le chiavi `x-noresult`.
2. Chiave della cache = (SHA-256 degli ingressi, `id@versione`, `params_sha256`); stessa chiave → stesso risultato, si riusa senza rifare.
3. Ingressi immutabili: un passo non cambia mai un dato, ne produce uno nuovo (correzione 7 di `PROVE_SU_CARTA.md`).
4. Unità esplicite: `mz_tol {da, ppm}`, RT in `min` o `s` dichiarati; mai codici di segno.
5. Prima di partire il piano si controlla tutto: tipi delle porte, riferimenti, versioni presenti, passo disponibile sulla piattaforma (`err.flow.unavailable`), parametri entro i limiti.
6. Gli errori si fermano al passo: `prov/1` scrive l'attività con `error`, le uscite già fatte restano valide e in cache.

**Interfaccia generata** (I, da provare): un modulo per passo dallo schema (numero con unità e limiti, menu da `enum`, gruppi da `x-group`, «More» per `x-advanced`, «Auto» per `x-estimate` con la prova accanto), solo in inglese, icone da `QICON`, transizioni morbide spente da `prefers-reduced-motion`, tastiera e `aria-label` dai `title`. Pochi clic: un workflow salvato si riapplica con «Open» → scegli i file → «Run».

## 4b. Come si parlano i moduli (dettaglio: `workflow/COMUNICAZIONE.md`)
Un filo porta un descrittore (`type`, `sha256`, faccette come `hr`/`lr`), mai i byte. Un'uscita si collega a un ingresso se tipo (o `accepts`), cardinalità, faccette (`requires`) e aciclicità tornano; se manca una conversione registrata (`converts`), il programma la PROPONE con un clic.
Ordine (Kahn) e parallelismo (rami indipendenti, `per_item` per file; worker nel browser, fili nell'app) si ricavano da ciò che ogni passo legge e scrive; l'unica risorsa condivisa è la memoria (`memory_hint_bytes`). Ogni famiglia ha preset con `when` sulle faccette (es. `hr`, `lr`, come fa già `run::recipe_for`, V) e alternative scambiabili con la stessa firma di porte. I campi sono nella bozza e i cinque esempi la validano (V).

## 5. Prova su carta (dettaglio: `workflow/PROVE_SU_CARTA.md`)
Quattro workflow veri (mzmine FBMN, OpenMS FFM, XCMS vignetta, patRoon TP) riscritti nel formato. Si esprimono tutti nel tratto rilevamento → allineamento → riempimento; non si esprimono gli algoritmi che non abbiamo (centWave, Obiwarp, lisciatura a sé) né i programmi esterni (BioTransformer, CTS, MetFrag). Undici correzioni, già nella bozza: `id` per istanza, tolleranze composte, `oneOf` per l'algoritmo, riferimenti espliciti, `x-advanced`, conversioni dichiarate, dati immutabili, `samples` come tipo, `err.flow.unavailable`, liste dell'utente come ingressi con hash, grafo dal primo giorno.

## 6. Prototipo minimo: grezzo → mzML → feature
`crates/mzlab-cli/examples/flusso.rs` (prototipo, non va nel sito): legge un `mzflow/2-draft` a catena lineare (`convert.mzml` → `features.table`), esegue i passi con il codice del motore (`mzlab_lettori::apri`, `lettori_mzml::scrivi`, `mzlab_cli::features`, resa pubblica per poter passare la cartella di lavoro: in WASI non c'è una cartella temporanea) e scrive `prov.json` con SHA-256, tempo e picco della memoria di ogni passo (allocatore che conta, come `examples/mzml_memoria.rs`).
```
cargo run --release -p mzlab-cli --example flusso -- FLOW.json WORKDIR                     # nativo
cargo build --release --target wasm32-wasip1 -p mzlab-cli --example flusso                 # WASM (WASI)
node wasi.mjs target/wasm32-wasip1/release/examples/flusso.wasm DIR /d/FLOW.json /d/OUT    # Node 22, node:wasi, DIR montata in /d
```

### 6.1 Misure (V, container cloud x86_64, 4 CPU, `small.RAW` LTQ 1 504 354 B, 48 spettri)
| Esecuzione | conversione (ms) | feature (ms) | picco heap conversione | picco heap feature | processo |
|---|---|---|---|---|---|
| nativo, giro 1 / 2 | 52,1 / 51,1 | 49,6 / 49,0 | 2 231 848 B | 6 426 896 B | 0,11 s, RSS 11,1 MB |
| WASM (Node 22 WASI), giro 1 / 2 | 124 / 119,4 | 118,1 / 110,3 | 2 156 824 B | 6 102 097 B | memoria lineare 7,1 MB; compilazione 10 ms |
- **Hash identici** su quattro giri: `conv.mzML` (1 829 427 B) SHA-256 `92dd492951bfec2c…`, `feat.csv` `4bd43e415156e608…`, uguali tra nativo e WASM. Il picco di heap differisce (puntatori a 32 bit in WASM), il risultato no.
- **Via diretta = via mzML**: `mzlab-cli features small.RAW` e `features conv.mzML` danno le stesse 3 righe e le stesse aree (V).
- **Scala**: il file è piccolo (3 feature); tempi e memoria su file veri vanno misurati con lo stesso esempio sui file di Rosina e del timololo (F8 di Rosina, sul Mac). WASI non è il sito: nel browser i passi girano nei moduli esistenti (§9).

### 6.2 Cosa ha mostrato il prototipo
1. **Due scrittori di mzML**: `raw::to_mzml` (esempio `raw2mzml`, software «opentfraw 0.1.0», 1 846 674 B) e `lettori_mzml::scrivi` (software «mzlab 1.0.1», 1 829 427 B). Le feature dal primo hanno aree diverse dello 0,014 % (es. 2 853 741,7 contro 2 853 340,2; altezze uguali; causa non cercata, I). Nel contratto il passo `convert.mzml` è UNO e la versione nomina lo scrittore.
2. **L'mzML scritto non è `indexedmzML`** (nessun `<offset>`), dichiara il CV 4.1.163 (attuale 4.2.6) e un solo `processingMethod` (conversione). Per la prova F6 di Rosina e per §7.2 servono indice, CV aggiornato e un `processingMethod` per ogni passo che tocca gli spettri.
3. **I nomi delle colonne del CSV vengono dal nome del file** (`datafile:small_1:area`): due file identici con nomi diversi danno CSV diversi. Nel contratto l'identità è lo SHA-256, il nome è un'etichetta; i confronti di hash si fanno a nomi uguali.

### 6.3 Cosa ha mostrato il codice
- `Recipe::file_text` è il testo `{:?}` (`Debug`) delle strutture Rust con un'impronta FNV a 128 bit (V): basta rinominare un campo o cambiare il formato di un `f64` per cambiare l'impronta senza cambiare il risultato. Va bene come chiave interna della cache; per il contratto serve la forma canonica della regola 1 (§4).
- `Recipe::row_window` usa `ppm < 0` per dire «Da» (V): ammesso dentro il motore, vietato nel contratto (regola 4).

## 7. Prove esterne (progettate, non eseguite)
1. **Stessa tabella di mzmine e OpenMS**: stesso file mzML, `tools/bench/` (V: `corri_mzmine.py` batch headless 3.9.0, `corri_openms.py` FeatureFinderMetabo + legame KD, adattatori `da_csv` per mzmine, XCMS, OpenMS, patRoon). Abbinamento di una riga: HR |Δm/z| ≤ 5 ppm e |ΔRT| ≤ 0,1 min; LR |Δm/z| ≤ 0,5 Da e |ΔRT| ≤ 0,1 min. Esiti dichiarati prima: richiamo delle righe dell'oracolo sopra 10 volte il rumore ≥ 0,9; mediana di |log2(area mzLab / area oracolo)| ≤ 0,15; Spearman delle aree ≥ 0,9. Concordanza misurata, non uguaglianza (l'oracolo non è la verità); la verità sta in D1, D2 e nel timololo.
2. **mzML validato**: XSD `mzML1.1.1.xsd` e lo schema `idx` della pagina PSI (nome da riverificare), validatore semantico Java di HUPO-PSI con `ms-mapping.xml` fuori rete, `FileInfo -v` di OpenMS; rilettura = lettore (F6 di Rosina). Oggi fallirebbe sull'indice (§6.2 punto 2: I, non eseguito).
3. **Riesecuzione con hash identico**: ogni passo, due giri e tre piattaforme (Linux x86_64, Mac arm64, WASM in Chromium); il primo pezzo è fatto (§6.1). In CI: `cargo test` di un piccolo `mzflow/2` sintetico che confronta i `generated.sha256` con valori scritti.
4. **Compito del timololo con meno clic di mzmine**: dai 13 `.raw` alla tabella dei TP; si contano clic e tasti con Playwright (mzLab) e con una registrazione dello schermo (mzmine 4.10 sul Mac, batch caricato contro batch costruito). Criterio: mzLab ≤ metà dei clic di mzmine con lo stesso risultato entro le tolleranze della prova 1.

## 8. Primo workflow di punta e piano di lavoro
Catena: `import.raw` (13 `.raw` del timololo) → `convert.mzml` (facoltativo, per l'esportazione) → `index.mzidx` → `features.estimate` → `features.detect` → `features.align` → `features.gapfill` → `table.filter` (`campioni/2`) → `features.ms2_link` → `tp.link` (`interp/1`, `tp/1`) → `tp.position` → Disegno («Open in Draw»). Tutti i passi esistono come codice (V, `MATRICE.md`); mancano descrittori, canonica, `prov/1` e il modulo generato.

| Chat DEV (Sonnet) | Perimetro | Prova |
|---|---|---|
| W-1 descrittori | `passo/1` dei passi della catena come dati Rust accanto al codice, esportati in JSON; nessun cambio di risultato | i descrittori validano lo schema; le uscite dei passi sono byte per byte quelle di oggi |
| W-2 canonica e `prov/1` | parametri canonici, `params_sha256`, `prov/1` in `mzlab-core`; `mzflow/2` letto e controllato (regola 5) | riesecuzione con hash identico, nativo e WASM (prova 3) |
| W-3 un solo mzML | `convert.mzml` = `lettori_mzml::scrivi` con `indexedmzML`, CV 4.2.6, `processingMethod` per passo; `raw::to_mzml` solo dietro questo | prova 2 sui file di Rosina (F6) |
| W-4 modulo generato | form dallo schema nel pannello Pipeline (inglese, `QICON`, `prefers-reduced-motion`, tastiera, axe) | `e2e_pipeline` e `e2e_axe_grafici` verdi; tour valido |
| W-5 oracoli | prova 1 sui file di D1 e del timololo con `tools/bench/` | numeri nel privato, esiti contro le soglie dichiarate |
| W-6 clic | prova 4 | conteggio scritto in `misure.md` |
Ordine: W-1 e W-3 in parallelo; W-2 dopo W-1; W-4 dopo W-2; W-5 e W-6 dopo W-4. Locale (Mac): mzmine 4.10, ThermoRawFileParser, misure F8.

## 9. Vincoli
- **Tetti `.wasm` non alzabili** (V: `tools/build_feat_wasm.sh`, `pages.yml`): finder 512 000 B, `sciex` 327 680, `chem` 569 344, motore 1 703 936. Il descrittore e la canonica sono piccoli ma non entrano in un modulo già vicino al tetto: nel browser i passi restano nei moduli che li hanno; il controllo del piano e `prov/1` vanno in un modulo NUOVO (tetto da fissare alla prima misura, I) o nel motore solo se la misura lo consente. L'orchestrazione resta nel worker in JS come fa già `feat-worker.js` (V): chiama i moduli, non calcola.
- **Licenze**: mzLab MIT; patRoon GPL-3 → algoritmi riscritti dalle fonti originali (articoli), mai codice copiato o tradotto (`docs/studi/patRoon_TP.md` §5, PR #1092). mzmine, OpenMS e XCMS: licenze da leggere prima di riusare qualunque cosa (I: OpenMS BSD-3, XCMS GPL, mzmine 4 con termini propri); stessi limiti, si leggono i file di parametri, non il codice. Dati esterni solo con licenza letta.
- **Interfaccia** in inglese (titoli, doc, errori del registro dei passi), m/z, RT, XIC, TIC, MS2 come sempre; Teoria e Pratica non cambiano.
- **Principio 1**: nel mondo LR un workflow non produce TP né rette: i passi `tp.*` e `annot.*` sono `runs_in` HR o mzFinder.

## 10. Cosa NON fare (e perché)
1. Non adottare CWL, Nextflow, Galaxy o KNIME come motore: non girano nel browser e i dati non devono uscire (V per CWL/Nextflow).
2. Non usare mzTab-M o mzML come formato di workflow o di tabella interna: non hanno grafo né parametri tipizzati (V per mzML; I per mzTab-M).
3. Non chiamare programmi esterni da un passo (R, Java, BioTransformer, CTS, MetFrag, SIRIUS): pesanti, rete, licenze.
4. Non copiare né tradurre codice GPL (patRoon, XCMS): solo idee e articoli.
5. Non inventare un secondo formato di sessione: `mzflow/2` estende `mzflow/1`, che resta leggibile.
6. Non fare dipendere l'identità di un dato dal nome del file né l'impronta dal testo `Debug` (§6.2-6.3).
7. Non tenere due convertitori in mzML equivalenti in apparenza (§6.2 punto 1).
8. Non mettere il motore dei workflow nel modulo principale vicino al tetto; non alzare i tetti.
9. Non promettere equivalenza con i parametri di mzmine o OpenMS: un import di batch è una traduzione dichiarata (`PROVE_SU_CARTA.md` B).
10. Non costruire la tela «Flow» prima del controllo del piano in Rust: la tela disegna e basta, compatibilità e ordine li decide il motore (COMUNICAZIONE.md §5), altrimenti le regole si scrivono due volte.

## 11. Decisioni da prendere (raccomandazione)
1. **Nome e posto**: `mzflow/2` con `passi`; `passo/1` e `prov/1` in `docs/schemi/` quando W-1 e W-2 li implementano (oggi bozze qui).
2. **Dove gira il controllo del piano nel browser**: modulo nuovo piccolo (raccomandato) contro motore (solo se la misura lo permette).
3. **Scrittore unico di mzML**: `lettori_mzml::scrivi` (raccomandato: è quello del registro e dà le stesse aree della via diretta).
4. **Import dei batch di mzmine**: solo dopo W-4, come traduzione con avviso dei parametri non tradotti (come `parametri_non_letti` di `tools/bench`).

## 12. Pubblicazione delle librerie (dettaglio: `workflow/PUBBLICAZIONE.md`)
Ordine proposto: prima il codec numpress staccato da `mzlab-core` (piccolo, verità esterna nella specifica), poi `mzlab-lib`, poi i lettori; npm dopo. Licenza MIT; le licenze delle dipendenze sono già controllate a ogni PR da `cargo deny` (V). Mancano API in inglese e stabile, rustdoc, `publish = false` su tutti i crate finché Federico non dà il via. Nessuna azione esterna senza il suo «via».
