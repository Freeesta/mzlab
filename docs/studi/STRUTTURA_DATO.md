# Studio: la struttura chimica come dato di prima classe (`strutt/1`)

Parte dello studio `docs/studi/WORKFLOW_MODULARI.md` (tipo `structure` del contratto). Domanda di Federico: nel Flow il disegno e la struttura devono entrare e uscire da ogni modulo che ne ha bisogno, senza perdere la stereochimica né l'incertezza. Segni: **V** verificato (codice letto o prova eseguita qui), **I** ipotesi. Prove in un ambiente a parte (RDKit 2024.09.6 con l'InChI che contiene, Open Babel 3.1.0), mai nel repository: RDKit fa solo da oracolo.

## 1. Come mzLab rappresenta oggi le molecole (V)
- **Tre grafi diversi**: `grafo::Mol` di `mzlab-feat` (`chem/grafo.rs`: solo atomi pesanti con `el`, `arom`, `h`, `charge`; legami con ordine 1, 2, 3 o 1,5), `legacy::fragtree::Graph` (una copia di `grafo::Mol`) e `Mol3d` di `mzlab-chem` (numero atomico, idrogeni espliciti, coordinate, ordine 4 = aromatico). Tutti nascono da `grafo::Mol::parse` (SMILES) o da `from_molfile` (V2000 della pagina Disegno).
- **Cosa perde la lettura di oggi** (sonda eseguita su `Mol::parse`):
  - `@`/`@@` saltati: (S)- e (R)-timololo danno lo stesso grafo;
  - `/` e `\` ignorati: acido maleico (Z) e fumarico (E) danno lo stesso grafo;
  - l'isotopo tra parentesi si salta: `[13CH3]O` diventa CH4O senza avviso, mentre `[2H]` è rifiutato (`error.grafo.element`);
  - sali e miscele rifiutati (`error.grafo.mixture`);
  - nessuna percezione dell'aromaticità: `C1=CC=CC=C1` (0 legami aromatici) e `c1ccccc1` (6) sono due grafi diversi della stessa molecola.
- **Nessun InChI calcolato**: la revisione di Attributio legge l'InChIKey solo dalle fonti (`attributio/review/structures.rs`: «it has no InChIKey»). Il modulo RT-logP cerca il logP con il primo blocco dell'InChIKey (`rtlogp.rs`, `values(ik1)`).
- **Già vicino**: `posizioni/simmetria.rs` calcola classi di equivalenza degli atomi alla Morgan/CANON, la base di un'etichettatura canonica a cui mancano lo spareggio e la stereo (I). L'incertezza esiste già in due forme: `parz/1` (atomi + Δ «+O -H2», livello 3b; `docs/agenti/disegno.md`) e il campo `drawing` del piano di Tabui (`mode: structure|multicenter|bracket|none`, `uncertainty: none|zone|whole`; `docs/studi/TABUI_TP_TIMOLOLO.md`).
- **Nella pagina**: Ketcher (Disegno, in iframe `sandbox`) e OpenChemLib (coordinate 2D, SMILES canonico per la cache della reattività: `docs/agenti/reattivita.md`) sono già distribuiti (V).

## 2. Confronto dei formati su un insieme reale (V, 20 molecole)
Insieme: timololo S, R, non specificato, racemo, maleato (sale); L/D-alanina; efedrina (1R,2S) e pseudoefedrina (1S,2S) da PubChem (CID 9294, 7028); trans-1,2-dimetilcicloesano racemo (CXSMILES `&1`) e cis (meso); acido maleico, fumarico e non specificato; 2-idrossipiridina e 2-piridone; acetilacetone cheto ed enolo; etanolo; paracetamolo-d4. Lo SMILES del (S)-timololo di PubChem (CID 33624) dà la stessa InChIKey del nostro.
| Domanda | SMILES | molfile V2000 | molfile V3000 | InChI standard |
|---|---|---|---|---|
| Identità stabile fra programmi? | **no**: lo SMILES canonico di RDKit e di Open Babel differisce in 15 casi su 20 | no (ordine degli atomi libero) | no | **sì**: le InChIKey di RDKit e Open Babel coincidono quando lo stereo arriva intero |
| Stereo tetraedrica e Z/E | sì (`@`, `/`) | solo con coordinate e cunei: con coordinate a zero o senza cunei RDKit non trova nessun centro; la colonna di parità vale 0 | come V2000 per i centri | sì (strato `/t`, `/b`) |
| «Non specificata» contro «nessun centro» | **indistinguibili**: timololo senza `@` ed etanolo si scrivono allo stesso modo | indistinguibili | indistinguibili | indistinguibili (manca lo strato `/t`) |
| Racemo o relativa | solo CXSMILES (`&1`, `or1`) | gruppi di RDKit conservati | `STERAC`/`STEREL` | **no**: racemo e S hanno la stessa InChIKey; `/SRac` è un'opzione del programma che lo calcola, vale per tutta la molecola e non viene dal dato |
| Tautomeri | due SMILES | due | due | 2-piridone = 2-idrossipiridina (H mobile), ma cheto ≠ enolo dell'acetilacetone |
| Sali | `.` | più pezzi | più pezzi | componenti separati; la InChIKey del sale ≠ quella del genitore |
| Coordinate | no | sì | sì | no |
| Ritorno da InChI a struttura | — | — | — | il tautomero cambia (2-piridone → 2-idrossipiridina) e il paracetamolo-d4 torna come imidato (`N=C(C)O`) |

**Conversioni perdenti fra programmi** (V): in Open Babel 3.1.0 `make2D` cancella la stereo tetraedrica (timololo ed efedrina passano alla InChIKey `…-UHFFFAOYSA-N`) e il suo scrittore V3000 non scrive i cunei. Il suo V2000 invece li conserva. La stessa catena di conversioni può quindi dare identità diverse a seconda del programma: è la ragione per non far passare la struttura per formati di testo fra un passo e l'altro.

## 3. Proposta: `strutt/1`, un grafo esplicito con strati
JSON, generato e letto solo in Rust (regola «Tutto in Rust»); i formati di testo sono IMPORTAZIONI ed ESPORTAZIONI, mai il dato fra due passi.
- **Atomi**: `el`, `h` (idrogeni scritti come numero, mai ricalcolati alla lettura), `iso`, `chg`, `rad`, `map` facoltativo.
- **Legami**: solo forma di Kekulé (1, 2, 3). L'aromaticità è una PROPRIETÀ calcolata da una nostra regola unica (I: Hückel sugli anelli semplici, da fissare con prove), mai scritta nel dato: così `C1=CC=CC=C1` e `c1ccccc1` danno lo stesso grafo.
- **Stereo esplicita**:
  - `tetra: [{atom, nbrs, turn: cw|ccw}]` con il verso riferito all'ordine dei vicini scritto, non all'ordine di una stringa; l'idrogeno implicito ha un posto fisso nella convenzione (I: da fissare e provare contro lo scrittore SMILES);
  - `db: [{bond, ref: [a, b], cfg: same|opposite}]`;
  - `undefined: [{kind: atom|bond, at}]` per i centri possibili ma non specificati (distingue il timololo non specificato dall'etanolo);
  - `groups: [{kind: abs|and|or, atoms}]` per assoluta, racemo e relativa (come i gruppi di V3000 e CXSMILES).
- **Componenti**: `components: [{atoms, role: parent|counterion|solvent}]`, così il maleato di timololo resta un sale e il genitore si prende senza ricalcolare.
- **Coordinate** (strato facoltativo): `coords2d`, `coords3d`, con `source` (Ketcher, OpenChemLib, `embed.rs`). Non cambiano l'identità.
- **Identità**: `id.canon` = SHA-256 della nostra forma canonica (classi di `simmetria.rs` + spareggio + stereo; I), più `id.inchikey` quando un modulo InChI la calcola (§4), con `id.source`.
- **Incertezza** (strato facoltativo, già in uso come `parz/1` e `drawing`): `regions: [{atoms, delta, mode: zone|whole|multicenter, confidence}]` e `sites: [{atom, p}]`. Uno SMILES non può dire «OH da qualche parte sull'anello»; qui è una regione con il suo Δ e il suo livello.
- **Sottostrutture**: frammenti e perdite neutre come `{of: id.canon, atoms, bonds_cut, formula}`, cioè indici del genitore, come fa già `frag/1` (`chem/frammenti.rs`).

**Prova di andata e ritorno** (V sul contenuto del formato, non sulla nostra implementazione che non esiste ancora): RDKit → `strutt/1` → RDKit sulle 20 molecole.
- In 19 casi su 20 coincidono tutte e quattro le prove: InChIKey, InChI con `/SRac`, CXSMILES e lista dei centri non specificati.
- Nel maleato di timololo coincidono InChIKey e InChI (lo strato `/b` di Z c'è). Lo SMILES di ritorno perde solo `/\`, perché il decodificatore di prova non imposta le direzioni dei legami che lo scrittore di RDKit chiede: è un limite della prova, non del formato.
- La prova da rifare con la nostra implementazione in Rust è la stessa, con RDKit e l'InChI ufficiale come oracoli nei test di parità.

## 4. InChI nel browser: licenza e fattibilità
- **Licenza**: il file `LICENSE` del repository IUPAC-InChI è MIT (V, letto su github.com/IUPAC-InChI/InChI; ultima versione 1.07.5 del 17/02/2026). Da quale versione valga MIT non è chiaro: il CHANGELOG cita il passaggio da LGPL alla «IUPAC/InChI-Trust Licence» nella 1.04 e due correzioni del testo nelle 1.07.x (I). Va riletto prima di distribuire.
- **WASM**: esiste una build ufficiale con Emscripten nel repository `IUPAC-InChI/InChI-Web-Demo` (MIT; V che esiste, dimensione del `.wasm` non dichiarata: I). RDKit MinimalLib pesa 7,38 MB di `.wasm` (V): troppo per mzLab. Un InChI in Rust puro: non trovato (ricerca su crates.io non riuscita: I).
- **Strada proposta**: un modulo `.wasm` a parte, caricato solo quando serve un'InChIKey, compilato dal C ufficiale (oggi in mzLab non c'è nessun C nel `.wasm`: si deve provare la catena di build, I). Non deve entrare nei tetti esistenti (chem 569344 B, finder 512000 B): un modulo nuovo con il suo tetto, misurato. Senza quel modulo l'identità resta `id.canon`, e l'InChIKey arriva dalle fonti come oggi.

## 5. Struttura nel Flow (tipo `structure` del contratto)
- **Entrate**:
  - `structure.draw`: Ketcher già nel sito (versione 3.18 secondo il Coordinatore, non letta qui: I), coordinate 2D da OpenChemLib;
  - `structure.import`: incolla SMILES, molfile V2000/V3000, SDF, InChI, file;
  - `structure.from_result`: un clic su un'annotazione o su un TP prende la sua struttura.
  - Un nome si accetta solo se c'è un vocabolario locale (I: oggi non c'è).
- **Uscite**: `structure.render` (`strutt/1` + strato d'incertezza → SVG, con lo stesso disegno di `TPMOL.svg(molfile, drawing)` e `parentesi.js`); `structure.export` (SMILES, CXSMILES, molfile V3000, SDF, InChI quando c'è il modulo).
- **Chi legge una struttura** (oggi sempre da uno SMILES, V): interpretazione (`interp`, `parent.smiles`), posizioni (`positions_json(smiles, …)`), reattività/Fukui (`reactivity_json(smiles, …)`), RT-logP (primo blocco dell'InChIKey), confronto con le librerie (InChIKey delle fonti). Con `strutt/1` tutti ricevono lo stesso grafo, e la stereo arriva a chi ne ha bisogno. Nessuno di questi passi oggi usa la stereo; la vista 3D sì (I).
- **Faccette del tipo** (per la regola di compatibilità di `COMUNICAZIONE.md`): `stereo: full|partial|none`, `uncertain: true|false`, `components: n`. Es. la reattività chiede `uncertain: false` e `components: 1`; il disegno accetta tutto.

## 6. Cosa NON fare
1. Non usare lo SMILES come chiave di identità né come dato fra due passi: cambia fra programmi (15 casi su 20) e non distingue «non specificata» da «assente».
2. Non affidarsi alla parità del molfile: vale 0 nei file letti e la stereo vera sta nelle coordinate e nei cunei.
3. Non ricavare il racemo dalla InChI standard: è uguale all'enantiomero.
4. Non convertire una struttura passando per Open Babel `make2D` o per il suo V3000: perdono la stereo (§2).
5. Non portare RDKit o Open Babel nel sito: pesano troppo e sono librerie esterne al motore (Principio 2). Restano oracoli dei test.
6. Non scrivere l'aromaticità nel dato: due scritture della stessa molecola diventerebbero due grafi, come oggi.

## 7. Da provare (I)
- La lettura in Rust di `@`, `/`, isotopi, sali e CXSMILES `&`/`or` nel grafo comune, senza rompere i test di oggi: gli idrogeni e le formule devono restare uguali.
- L'etichettatura canonica con stereo e la prova «stessa `id.canon` per i 20 SMILES di RDKit e di Open Babel della stessa molecola».
- La build WASM dell'InChI: byte, tempo, InChIKey uguale a RDKit sulle 20 molecole.
- La regola unica dell'aromaticità sui casi difficili (piridone, tiadiazolo del timololo, azuleni).
