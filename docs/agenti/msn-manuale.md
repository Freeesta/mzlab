# Manuale d'uso MSn: dall'infusione `.raw` all'albero, alla libreria e alla tabella

Per chi lancia il lavoro dalla shell o da un piano Flow; il progetto del grafo è in `msn-grafo.md`, la libreria in `msnlib.md`. Il programma non dà verdetti: le sottoformule sono candidati con ppm e prove.

## 1. Riga di comando (`mzlab-cli`)
- **Grafo**: `mzlab-cli msn-graph INFUSIONE.raw --formula "C13H24N4O3S [M+H]+" --out grafo.json`. Il file può essere un `.raw` Thermo o un mzML; `--formula` accetta lo ione (`C13H25N4O3S+`, `C13H24N4O3S+H`, `"FORMULA [M+H]+"`); senza, il grafo ha solo le masse. Opzioni utili: `--activation cid|hcd` (un solo tipo), `--ppm-formula 10`, `--isotopes`, `--pick MZ=ION` (fissa la formula di un nodo), `--smiles` (scritto nel JSON). `--summary` stampa una riga di numeri.
- **Libreria**: `mzlab-cli msn-lib add lib.json INFUSIONE.raw --id ID --formula ION` aggiunge la voce (la stessa voce due volte resta com'è); `msn-lib build MANIFEST.tsv` ne scrive una per riga (`id`, `formula_neutra`, `addotto_atteso`, `file`).
- **Esportazione come libreria**: `mzlab-cli msn-lib export lib.json --format msp|massbank [--clean] [--all] [--annotate]` (`--clean` tiene i picchi spiegati da un nodo; `--all` ogni livello MS; `--annotate` scrive formula e perdita neutra di ogni picco).
- **Esportazione come tabella**: `mzlab-cli msn-graph INFUSIONE.raw --formula ION --export csv|mgf|msp [--clean] [--node nK|tutto]`.
- **Cerca**: `mzlab-cli msn-lib search lib.json grafo.json [--ppm 10] [--tol 0.01] [--min-evidence 1] [--top 10] [--delta Da]` dà `msncerca/1`: tre punteggi separati (entropia MS2, albero, perdite condivise) con le prove, senza verdetto.

## 2. Passi del Flow
Piano `mzflow/2` con l'ingresso `.raw` (o mzML) e, in ordine:
- `spectra.msn_graph@2`: porta `source` (l'infusione), uscita `graph` (`msngrafo/2`). Il composto si dà con `formula` + `adduct`, oppure con `molecule` (nome) e la porta `molecules` (elenco CSV delle molecole); gli altri parametri hanno i nomi dei flag di `msn-graph` (`ppm_formula`, `activation`, `isotopes`, `pick`, `smiles`, …); un piano salvato a `@1` con i nomi italiani si apre e gira uguale.
- `msn.library_export@1` (`graph` → libreria): `format` msp|massbank, `levels` all|ms2, `clean`, `annotate`, `id` (nome del record).
- `msn.table_export@1` (`graph` → tabella CSV): `view` modifications|nodes|peaks, `clean`.
Le perdite neutre stanno nella tabella `modifications`, le sottoformule in `nodes`, i picchi in `peaks`; la vista «Grafo MSn» della corsa le mostra senza aprire il CSV. Ogni parametro ha il suo `--set ISTANZA.NOME=VALORE` in `mzlab-cli flow run`.

## 3. Come si legge il risultato
- Ogni nodo è uno ione con formula proposta e ppm; l'arco tra due nodi è la perdita neutra (formula e massa a 5 decimali).
- Un frammento con più formule possibili resta con tutti i candidati: la scelta è dell'utente (`--pick`).
- A risoluzione unitaria un m/z è un candidato, non un'identificazione.

## 4. Elenco degli esperimenti MSn di un file DDA

Codice: `mzlab/web/msn-elenco.js` (modulo ES caricato al primo uso da `COMP.tree` in `composizione.js`), `DDA.get`/`DDA.showMs2` (`dda.js`). Prova: `tests_e2e/e2e_hr_msnelenco.py`.

- **Quando**: la finestra «Albero MSn» di un file con un elemento `dda` si apre sulla scheda «Experiments»; la scheda «Tree» c'è solo se il file ha MS3 o oltre (l'albero di `/api/msntree` rifiuta i DDA con soli MS2) e si calcola alla prima apertura. Le frecce passano da una scheda all'altra.
- **Dati**: `/api/dda` (Rust `hrroutes.rs`, riferimento `explore.dda`, parità `hr_parity`) con le colonne `lev` (livello MS) e `z` (carica, null se manca); l'intensità del picco base da `/api/scanlist?level=N`. `groups(D, tol)` raggruppa le scansioni per il precursore del loro antenato MS2 (moda dei m/z entro la tolleranza del file).
- **Tabella**: `role=grid`, righe di 24 px; nel DOM quelle visibili più uno schermo sopra e uno sotto, ridisegnate quando il margine scende sotto mezzo schermo (lo scorrimento veloce non mostra bande vuote; 50 000 scansioni in meno di 0,2 s). Intestazioni ordinabili (`aria-sort`; la colonna dell'attivazione ordina i gruppi per numero di scansioni), filtri m/z, RT, livello e «espandi tutto» (acceso fino a 200 gruppi). Tastiera: frecce, Pag, Home/Fine, Invio/Spazio, Sinistra/Destra chiudono e aprono un gruppo (`aria-activedescendant`).
- **Azioni**: clic o Invio su una scansione chiude la finestra e la mostra nel trio DDA (`DDA.showMs2`). Per gruppo: «TIC» = cromatogramma della scheda MS2 con `prec` e `ptol` (metà della dispersione dei precursori + `HR.tolDa`) e il suo spettro collegato (`addMs2Spec`); «XIC» = `xicDirect` del m/z sulla scheda Full con la finestra ppm del file e uno spettro collegato.
- **`ptol`** di `/api/chrom` (Rust `chrom_raw`): finestra del precursore in Da, predefinita 0,6; è una proprietà del solo pannello (`getChrom`, salvata nel taccuino), perché `ms2_exps` arrotonda i precursori a 0,1.
- **Trappola**: i filtri ascoltano solo `input`, non `change` (il `change` al blur ricostruiva le righe a metà di un clic).

## 5. File di prodotto a un livello MSn diverso da 2 (MS3)

Codice: `iteminfo.rs` (`msn_level`), `axisroutes.rs` (`ms2_transitions(level)`), `itemroutes.rs` (`item_method`), `explore.js` (`lv`, `getMs2Tr`), `titoli.js`, `picchi.js`, `spettro.js`, `pannelli.js`, `tabs.js`, `eventi.js`, `hr.js`. Prove: `crates/mzlab-core/tests/ms3_only.rs` (feature `sciex`, sulla coppia sintetica `ms3` di `tools/wiff_riferimento.py`), `tests/test_titoli.py`, `tests_e2e/e2e_ms3.py` (file di `ms3` in `tools/synthetic_data_msn.py` e un secondo file MS3 con due precursori), `tests_e2e/e2e_ms23.py` e le prove di parità di `iteminfo`/`mzidx` (file di `ms2_ms3`).

- **Livello del file**: l'info di un file (`item_info`) ha `msn_level` = il livello più basso fra le sue scansioni di livello ≥ 2 (`null` se non ne ha); un file di sole MS3 ha `kind` `ms2` e `msn_level` 3. L'intervallo m/z del prodotto (`mz2_min`/`mz2_max`) viene dalle scansioni di quel livello.
- **Interfaccia**: la scheda dei prodotti usa `lv` = `msn_level` del file (2 se manca): cromatogramma del precursore, lista delle scansioni, spettro e tabella dei precursori (`/api/ms2tr?level=`, 2 se manca) chiedono la sezione di quel livello, mai una sezione MS2 che il file non ha. Il menu del livello dello spettro offre «MS3» per un file di sole MS3.
- **Titoli**: con `lv` 3 i titoli dicono «TIC · MS³», «TIC · MS³ (tutti i precursori)», «TIC · MS³ · precursore {prec}», «MS³ di {prec}» (le chiavi `panel.title.ticMs2`, `ticMs2All`, `ms2prec`, `productOf` hanno `{ms}` = «MS²» o «MS³»).
- **Spettro di un livello > 1** (`picchi.js`, `spettro.js`): come per MS2, un precursore che il file mostrato non ha porta al file caricato che lo ha; «tutti i precursori» di un file a bassa risoluzione dà uno spettro per precursore; l'asse m/z si adatta ai picchi dello spettro e al precursore + 5; la riga della scansione dà la CE se il file la dice.
- **Esportazioni dello spettro**: nome del file «spettroMS{n}» (`export.file.spectrumMsn`), descrizione e foglio dei metadati «MS2»/«MS3» («MS²»/«MS³») secondo il livello del pannello.
- **Precursore**: di una scansione MS3 è la seconda massa isolata (quella del lettore), come nell'mzML scritto; la prima è un userParam del lettore e non entra nei menu.
- **Metodo** (`/api/method`): i tempi dei cicli di un file di prodotto si contano sulle scansioni del suo livello più basso ≥ 2.
- **File con MS2 e MS3 insieme**: `file_parts` (`iteminfo.rs`, `explore.py`) lo divide in una parte per livello, `file#MS2` e `file#MS3` (campo `level` della parte; `sniff` le chiama «MS2» e «MS3»); ogni voce ha il suo `msn_level` e la sua etichetta «· MS2»/«· MS3».
- **Lista e titoli di un file misto**: il gruppo dei prodotti nella lista dei file nomina i livelli presenti («MS2/MS3 (Product Ion)», `msLv` in `explore.js`, anche per il gruppo `hr.ms2` con `files.group.ms2` = «{ms} (Product Ion)»). Il titolo del cromatogramma usa il livello dei file che hanno il suo precursore (`titoli.js`); scegliere il precursore MS3 porta spettro e cromatogramma a livello 3.
- **Fuori da qui**: le librerie e l'identificazione restano su spettri MS2; anche MGF, MSP, MassBank e SIRIUS dello spettro (`workflow.js`, `fair-azioni.js`) si offrono solo per MS2, perché scrivono il precursore come ione del campione; i pannelli MS2 di un trio DDA (`dda.js`) restano di livello 2.

## 6. Figura e HTML del grafo MSn (`msngrafo/1` senza aprire l'app)

- **Codice**: `crates/mzlab-feat/src/msnfigure/` (feature `figura`; CLI e `.wasm` `msnfigura`): `mod.rs` (SVG statico: stesso layout `vista` albero/radiale della pagina, `completa` se manca, stessa regola dei testi di `msngrafo-vista.js`: m/z a 4 decimali, % relativa a 1, livelli, fino a 4 candidati (1 nel radiale), perdita come formula o «Δ x.xxx Da» con «(compatible with …)», regola «ids (weak), +n»; colori Okabe-Ito scritti nell'SVG, nessun CSS esterno).
- **Altri file**: `html.rs` + `html.html` + `pagina.js` (HTML autonomo), `figure_text` (svg o html del passo); in `crates/mzlab-cli/src/`: `msn_png.rs` (PNG), `msn_outputs.rs` (formati, scrittura, comando `figure`), `flow_msnfigure.rs` (passo).
- **Radice**: se il grafo ha un molfile con coordinate 2D la scatola della radice disegna la molecola (`molecola`, da `mzlab-mol`); con SMILES o molfile ma senza coordinate scrive «no 2D coordinates»; senza struttura niente.
- **HTML**: un solo file, nessuna risorsa esterna, nessuna richiesta di rete (CSP nel meta); contiene il JSON, il disegnatore `msngrafo-vista.js` (senza `export`), gli SVG di Rust delle due viste per i download, la tabella dei dati con «Copy data», vista Tree/Radial, zoom, tastiera (Tab sugli ioni, Invio per i candidati), axe 0 violazioni; 80 ioni ≈ 390 KB (limite 2 MB).
- **PNG**: feature cargo `png` (non predefinita, mai nel `.wasm`; `resvg`, MPL-2.0, ammessa da `deny.toml`) a scala 2 con i font di sistema, quindi il PNG dipende dai font installati; senza la feature l'errore è `err.msnfigura.png` con `build=cargo build -p mzlab-cli --features png`. Nel browser il PNG nasce dallo SVG con un canvas (`msngrafo-immagine.js`; anche l'HTML ha «Download PNG»).
- **Riga di comando**: `mzlab-cli msn-graph FILE --out STEM --figure svg,html[,png] [--view tree|radial]` (non con `--summary`, `--export`, `--structures`) scrive `STEM.svg|html|png` accanto al JSON; `msn-graph figure IN.json [--figure …] [--view …] [--out STEM]` parte da un grafo già fatto (predefinito svg,html).
- **Flow**: passo `msn.figure@1` (`graph` report → `figure` report; parametri `format` svg|html|png, `view` tree|radial), SEPARATO da `spectra.msn_graph` perché un'esecuzione dà un solo artefatto e ogni formato è una sua esecuzione; gli stessi byte del comando (prova in `msn_figure_tests.rs`); ogni parametro ha il suo `--set`; descrittore in `docs/schemi/steps-cli-1.json`.
- **Flow nel browser**: svg e html dal `.wasm` `msnfigura` (`figure_text`, `flow-web-struct.js`), stessi byte di `flow run` (`e2e_flow_web_struct.py`); il PNG resta al computer.
- **Prove**: `msn_figure_tests.rs` (golden in `tests/msn_figura/`, riscritti con `MZLAB_UPDATE_FIGURE=1`), `tests/msn_figura_parita.js` (i testi di Rust = quelli del disegnatore, anche sul grafo da 80 ioni di `msngrafo_80.json`), `tests_e2e/e2e_msn_figura.py` (file://, rete, tastiera, download, axe).

## 7. Prove delle regole di frammentazione (`mzlab-cli msn-rules`)

Solo numeri e criteri; nessuno spettro. Comando: `python3 tools/validate_rules.py` (MassBank 2026.03, Zenodo 19590880, record CC0/CC BY con SMILES; dettagli in `msn-interpreta.md`). Ogni record diventa un grafo a un livello (`graph_from_peaks`) con la sua formula e il suo SMILES; i picchi frammento escludono quello del precursore.

### Criteri
- Strati separati per attivazione e polarità: `cid_trappola` (CID su trappola), `hcd`, `qtof` (CID su TOF, letto come fascio), `fascio` (CID su altri strumenti, letto come fascio). Mai mescolati.
- Copertura (a) picchi con una sottoformula; (b) più regole della sola formula; (c) più regole della struttura. Intensità in % del picco base. I record con (a) < 0,5 (massa nominale) sono contati a parte (`nominali`) e restano fuori dalla prova a 10 ppm.
- Decoy deterministici (seme per record = FNV-1a dell'id): `ds` legami della struttura rimescolati con scambi che conservano l'ordine (stessa formula, stessi gruppi); `dp` spettro di un altro record con stesso precursore nominale e polarità e formula neutra diversa (lo stesso composto a un'altra energia o un isomero non vale); `dl` perdite spostate di −0,0137 Da.
- La copertura (c) supera la prova se, per ogni decoy applicabile, (c − decoy) ≥ 0,5·c. Una regola con meno di 20 applicazioni vere è «non misurata». Il tasso di una regola è il massimo, sui decoy applicabili, di applicazioni decoy / applicazioni vere; `ds` si applica solo alle regole con `site` o `kind` cleavage (una regola che chiede un gruppo o un elemento lo passa per costruzione). Sopra 0,30 in uno strato con almeno 20 applicazioni la regola scende a priorità 3 in `fragrules-1.json`, con il suo perché.
- Le regole della sola formula valgono per addotti rari ([M+Na]+, [M+NH4]+, [M+Cl]-, [M+HCOO]-) e la copertura (b) resta 0 negli spettri [M+H]+/[M-H]-.

### Registri e scarti
Scartati: attivazione non letta 10897, ione non supportato 250, SMILES non leggibile 693, spettro vuoto 375.

| strato | pol | registri | nominali | (a) | (c) | c−ds | c−dp | c−dl | prova |
|---|---|---|---|---|---|---|---|---|---|
| cid_trappola | − | 333 | 19 | 0.995 | 0.217 | 0.129 | 0.218 | 0.217 | sì |
| cid_trappola | + | 2514 | 89 | 0.998 | 0.283 | 0.168 | 0.284 | 0.283 | sì |
| fascio | − | 35 | 137 | 0.757 | 0 | 0 | 0 | 0 | sì |
| fascio | + | 42 | 326 | 0.845 | 0.049 | 0.034 | 0.049 | 0.049 | sì |
| hcd | − | 5742 | 705 | 0.994 | 0.084 | 0.048 | 0.086 | 0.084 | sì |
| hcd | + | 17562 | 1609 | 0.993 | 0.159 | 0.119 | 0.158 | 0.159 | sì |
| qtof | − | 1618 | 405 | 0.952 | 0.079 | 0.044 | 0.074 | 0.079 | sì |
| qtof | + | 5058 | 949 | 0.938 | 0.17 | 0.113 | 0.169 | 0.17 | sì |

La prova passa in ogni strato; nello strato `fascio` (c) vale al più 0,05: dati insufficienti, non regola respinta. Dove (c) è alta (CID su trappola, 0,22-0,28) i decoy `dp` e `dl` la azzerano e `ds` la riduce a circa 0,4.

### Regole declassate a priorità 3

| regola | strato, pol, applicazioni vere, tasso |
|---|---|
| `ion_tropylium` | hcd + 130 0.454 |
| `loss_nh3_amide` | cid_trappola + 30 0.4; hcd + 133 0.361; qtof + 29 0.483 |

Le coppie composte («a+b») seguono le regole che le compongono. Le altre regole hanno tasso ≤ 0,30 o meno di 20 applicazioni (non misurate): restano alla priorità della tabella, senza prova a favore.

### Protocollo (regole fisse)
- Insieme: MassBank Europe alla release fissata da `tools/librerie_pacchetti.py` (SHA-256 controllato), record CID o HCD con SMILES e licenza CC0 o CC BY; nel repository solo accessioni e conteggi, gli spettri restano nella cache `.verifica/`. Frammento entro 10 ppm (20 sotto m/z 150), perdita entro 3 mDa, picchi sopra il 2 % del picco base; generatore dei decoy in Rust con seme fisso.
- I SMARTS documentali si confrontano con RDKit solo in uno strumento locale, mai in CI: il valutatore Rust è l'unico motore.
