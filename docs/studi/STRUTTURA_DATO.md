# La struttura chimica come dato centrale di mzLab

Studio di architettura: come rappresentare una molecola una volta sola, per tutto il programma (Tabui, regole di trasformazione, Fukui, interp, posizioni, Disegno, Flow). Solo documenti e prototipi di misura; nessun codice di prodotto toccato.
File collegati: prove e numeri in `STRUTTURA_DATO_PROVE.md`; record delle trasformazioni e collegamenti in `STRUTTURA_DATO_TRASFORMAZIONI.md`. Strumenti: `tools/struttura_dato/` (riproduzione in fondo).
Legenda: **[verificato]** = letto nel codice o misurato qui, con il comando; **[ipotesi]** = proposta o deduzione non ancora provata.

## 0. Risposta breve
- **Sì al grafo interno in Rust come dato centrale**, con la stereochimica scritta in modo esplicito e con uno stato («definita», «sconosciuta»), non dedotta dalla stringa. Il prototipo del record `mol/1` porta 20 molecole su 20 avanti e indietro senza perdere nulla, anche racemi e relativa (§3, [verificato] con RDKit come arbitro).
- **Lo SMILES resta un formato di scambio, mai un identificatore**: RDKit e Open Babel scrivono SMILES diversi per 14 molecole su 20 dell'insieme di prova; «stereo sconosciuta» e «nessuno stereocentro» si scrivono uguali (timololo senza `@` e esano: nessun segno in entrambi) [verificato].
- **L'identità è l'InChIKey standard più il nostro canonico**: l'InChIKey distingue gli enantiomeri, i diastereoisomeri e Z/E, unisce i tautomeri 2-idrossipiridina/2-piridone (ma non chetone/enolo), **non** distingue il racemo dal puro S (stessa chiave `BLJRIMJGRPQVNF-JTQLQIEISA-N`) [verificato]. Per questo serve anche lo stato della stereo nel nostro record.
- **InChI nel browser è possibile**: la libreria ufficiale IUPAC 1.07.5 è MIT e l'ho compilata in wasm qui: 689 873 B (286 401 B compressa), 20 InChIKey su 20 uguali a RDKit [verificato]. Non entra in nessun `.wasm` esistente: andrebbe in un modulo a parte, caricato solo quando serve (decisione di Federico, §5). Un InChI puro Rust esatto non esiste (§5).
- **Le proprietà per atomo (Fukui, cariche, probabilità di sito) si agganciano agli indici del record**, che seguono il nostro ordine canonico, non l'ordine in cui è stato scritto lo SMILES (oggi è così, e due scritture della stessa molecola danno indici diversi) [verificato nel codice].

## 1. Come sono rappresentate oggi le molecole [verificato, letto nel codice]
| Dove | Cosa tiene | Cosa perde |
|---|---|---|
| `crates/mzlab-feat/src/chem/smiles.rs` (lettore SMILES) | atomi e legami nell'ordine scritto, anelli, cariche, H nei `[]` | `@`/`@@` saltati (r. 108), `/` e `\` saltati (r. 199), isotopo (cifre iniziali del `[]` saltate, r. 89), classe atomica; aromaticità solo come scritta (minuscole) |
| `chem/grafo.rs` (`Mol`: `Atom{el,arom,h,charge}`, `Bond{a,b,order f32}`) | grafo degli atomi pesanti; ordine 1,5 per aromatico | rifiuta `.` (sali, miscele), `[H]` esplicito e elementi fuori da B C N O P S F Cl Br I; massimo 200 atomi; niente stereo, niente coordinate |
| `Mol::from_molfile` (V2000 dal Disegno) | atomi nell'ordine del file, `M  CHG`, legame tipo 4 aromatico | campo stereo dei legami (cunei) ignorato, `M  ISO` ignorato, atomi H espliciti rifiutati |
| `chem/posizioni/` (`posiz/1`), `simmetria.rs` | siti e classi di simmetria (raffinamento alla Morgan) sugli indici di `grafo::Mol` | gli atomi enantiotopici e diastereotopici cadono nella stessa classe (non c'è stereo) |
| `crates/mzlab-chem` (`Mol3d`, `reatt/1`) | atomi pesanti nell'ordine dello SMILES, poi gli H; geometria ETKDG; Fukui per atomo | la geometria non segue nessuna stereo scritta (non arriva); per un solo centro non cambia nulla (i due enantiomeri hanno gli stessi Fukui), per due centri sì [ipotesi: fisica degli specchi] |
| `legacy/fragtree/mol.rs` (`Graph`) | copia di `grafo::Mol` per i frammenti | come sopra |
| `interp/` (`interp/1`) | differenza di formula dal genitore, alternative, controllo isotopico; `drawing` con indici nell'ordine dello SMILES del genitore | struttura del TP mai scritta come grafo: solo genitore + Δ + zona |
| `chem/frammenti.rs` (`frag/1`, `parz/1`) | lati di un legame spezzato; struttura parziale con Δ e `level: "3b"` | indici nell'ordine del molfile di Ketcher, diversi da quelli dello SMILES |
| `TP_Mine/js/46_tp_mol.js` (`TPMOL`) | coordinate 2D di OpenChemLib sugli indici dello SMILES del genitore | presuppone che OpenChemLib tenga l'ordine dello SMILES: vero per SMILES semplici, falso con `[H]` scritto (§4 di `_PROVE`) |
| `crates/mzlab-target` (`entry.rs`), `TP_Mine/data/composti.csv` | SMILES + InChIKey + `stereo_to_verify`, `is_salt_or_hydrate` | 625 righe: InChIKey coerente con lo SMILES in 625; 286 hanno elementi stereo, 111 con almeno uno non specificato (`confronto.py --composti`) |
| Disegno (Ketcher 3.18, OpenChemLib in vendor) | `.ket`, molfile V2000/V3000, S-group DAT `MZLAB`/`MZLAB_ENDPTS` (Δ, multicentro) | il passaggio al motore è V2000 senza stereo (vedi sopra) |

**Conversioni** [verificato]: SMILES → `grafo::Mol` (Rust, `smiles.rs`); molfile V2000 → `grafo::Mol` (`from_molfile`); `grafo::Mol` → `Mol3d` (+H, `mzlab-chem`); SMILES → grafo con coordinate (OpenChemLib, `TPMOL`); Ketcher → SMILES (`plainSmiles` toglie il CXSMILES) e → V3000 (`multiMolfile`); `fragtree` → SMILES di una sottostruttura (`mask_smiles`). Ognuna ha il suo ordine degli atomi: quattro ordini diversi (SMILES scritto, molfile di Ketcher, OpenChemLib, H in coda in `Mol3d`).

## 2. Cosa non va con i formati di oggi [verificato, `STRUTTURA_DATO_PROVE.md` §1]
- **SMILES**: non è un identificatore stabile (14/20 scritti diversi da RDKit e Open Babel; OpenChemLib scrive ancora un'altra forma, per esempio `n[s]n` nel timololo). Non distingue «stereo sconosciuta» da «non c'è stereo». Il CXSMILES di RDKit porta i gruppi `&1`/`o1` (racemo, relativa), ma Ketcher, Open Babel e il nostro lettore non lo usano allo stesso modo.
- **Molfile**: la stereo dei centri sta nei cunei; senza cunei va persa (7 casi V2000 su 7 con un centro definito). Peggio, sui doppi legami non specificati le coordinate 2D **inventano** una Z/E se manca il flag «either» (but-2-ene). Racemo e relativa solo in V3000 (`STERAC`, `STEREL`).
- **InChI**: buona identità, ma non un disegno (niente coordinate, atomi riordinati, H mobili); la versione standard non porta racemo né relativa (`timolol_rac` = `timolol_S`; `trans_dmch_rel` diventa assoluta `/s1`).

## 3. Proposta: il record `mol/1` [ipotesi di formato; andata-e-ritorno verificata con RDKit]
Un record per una molecola connessa (un componente). Gli strati sono separati: chi non ne ha bisogno li ignora.
```
{ "format": "mol/1",
  "atoms":  [{"el":"C","iso":0,"chg":0,"rad":0,"h":3}, …],        // indice = ordine canonico nostro
  "bonds":  [{"a":0,"b":4,"o":1}, …],                            // Kekulé 1/2/3; l'aromaticità si calcola con la nostra regola, non si scrive
  "stereo": [{"t":"tetra","c":6,"refs":[5,7,8],"implicitH":true,"par":"cw","status":"defined"},
             {"t":"db","a":8,"b":9,"refs":[22,23],"cfg":"cis","status":"defined"},
             {"t":"tetra","c":3,"status":"unknown"}],
  "groups": [{"kind":"abs|and|or","atoms":[6]}],                  // and = racemo, or = relativa/una delle due
  "coords": {"dim":2,"xy":[[x,y],…],"from":"ketcher"},             // facoltativo; mai fonte della stereo dopo l'importazione
  "id":     {"can":"…","inchikey":"…","skeleton":"14 lettere"},
  "props":  [{"name":"fukui_minus","from":"reatt/1 GFN2-xTB ALPB","can":"…","v":[…]}],
  "regions":[{"atoms":[…],"delta":"+O","kind":"zone|multicenter|whole","support":0.6}] }
```
- **Atomi**: elemento, isotopo (0 = naturale; serve per gli standard deuterati, `timolol_d9` [verificato]), carica, radicale, numero di H. Gli H restano un numero sull'atomo pesante; un H con isotopo (D) è un atomo.
- **Legami**: si scrive la forma di Kekulé, che non dipende da nessun modello di aromaticità; il flag aromatico è un **derivato** con una regola unica nostra (Hückel 4n+2 sugli anelli piccoli, scritta e provata in Rust). Oggi il lettore si fida delle minuscole: la regola nuova va confrontata con RDKit sulle 625 righe di `composti.csv` prima di sostituirlo [ipotesi].
- **Stereo esplicita**: un elemento per ogni centro tetraedrico e doppio legame **che può essere stereogenico**; `refs` ordinati per indice canonico e `par` (orario/antiorario guardando dal primo riferimento) o `cfg` (cis/trans tra i due riferimenti). Così la stereo non dipende dall'ordine di scrittura. `status: "unknown"` = c'è il centro ma non si sa; **nessun elemento** = il centro non esiste. Le etichette CIP (R/S, E/Z) si **calcolano** dal record, non si scrivono.
- **Stereogenicità**: decidere quali atomi sono stereocentri richiede un algoritmo nostro (classi di simmetria dei vicini, caso meso, para-stereocentri). È la parte più difficile [ipotesi]: si scrive in Rust con `Chem.FindPotentialStereo` di RDKit come riferimento dei test di parità (dati d'oro da `tools/`), come per gli altri motori.
- **Ordine canonico nostro**: raffinamento delle classi già in `simmetria.rs` + rottura dei pari deterministica + stereo come invariante finale. Prova da fare: due scritture della stessa molecola danno lo stesso record byte per byte [ipotesi]. Il canonico è nostro e versionato (`can` porta la versione): se cambia l'algoritmo, cambia la versione e i `props` vecchi si rimappano o si ricalcolano.
- **Identità a tre livelli**: stessa molecola = `inchikey` uguale **e** stesso stato della stereo (per separare racemo e puro); stesso scheletro = primo blocco dell'InChIKey (già usato da Attributio, `inchikey_block`); stessa formula. Dove l'InChIKey non c'è (nessun modulo InChI caricato) basta `can`. Per il Flow (tipo `structure`, che indica le strutture per SHA-256) l'hash è quello del **nucleo** canonico (atomi, legami, stereo, gruppi), senza coordinate, proprietà e regioni: ridisegnare non cambia l'identità.
- **Sali e miscele**: un record = un componente. La voce di una lista (`composti.csv`, una libreria) tiene il genitore e i controioni a parte; l'identità per l'MS è quella del genitore neutro (come fa già `is_salt_or_hydrate`). L'InChIKey del sale è un'altra (`timolol_maleate` `WLRMANUAADYWEA-…`, `diclofenac_na` termina `-M`) [verificato].
- **Tautomeri**: il record tiene il tautomero disegnato; l'InChI standard ne unisce alcuni (1,3 tra eteroatomi) e non altri (chetone/enolo) [verificato]. Niente tautomero canonico nostro (§6).
- **Coordinate**: strato facoltativo, 2D (Ketcher, OpenChemLib) o 3D (ETKDG di `mzlab-chem`), con la fonte. All'importazione da molfile la stereo si **legge** da cunei e coordinate e si scrive negli elementi stereo; dopo, le coordinate servono solo al disegno.
- **Proprietà per atomo** (`props`): Fukui f(+), f(−), f(0), cariche, probabilità di sito, CIP calcolate; vettore lungo quanto gli atomi del record, con la fonte e il `can` della molecola su cui sono state calcolate (se non coincide, il valore non si usa). Fukui come in `FUKUI_COSA_DICONO.md`: solo indizio su S/N, mai ordinamento delle posizioni C–H.
- **Incertezza** (`regions`): quello che uno SMILES non sa dire, «OH da qualche parte sull'anello». È lo stesso contenuto di `parz/1` (Δ su un insieme di atomi), di `MZLAB_ENDPTS` (multicentro) e del `drawing` di `interp/1` (`zone`, `whole`), ma sugli indici canonici del genitore. Un TP incerto è «genitore + regione», non una struttura (livello 3 di Schymanski).

## 4. Dove vive e quanto pesa [ipotesi, con le misure di oggi]
- **Crate nuovo piccolo** (`crates/mzlab-mol`, senza dipendenze oltre `mzlab-core`): record, lettori SMILES/molfile V2000-V3000, scrittore SMILES, canonico, stereo, aromaticità. `mzlab-feat::chem::grafo` diventa una vista sul record (stessi campi), così `posiz/1`, `frag/1`, `interp/1` e `fragtree` non cambiano formato in un colpo solo.
- **Tetti** (non si alzano): `grafo` misura 199 697 B su 512 000 (`tools/build_feat_wasm.sh`): c'è posto per il crate nuovo. `chem` 551 415 su 569 344 (18 KB liberi): deve ricevere un record già pronto (atomi, legami, stereo), non i lettori né il canonico. `interp` deve restare sotto 470 000 (cancello dell'albero MSn): stessa regola, riceve il record. `finder` 512 000, `sciex` 327 680 e il motore 1 703 936 non toccati. InChI fuori da tutti (§5).
- **Interfaccia**: solo inglese in Tabui («Stereo: unknown», «Stereo: none», «Racemic», «Relative»); nessuna chiave `lang/` in Tabui; mai «formula» da sola.

## 5. InChI in WASM: fattibilità e licenza [verificato salvo dove detto]
- **Licenza**: InChI 1.07 e successive sono MIT (`LICENSE` del tag `v1.07.5` letto: «MIT License, Copyright (c) 2024 InChI Project»); compatibile con mzLab (MIT). Le versioni ≤ 1.06 avevano la licenza IUPAC/InChI-Trust.
- **Misura**: `tools/struttura_dato/inchi_wasm/build.sh` compila la libreria C ufficiale (`v1.07.5`) con `zig cc -target wasm32-wasi -Oz`: **689 873 B**, **286 401 B** con gzip -9; importa solo 6 funzioni WASI (`fd_close`, `clock_time_get`, `fd_fdstat_get`, `fd_read`, `fd_seek`, `fd_write`), facili da simulare nel worker. Su 20 molfile: 20 InChIKey uguali a RDKit; `AuxInfo /N:` dà la numerazione canonica di InChI.
- **Puro Rust**: non esiste un InChI esatto. `chematic-inchi` (MIT/Apache) si dichiara «not bit-exact» rispetto a IUPAC; il crate `inchi` lega la libreria C (non provato su wasm32). Riscrivere InChI non ha senso (§6).
- **Il sito ufficiale** (`IUPAC-InChI/InChI-Web-Demo`, MIT) compila InChI con Emscripten; dimensione del suo `.wasm` non letta.
- **Opzioni** (decide Federico): (a) modulo `inchi` a parte con un tetto suo nuovo (~700 000 B), caricato solo per esportare o per controllare l'identità di una struttura disegnata o predetta; (b) InChIKey solo dalle librerie e da `tools/` (oggi è già così: il programma non calcola InChIKey), identità nel sito solo con `can`; (c) InChI solo nell'app desktop (nativo). Consiglio (a) dopo (b): prima il record e il canonico, poi il modulo quando un caso reale lo chiede.

## 6. Cosa NON fare
- Non usare lo SMILES (nostro, di RDKit, di OpenChemLib o di Ketcher) come chiave di un dizionario, di un file o di una cache.
- Non scrivere le proprietà per atomo sugli indici dello SMILES scritto o del molfile di Ketcher: si spostano appena la stessa molecola è scritta in un altro modo.
- Non dedurre la stereo dalle coordinate dopo l'importazione; non lasciare che un molfile senza flag «either» inventi una Z/E.
- Non trattare «nessun segno di stereo» come «stereo assente»: nel dubbio è `unknown`.
- Non scrivere l'aromaticità come dato (minuscole o tipo 4) né adottare il modello di un altro toolkit: Kekulé nel record, regola nostra per il flag.
- Non scrivere un tautomero canonico nostro né un InChI nostro: si sbaglia in silenzio. Per i tautomeri basta l'InChI standard dove c'è.
- Non mettere RDKit, Open Babel, OpenChemLib o InChI dentro i `.wasm` con tetto, né Python nel sito: RDKit e Open Babel sono solo arbitri dei test in `tools/`.
- Non copiare codice GPL (patRoon, Open Babel, BioTransformer è LGPL-2.1 [da verificare]): regole e algoritmi si riscrivono da articoli e manuali, come in `patRoon_TP.md` («idee sì, codice no»).
- Non dare a un TP incerto una struttura unica: è genitore + regione, livello 3.
- Non cambiare in un colpo solo `posiz/1`, `frag/1`, `interp/1`: prima il record accanto, poi la migrazione formato per formato con le sue prove.

## 7. Prossimi passi (un pacchetto per chat)
1. **P1 record e lettori**: `crates/mzlab-mol` con `mol/1`, lettore SMILES con `@`, `/`, `\`, isotopi e CXSMILES `&`/`o`; lettore molfile V2000/V3000 con cunei e `STERAC`/`STEREL`; dati d'oro da RDKit (`tools/struttura_dato/confronto.py` diventa il generatore). Prova: le 20 molecole e `composti.csv` andata e ritorno.
2. **P2 canonico e stereogenicità** in Rust, parità con RDKit sulle 625 righe. Prova: due scritture → stesso record.
3. **P3 vista `grafo::Mol`** sul record: `posiz/1`, `frag/1`, `interp/1`, `Mol3d` senza cambiare uscite (golden invariati).
4. **P4 trasformazioni** `trasf/1` (`STRUTTURA_DATO_TRASFORMAZIONI.md`).
5. **P5 modulo InChI** se Federico sceglie l'opzione (a).

## 8. Riproduzione
- `pip install rdkit openbabel-wheel` (solo strumenti) poi `python3 tools/struttura_dato/confronto.py --json OUT.json --molfiles DIR` (tabella §1 di `_PROVE`, atteso: 20/20 `record_roundtrip`); `--composti TP_Mine/data/composti.csv` (atteso: 625 uguali, 286 con stereo, 111 non specificati).
- `node tools/struttura_dato/ocl_ordine.mjs` (ordine degli atomi di OpenChemLib; atteso `order_kept=false` solo per `explicit_H`).
- `pip install ziglang`, poi `bash tools/struttura_dato/inchi_wasm/build.sh <cartella di lavoro> DIR` (atteso: 689 873 B e 20 InChIKey uguali a quelle del JSON).
