# Confronto spettrale con le librerie: motore unico per mzFlow e pannello laterale

Studio d'architettura (sola lettura del prodotto). Ogni scelta è una raccomandazione con il suo perché; le decisioni aperte per Federico stanno in fondo (§13). Prompt dei lavori: `docs/archivio/prompt-match/{a..f}.md`.

## 1. Cosa c'è già (misurato, da non rifare)
- **Formato `.mzlib` v1** (`crates/mzlab-lib`, ~5000 righe, senza `unsafe`): record di spettro da 32 B (precursore, polarità, licenza `u8`, RT `f32` facoltativo), picchi m/z quantizzati a 1e-5 Da (`u32`) e intensità `u16` in blocchi da 512 spettri compressi zstd, metadati (10 campi: nome, addotto, CE, strumento, formula, InChIKey, SMILES, accessione, autori, licenza) in blocchi da 128, **indici Flash su frammenti e perdite neutre** (voci da 10 B in pagine da 64 KB, ordinate per m/z).
  Lettura a fette con cache LRU (64 MB, 32 MB su telefono/iPad: `read.rs`). Costruzione a memoria limitata (`build.rs`, scratch su disco). Pulizia alla Flash (`clean.rs`: precursore − 1,6 Da, rumore < 1 %, fusione 0,05 Da, ≤ 500 picchi).
- **Parser** (`parse.rs`): MSP, MGF, MassBank `.txt`, JSON MoNA/GNPS/mzMine, riconosciuti dal contenuto (`detect_format`).
- **Punteggi** (`scores.rs`, `prep.rs`): entropia spettrale (Li 2021), coseno (radice delle intensità), forward/reverse, intensità spiegata, picchi comuni, perdite neutre comuni, coseno modificato (32 picchi), coseno GNPS; contributi per picco (`contrib.rs`, `score_terms`). Tolleranza Da o ppm (`frag_tol.rs`).
- **Ricerche** (`search.rs`): `identity` (finestra del precursore, ogni candidato punteggiato), `flash_scores` (aperta / perdite neutre / ibrida sull'indice), `analogs` (ibrida nella finestra ±200 Da, poi coseno modificato ≥ 4 picchi). FDR target-decoy (`decoy.rs`).
- **wasm** `crates/mzlab-lib-wasm` (`static/wasm-lib/`, tetto 614 400 B, caricato solo all'apertura delle librerie; worker `libreria-worker.js`): `Importer`, `Lib.open/identity/best/analogs/verify`.
- **Cartella delle librerie** (`libreria-cartella.js`): app desktop = cartella nativa memory-mapped; Chrome/Edge = `showDirectoryPicker`; altrove OPFS `librerie/`. Sorgenti dell'utente mai modificate, indice in `.mzlab/<nome>.mzlib` + `.idx.json` (dimensione, data, SHA-256), stati pronta/da indicizzare/aggiornamento/danneggiata.
- **Catalogo** `librerie/2` (`tools/librerie_pacchetti.py` → `site/static/librerie/catalogo.json` + pezzi da 4 MB con SHA-256; `librerie/1` ancora accettato): la pagina scarica SOLO dal sito stesso (CSP `connect-src 'self'` invariata); `fonti` (elenco a mano `tools/librerie_catalogo.json`, `modo` site|external|link), `esterne` solo nell'app desktop con l'interruttore apposito; dettagli in `librerie.md`.
- **Flow**: `spectra.match@1` (algoritmi `spectral_entropy` predefinito, `cosine`, `modified_cosine` con `delta_formula_mda`, `tree` per MSⁿ; filtri nome/addotto/strumento/CE/RT, `decoys`, `matched_peaks`, `score_terms`) e `table.match@1` (l'MS² di ogni riga del `.mzfeat` contro una libreria; uscita `candidates/1`, query = id della riga); `annot.level@1` (cascata di Schymanski, `livelli-1.json`); `rt.predict` (Crippen/foresta, direzione dell'RT); `polpair/1` (stessa massa neutra nell'altra polarità, prova e mai unione).
- **Pagina**: «Cos'è?» (HR, `libreria-finestra.js`: master/detail con schede Candidati e Analoghi, gruppi per primo blocco InChIKey, struttura 2D, specchio con linee piene/tratteggiate/puntinate e passaggio del mouse, livello da `LIVELLI.assess`); in LR «Cerca nelle librerie…» senza livelli.
- **Chimica riusabile**: `COMMON_DELTAS` (42 modifiche) e `delta_formulas` (`mzlab-feat/src/attributio/review/matching.rs`), `regole_tp_2.json` (49 regole fotocatalisi con `delta_formula`, `direzione_rt`), `isoscore.rs` (pattern isotopico fine), `kendrick.rs` (KMD), `mzlab-mol/src/descr.rs` (TPSA, Crippen logP), RDBE in `msngrafo/candidati.rs`. NOSC: non esiste.
- **Mancano**: ricerca su PIÙ librerie con un solo risultato, raggruppamento per composto nel motore (oggi in JS), accumulatori piatti nella ricerca aperta, coseno pesato m/z (GC-EI) e RI, ΔFormula vincolata al padre, preset di librerie, tipo/tecnica della libreria, catalogo esteso.

## 2. Architettura: un solo motore
- **Raccomandazione**: nuovo modulo `mzlab_lib::libset` (insieme di librerie aperte + preset) e `mzlab_lib::matchres` (risultato `libmatch/1`), SOPRA `search.rs` esistente. Lo chiamano: `table.match`/nodo Attribution (nativo, `mzlab-cli`), il pannello laterale (wasm `mzlab-lib-wasm`, export `LibSet.search(query_json) -> libmatch/1`), l'app desktop (stesso codice nativo). *Perché*: oggi raggruppamento, ordinamento e badge stanno in `libreria.js`; spostarli in Rust dà parità pagina = Flow per costruzione.
- Il motore lavora per **query** `{prec, pol, mz[], it[], ms_level, tech, rt?, ri?, parent?}`; `parent` (formula del padre, facoltativo) abilita ΔFormula vincolata (§7).
- Il JS resta colla: apre i file, passa i byte (callback `read(offset,len)` già esistenti), disegna. Nessuna nuova dipendenza Rust.

## 3. Posto in mzFlow: modulo «Attribution»
- **Punto del grafo**: per ogni polarità, `features.table` → `table.match` → **`table.attribute@1`** (nuovo) → `annot.level` / `tp.link`. Non esiste un passo che unisce le tabelle pos+neg: le due tabelle restano separate per scelta (`polpair/1` è una prova). *Perché*: l'MS² appartiene alla riga di una polarità; l'unione pos+neg è un join di righe che `table.attribute` fa leggendo `polpair/1` come ingresso facoltativo (colonna `other_polarity_hit`).
- **Palette**: «Attribution» è un gruppo di palette nel pacchetto `attributio` (nessun pacchetto nuovo) con il preset di catena `table.match → table.attribute → annot.level`. Solo inglese come tutto il Flow.
- **`table.match@1` esteso** (non un passo nuovo): porta `library` diventa `.many()` (più `.mzlib`/MSP/MGF), parametri nuovi `library_preset` (§9), `search_mode` (`identity` | `identity+analog`), `algorithm` invariato (`spectral_entropy` predefinito; nuovi `hybrid_entropy`, `stein_cosine`). Uscita: `candidates/1` + blocco `libmatch` per query (compound-centric, §6). *Perché*: un solo passo di ricerca, già con test e CLI.
- **`table.attribute@1`** (file `flow_attribute.rs`): ingressi `table` (`.mzfeat`), `hits` (`candidates/1`), facoltativi `polpair`, `rt` (uscita di `rt.predict`), `parent` (structure). Uscita `attrib/1` (JSON per riga, letto da Tabui e dalle viste del Flow).
  Colonne aggiunte: `best_name`, `best_inchikey14`, `match_type` (exact | analog | in_silico), `score` (+ `score_kind`), `matched` (n/N), `shifted` (picchi spostati), `reverse`, `explained`, `delta_ppm`, `delta_mz`, `delta_formula` (analoghi), `n_compounds`, `library`, `confidence` (livello calcolato, §8), `rt_dir_ok` (se c'è `rt`). Un parametro per scelta: `min_score`, `min_matched`, `top_compounds` (10), `analog_max_delta` (200 Da).
- **CLI**: `--set ISTANZA.library_preset=…`, `--set ISTANZA.search_mode=…` ecc. (Regola CLI, `docs/agenti/cli.md`).

## 4. Pannello laterale MSⁿ (innesto su IDENT-LIBRERIE-UI)
- IDENT-LIBRERIE-UI (DEV) fa il pannello «Identification with the libraries»: cartella scelta dall'utente, elenco delle librerie caricate con selezione automatica pos/neg, menu di download e link. Il ramo `claude/ident-librerie*` non era ancora pubblicato alla scrittura di questo studio: i prompt d e f chiedono di leggerlo prima.
- **Comportamento raccomandato**: aprendo uno spettro MSⁿ (n ≥ 2) parte subito `LibSet.search` su tutte le librerie attive del preset con la polarità dello spettro; il pannello mostra prima i match esatti, poi gli analoghi, in < 100 ms percepiti (identità) e senza bloccare (analoghi in coda, annullati se si cambia spettro: `AbortSignal` nel worker, `cancel(token)` nel motore). Nessun pulsante «Cerca» da premere.
- MS³+: la query usa lo spettro e il precursore del livello aperto; le librerie MSⁿ (MSnLib, `msnlib/1`) passano per `spectra.match` `tree`, fuori da questo motore.
- LR (Principio 1): la ricerca resta («Cerca nelle librerie…» oggi), ma `confidence` è vuoto e nessun badge suggerisce un'identità; tolleranze del profilo (0,5 Da / 0,3 Da). HR: tolleranze del profilo dello strumento (precursore ppm, picchi 10 ppm, minimo 0,005 Da).
- Interfaccia del pannello: chiavi in `lang/it.js`/`lang/en.js` (il pannello sta nella vista Dati, non in Tabui).

## 5. Libreria: formato, cartella, rilevamento, deduplicazione
- **Formato**: `.mzlib` resta (già memory-mapped nel desktop, letto a fette nel browser). **v2** solo per tre aggiunte: (1) `ri` (`f32`, indice di Kovats) nel record con `FLAG_HAS_RI`; (2) sezione `SEC_LIBINFO` (JSON breve: `kind` experimental/in_silico, `tech` lc_msms/gc_ei/msn, `analyzers` (contatori per Orbitrap/QTOF/QqQ/IT/EI), `pol` (contatori), `source` (id del catalogo), `version`, `sha256` della sorgente); (3) `inchikey14` come `u64` (hash dei primi 14 caratteri) in una sezione per spettro, per raggruppare senza leggere i metadati.
  Il lettore v2 legge v1 (`MIN_READER` resta 1). *Perché*: niente reimportazione obbligatoria; le informazioni che servono a preset e badge stanno nel file.
- **Rilevamento automatico** (in `parse.rs`/`build.rs`, scritto in `SEC_LIBINFO`): polarità da `Ion_mode`/`IONMODE`/addotto; tecnica `gc_ei` se manca il precursore e `Spectrum_type`/`IONIZATION` = EI o c'è `RI`; `in_silico` da nome/commenti noti (LipidBlast, «in-silico», `PREDICTED`) o dal catalogo; analyzer da `Instrument_type` (parole chiave). *Perché*: l'utente non deve dichiarare nulla.
- **Cartella**: invariata (§1). Aggiornamento = nuovo SHA-256 della sorgente → «Da indicizzare»; pacchetto del catalogo con `versione` nuova → «Aggiornamento disponibile». Mai unire i file scaricati.
- **Deduplicazione senza unire**: sidecar `.mzlab/libset.bin` costruito alla prima apertura di un insieme: per ogni spettro di ogni libreria la chiave `(inchikey14, pol, round(prec, 0.001), hash dei 6 picchi più alti)`; i duplicati ricevono un `group_id` comune e una **maschera di provenienza `u16`** (bit = libreria). Il risultato mostra un composto una volta con le librerie che lo contengono. *Perché*: MassBank, MoNA e GNPS condividono molti spettri; ripeterli in Top 10 toglie posto ad altri composti.
- **Guida all'utente**: nessun paragrafo; il pannello mostra la cartella, il pulsante «Choose folder», l'elenco e il menu del catalogo (§10) con link al sito di ogni libreria «solo link».
- **Memoria a 1 milione di spettri** (browser): in RAM restano `Light` (precursore `u32` + scala `f32` = 8 MB), directory dei blocchi e delle pagine (< 2 MB), accumulatori piatti della ricerca aperta (`f32` + `u16` = 6 MB), cache LRU 64 MB (32 MB su telefono), maschera e `group_id` (6 MB). Totale ≈ 90 MB: ammesso su desktop, al limite su iPad; su telefono il pannello non esiste (`telefono.js`). Su disco: ≈ 320 B per spettro sintetico da 22 picchi (320 MB per 1 milione; misura reale su MassBank/GNPS nel prompt a).

## 6. Motore, indice e misure wasm
Banco (scratch, non committato): libreria sintetica deterministica di `identity_bench.rs` (precursori 100–900, 5–40 picchi da un pool di 400 frammenti), 200 query (50 per le ricerche aperte), stessa macchina (4 CPU), Node 22 per il wasm (`wasm32-unknown-unknown`, `opt-level 3`, LTO), libreria intera in memoria.

| ricerca (ms per query) | 100k unit | 100k HR | 1M unit nativo | 1M unit wasm | 1M HR nativo | 1M HR wasm |
|---|---|---|---|---|---|---|
| `identity` (finestra precursore, top 10) | 0,13 / wasm 0,31 | 0,013 / wasm 0,03 | 1,4 | 2,5–3,3 | 0,04 | 0,12–0,15 |
| `analogs` (ibrida ±200 Da + coseno mod.) | 3,4 / wasm 5,2 | 2,7 / wasm 4,1 | 45 | 66 | 27 | 35 |
| ibrida aperta su tutto l'indice (`flash_scores`) | 6,9 / wasm 9,1 | 5,2 / wasm 7,0 | 106 | 140 | 60 | 81 |
| prototipo con accumulatori piatti (stessa ricerca) | – | – | 21 | 33 | 11 | 17 |

- **Lettura**: l'identità rispetta già l'obiettivo < 5 ms per spettro su 1 milione anche in wasm. La ricerca aperta/ibrida no: `flash_scores` raccoglie le coppie in un `Vec` e le ordina (`score_pairs`); gli accumulatori piatti (`f32` per spettro + elenco dei toccati, risolvendo i conflitti uno-a-uno solo per i toccati con conflitto) danno ×4–5. Il pool sintetico è patologico (ogni frammento compare in migliaia di spettri): su librerie reali le voci visitate sono molte meno; il prompt a misura MassBank (≈ 100k) e GNPS. Costruzione: 1M spettri in 10–13 s nativo, 15–17 s wasm.
- **Raccomandazioni**: (1) `flash_scores` con accumulatori piatti, a parità di risultato con la versione a coppie (test di parità bit per bit sui punteggi); (2) l'ibrida limitata dalla finestra `analog_max_delta` come oggi (le ricerche aperte su tutto l'indice solo su richiesta); (3) analoghi in coda nel pannello (35–66 ms tollerabili se non bloccano); (4) multi-libreria: ogni libreria è cercata a turno con gli stessi accumulatori dimensionati sulla più grande, poi fusione dei top per punteggio (nessun indice globale: *perché* le librerie si aggiornano una alla volta).
- Obiettivo di accettazione: identità ≤ 5 ms e analoghi ≤ 50 ms per query in wasm su 1M sintetico; tempi reali in `docs/agenti/misure-rust.md`.

## 7. Algoritmi per tecnica
| tecnica | predefinito | perché | esiste? |
|---|---|---|---|
| LC-MS/MS identità | entropia spettrale (Li 2021), pulizia Flash | più robusta del coseno su spettri rumorosi; già `spectral_entropy` | sì |
| analoghi / TP | **entropia ibrida** (picchi diretti + spostati di Δprec, Flash hybrid) per i candidati, poi coseno modificato per ordinare + ΔFormula | la ricerca ibrida è nell'indice; il coseno modificato riproduce GNPS e dà i picchi spostati per lo specchio | sì (`analogs`); manca il punteggio ibrido esposto |
| riproduzione GNPS | coseno GNPS (`gnps_cosine`) | confronto con i risultati GNPS/FBMN | sì |
| GC-EI | coseno pesato di Stein, pesi I^a·(m/z)^b con forward e reverse; ΔRI gaussiano (σ predefinito 10 unità RI) moltiplicato al punteggio | standard NIST; il precursore non c'è: ricerca aperta sui frammenti o finestra sul picco più alto | no: nuovo `stein_cosine` in `scores.rs` |
| MSⁿ | `tree` (`msngrafo/cerca.rs`) | alberi, non spettri singoli | sì, fuori dal motore |

- Pesi Stein: in letteratura NIST usa a = 0,53, b = 1,3 (Kim et al. 2012) e Stein & Scott 1994 m/z³·I^0,6 per la componente composita; il prompt b verifica la fonte prima di fissare i default (proposta di Federico: a = 0,5, b = 1,3).
- **Fuori**: Spec2Vec e MS2DeepScore (modelli addestrati da decine di MB, inferenza ONNX/TensorFlow: nuove dipendenze pesanti e niente parità semplice); da riconsiderare solo come ordinamento secondario nell'app desktop.

## 8. Risultato `libmatch/1`
- Per query: `query` (id riga o scansione), `params` (tolleranze, algoritmo, preset), `compounds[]` ordinati, al massimo `top_compounds` (10). Un composto = primo blocco InChIKey (o nome normalizzato se manca): `inchikey14`, `name`, `formula`, `smiles`, `badge` (exact | analog | in_silico), `best` (lo spettro migliore), `spectra[]` (tutti gli spettri del composto con punteggio, espandibili), `libraries` (maschera e nomi).
- Per spettro: `library`, `accession`, `score` e `score_kind`, `reverse`, `explained` (intensità della query spiegata), `matched` n e `of` N (picchi della query), `direct` e `shifted` (coppie dirette e spostate), `delta_ppm` del precursore, `delta_mz`, per gli analoghi `delta_formula[]` (§9), `adduct`, `ce`, `instrument`, `licence`, `pairs[]` (query_mz, library_mz, kind direct|shifted|loss: lo specchio disegna da qui, già nel formato di `matched_peaks`).
- `confidence`: livello calcolato da regole (Schymanski/MSI), mai deciso: nel Flow lo dà `annot.level` (cascata Rust esistente, `livelli-1.json`); nel pannello la stessa cascata. Raccomandazione: esporre la cascata di `mzlab-feat` come funzione pura `level_of(evidence)` richiamabile dal wasm delle librerie solo se il tetto 614 400 B lo regge; altrimenti il pannello continua con `LIVELLI.assess` (stesse regole, `livelli-1.js`) finché non migra (decisione 2). Esatto sperimentale HR → 2a; analogo → 3b; in silico → mai 2a; LR → vuoto.
- Badge e colonne neutre per segnalazioni (es. `chimeric` dal passo `spectra.chimeric`): nessun testo d'avviso.

## 9. Analoghi: ΔFormula e modifiche note
- **Tabella unica** `mzlab-core/data/modifiche-1.json` (spostata da `COMMON_DELTAS`, che diventa un lettore): voci `{id, label, delta_formula, mass, kind}` con almeno +O 15,9949, ±CH2 14,0157, −2H 2,0157 (−H2), ±H2O 18,0106, −Cl+H −33,9610, −Cl+OH −19,9720, +CF2 49,9968, +SO3 79,9568, +glucuronide 176,0321, + le 42 esistenti e le `delta_formula` di `regole_tp_2.json` (una sola fonte; *perché* oggi Attributio e Tabui hanno elenchi separati).
- **Decomposizione vincolata al padre**: con la formula del padre (porta `parent`, o del composto di libreria per gli analoghi) le proposte che tolgono un elemento assente o più atomi di quelli presenti si scartano (non si perde Cl se il padre non lo ha); ordine: modifiche note entro `delta_formula_mda` (5 mDa HR), poi combinazioni di due modifiche note, poi ricerca libera CHNOPS(+F,Cl,Br) entro la stessa tolleranza, la più semplice prima, al massimo 3. In LR nessuna ΔFormula (massa nominale).
- I picchi spostati del coseno modificato (Δ = prec_query − prec_lib) sono la prova strutturale dell'analogo: lo specchio li disegna tratteggiati come oggi.

## 10. Kit di proprietà attese (solo sul candidato)
- Da formula: RDBE (esiste), NOSC (nuovo, 10 righe in `mzlab-core`), KMD CH2 e CF2 (`kendrick.rs`), pattern isotopico atteso contro osservato (`isoscore.rs`). Da struttura: TPSA e Crippen logP (`mzlab-mol/descr.rs`).
- ΔRT contro polarità: lavoro in comune con il modulo TP. Innesto: `rt.predict` (già passo) dà `rt_ok` e direzione; `regole_tp_2.json` dà `direzione_rt` per regola; `table.attribute` legge l'uscita di `rt.predict` e scrive `rt_dir_ok` (RPLC: logP minore → RT minore; HILIC: inverso, parametro `chrom_mode`). Nessuna nuova logica di RT nel motore spettrale.
- Le proprietà sono colonne dell'uscita, mai filtri predefiniti.

## 11. Catalogo e preset
- **Catalogo** `librerie/1` esteso in `librerie/2` (stesso file `catalogo.json`, generato da `tools/librerie_pacchetti.py` più un elenco versionato `tools/librerie_catalogo.json` scritto a mano): per voce `id`, `nome`, `url` (pagina), `download` (file), `licenza` (SPDX), `formato`, `dimensione`, `sha256` o null, `kind`, `tech`, `pol`, `doi`, `verificato` (data della verifica con fetch reale), `modo` (`site` = pezzi sul sito, solo licenze che permettono la ridistribuzione; `external` = solo app desktop; `link` = solo collegamento al sito).
- Il browser scarica solo `site` (CSP invariata): l'esito della verifica CSP/CORS di IDENT-LIBRERIE-UI decide se un `external` può passare anche nel browser; finché non arriva, `external` resta solo desktop.
- Candidate (da Gemini, NON verificate: licenze, dimensioni, DOI e URL li verifica il prompt f): `site`/`external`: MassBank (già), MS-DIAL MSP (LC pos/neg, GC-EI con RI), GNPS ALL_GNPS_NO_PROPOGATED (CC0), MoNA (sottoinsiemi), NIST DIMSpec PFAS, MultiMS2 (CC0), EMBL-MCF 2.0 (demo), LipidBlast 2022 (in silico), NIST Glycan. `link`: MSnLib, GMD Golm, ENFSI/SWGDRUG, PeptideAtlas, ProteomeTools, ISDB/LOTUS. Escluse: NIST commerciale, mzCloud, METLIN a pagamento. Menu base: MassBank + MS-DIAL + GNPS/MoNA; gruppo «Specialistiche / in silico» chiuso.
- **Preset** (`mzlab-lib/data/preset-1.json`): vista filtrata, non copia: `experimental` (kind experimental), `environmental` (MassBank, DIMSpec PFAS, MultiMS2, MS-DIAL), `metabolomics` (MS-DIAL, MoNA, GNPS, LipidBlast), `natural_products` (GNPS, ISDB se presente), `forensic` (SWGDRUG se presente); polarità e analyzer dallo spettro, automatici. Il preset si traduce in una maschera `u16` sulle librerie aperte.

## 12. Ordine dei lavori e dipendenze
| prompt | lavoro | dipende da | file principali |
|---|---|---|---|
| a | `.mzlib` v2 (RI, `SEC_LIBINFO`, `inchikey14`), rilevamento, `libset` + dedup + maschera, accumulatori piatti, banco | – | `crates/mzlab-lib/src/{format,build,parse,read,search,libset}.rs` |
| b | `stein_cosine`, `hybrid_entropy` esposto, `matchres` → `libmatch/1` compound-centric, test sintetici | a | `scores.rs`, `matchres.rs` |
| c | nodo Attribution: `table.match` multi-libreria + `table.attribute@1` + CLI | b | `flow_attribute.rs` in `crates/mzlab-cli/src/`, `mzlab-feat/src/steps/` |
| d | wasm `LibSet.search` + pannello laterale (su IDENT-LIBRERIE-UI) | b, pannello IDENT | `crates/mzlab-lib-wasm`, `libreria-worker.js` |
| e | analoghi: `modifiche-1.json`, ΔFormula vincolata, kit proprietà | b (c, d per le colonne) | `mzlab-core`, `matching.rs` |
| f | catalogo `librerie/2`, verifica URL/licenze, preset | a | `tools/librerie_*`, `libreria-scarica.js`, `scarica.rs` |

a e f possono partire insieme; c, d, e dopo b; d attende anche la PR di IDENT-LIBRERIE-UI.

## 13. Decisioni aperte per Federico
1. **Deduplicazione**: *consigliata* sidecar `libset.bin` con maschera `u16` e file separati; alternative: (b) nessuna deduplicazione (Top 10 con doppioni); (c) un `union.mzlib` per preset con provenienza per spettro (più veloce, ma duplica i dati su disco).
2. **Livello nel pannello**: *consigliata* cascata Rust nel wasm delle librerie se sta nel tetto; alternative: (b) resta `LIVELLI.assess` in JS; (c) alzare il tetto del wasm librerie.
3. **Ricerca aperta su tutto l'indice** (oltre ±200 Da): *consigliata* solo su richiesta (voce «Open search»); alternative: (b) sempre nel Flow, mai nel pannello; (c) mai.
4. **GC-EI**: *consigliata* nel prompt b (Stein + ΔRI) ma senza interfaccia finché non serve; alternative: (b) rinviare tutto; (c) farlo completo subito con il pannello.
5. **Download esterni nel browser**: *consigliata* solo pacchetti ridistribuiti dal sito (licenze libere); alternative: (b) anche URL esterni se la CSP/CORS verificata lo permette (CSP da allargare); (c) solo link, nessun download nel browser.
6. **Analoghi in LR**: *consigliata* niente analoghi né ΔFormula a risoluzione unitaria (Principio 1 e massa nominale); alternative: (b) analoghi senza ΔFormula; (c) come in HR.
