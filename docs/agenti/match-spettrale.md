# Confronto spettrale con le librerie: motore unico per mzFlow e pannello laterale

Stato del motore di confronto spettrale e lavori aperti (§12); ogni scelta porta il suo perché; le decisioni aperte stanno in fondo (§13).

## 1. Cosa c'è (misurato)
- **Formato `.mzlib` v1** (`crates/mzlab-lib`, ~5000 righe, senza `unsafe`): record di spettro da 32 B (precursore, polarità, licenza `u8`, RT `f32` facoltativo), picchi m/z quantizzati a 1e-5 Da (`u32`) e intensità `u16` in blocchi da 512 spettri compressi zstd, metadati (10 campi: nome, addotto, CE, strumento, formula, InChIKey, SMILES, accessione, autori, licenza) in blocchi da 128, **indici Flash su frammenti e perdite neutre** (voci da 10 B in pagine da 64 KB, ordinate per m/z).
  Lettura a fette con cache LRU (64 MB, 32 MB su telefono/iPad: `read.rs`); il dispositivo lo dice la pagina nel messaggio `config` (`libreria-corpo.js`, regola di `rust-budget.js` `device()`), perché nel worker un iPad con lo user agent del Mac non ha `maxTouchPoints`. Costruzione a memoria limitata (`build.rs`, scratch su disco). Pulizia alla Flash (`clean.rs`: precursore − 1,6 Da, rumore < 1 %, fusione 0,05 Da, ≤ 500 picchi).
- **Parser** (`parse.rs`): MSP, MGF, MassBank `.txt`, JSON MoNA/GNPS/mzMine, riconosciuti dal contenuto (`detect_format`).
- **Punteggi** (`scores.rs`, `prep.rs`): entropia spettrale (Li 2021), coseno (radice delle intensità), forward/reverse, intensità spiegata, picchi comuni, perdite neutre comuni, coseno modificato (32 picchi), coseno GNPS; contributi per picco (`contrib.rs`, `score_terms`). Tolleranza Da o ppm (`frag_tol.rs`).
- **Ricerche** (`search.rs`): `identity` (finestra del precursore, ogni candidato punteggiato), `flash_scores` (aperta / perdite neutre / ibrida sull'indice), `analogs` (ibrida nella finestra ±200 Da, poi coseno modificato ≥ 4 picchi). FDR target-decoy (`decoy.rs`).
- **wasm** `crates/mzlab-lib-wasm` (`static/wasm-lib/`, tetto 614 400 B, caricato solo all'apertura delle librerie; worker `libreria-worker.js`): `Importer`, `Lib.open/identity/best/analogs/verify`.
- **Cartella delle librerie** (`libreria-cartella.js`): app desktop = cartella nativa memory-mapped; Chrome/Edge = `showDirectoryPicker`; altrove OPFS `librerie/`. Sorgenti dell'utente mai modificate, indice in `.mzlab/<nome>.mzlib` + `.idx.json` (dimensione, data, SHA-256), stati pronta/da indicizzare/aggiornamento/danneggiata.
- **Catalogo** `librerie/2` (`tools/librerie_pacchetti.py` → `site/static/librerie/catalogo.json` + pezzi da 4 MB con SHA-256; `librerie/1` ancora accettato): la pagina scarica SOLO dal sito stesso (CSP `connect-src 'self'` invariata); `fonti` (elenco a mano `tools/librerie_catalogo.json`, `modo` site|external|link), `esterne` solo nell'app desktop con l'interruttore apposito; dettagli in `librerie.md`.
- **Flow**: `spectra.match@1` (algoritmi `spectral_entropy` predefinito, `cosine`, `modified_cosine` con `delta_formula_mda`, `tree` per MSⁿ; filtri nome/addotto/strumento/CE/RT, `decoys`, `matched_peaks`, `score_terms`) e `table.match@1` (l'MS² di ogni riga del `.mzfeat` contro una libreria; uscita `candidates/1`, query = id della riga); `annot.level@1` (cascata di Schymanski, `livelli-1.json`); `rt.predict` (Crippen/foresta, direzione dell'RT); `polpair/1` (stessa massa neutra nell'altra polarità, prova e mai unione).
- **Pagina**: «Cos'è?» (HR, `libreria-finestra.js`: master/detail con schede Candidati e Analoghi, gruppi per primo blocco InChIKey, struttura 2D, specchio con linee piene/tratteggiate/puntinate e passaggio del mouse, livello da `LIVELLI.assess`); in LR «Cerca nelle librerie…» senza livelli.
- **Chimica riusabile**: `COMMON_DELTAS` (42 modifiche) e `delta_formulas` (`mzlab-feat/src/attributio/review/matching.rs`), `regole_tp_2.json` (49 regole fotocatalisi con `delta_formula`, `direzione_rt`), `isoscore.rs` (pattern isotopico fine), `kendrick.rs` (KMD), `mzlab-mol/src/descr.rs` (TPSA, Crippen logP), RDBE in `msngraph/candidates.rs`. NOSC: non esiste.
- **Motore multi-libreria** (`crates/mzlab-lib`): `libset.rs` (fino a 16 librerie, una maschera per libreria, accumulatori condivisi, fusione per punteggio, sidecar dei gruppi di duplicati), `matchres.rs` (`libmatch/1`, `docs/schemi/libmatch-1.json`), `preset.rs` (`data/preset-1.json`), `libinfo.rs` (`SEC_LIBINFO`, hash `inchikey14`), `stein.rs` (`stein_cosine`, `ri_factor`, `hybrid_entropy`), accumulatori piatti in `flash.rs` (`FlashAcc`). Banco `examples/libset_bench.rs`, misure in `misure-rust.md`.
- **Manca**: `LibSet` nel wasm e nel pannello, `table.match` multi-libreria, `table.attribute@1`, ΔFormula vincolata al padre, `modifiche-1.json`, kit di proprietà (§12).

## 2. Architettura: un solo motore
- `mzlab_lib::libset` (insieme di librerie aperte + preset) e `mzlab_lib::matchres` (risultato `libmatch/1`) stanno SOPRA `search.rs`. Li usano solo i test di `mzlab-lib`; devono chiamarli `table.match`/nodo Attribution (nativo, `mzlab-cli`), il pannello laterale (wasm `mzlab-lib-wasm`, export `LibSet.search(query_json) -> libmatch/1`) e l'app desktop (stesso codice nativo). *Perché*: raggruppamento, ordinamento e badge del pannello stanno ancora in `libreria.js`; in Rust danno parità pagina = Flow per costruzione.
- Il motore lavora per **query** `{prec, pol, mz[], it[], ms_level, tech, rt?, ri?, parent?}`; `parent` (formula del padre, facoltativo) abilita ΔFormula vincolata (§7).
- Il JS resta colla: apre i file, passa i byte (callback `read(offset,len)` già esistenti), disegna. Nessuna nuova dipendenza Rust.

## 3. Posto in mzFlow: modulo «Attribution»
- **Punto del grafo**: per ogni polarità, `features.table` → `table.match` → **`table.attribute@1`** (da fare) → `annot.level` / `tp.link`. Non esiste un passo che unisce le tabelle pos+neg: le due tabelle restano separate per scelta (`polpair/1` è una prova). *Perché*: l'MS² appartiene alla riga di una polarità; l'unione pos+neg è un join di righe che `table.attribute` fa leggendo `polpair/1` come ingresso facoltativo (colonna `other_polarity_hit`).
- **Palette**: «Attribution» è un gruppo di palette nel pacchetto `attributio` (nessun pacchetto nuovo) con il preset di catena `table.match → table.attribute → annot.level`. Solo inglese come tutto il Flow.
- **`table.match@1` esteso** (da fare, non un passo nuovo; ha una sola libreria e `spectral_entropy`/`cosine`/`modified_cosine`): porta `library` diventa `.many()` (più `.mzlib`/MSP/MGF), parametri nuovi `library_preset` (§9), `search_mode` (`identity` | `identity+analog`), `algorithm` invariato (`spectral_entropy` predefinito; nuovi `hybrid_entropy`, `stein_cosine`). Uscita: `candidates/1` + blocco `libmatch` per query (compound-centric, §6). *Perché*: un solo passo di ricerca, già con test e CLI.
- **`table.attribute@1`** (da fare, file `flow_attribute.rs`): ingressi `table` (`.mzfeat`), `hits` (`candidates/1`), facoltativi `polpair`, `rt` (uscita di `rt.predict`), `parent` (structure). Uscita `attrib/1` (JSON per riga, letto da Tabui e dalle viste del Flow).
  Colonne aggiunte: `best_name`, `best_inchikey14`, `match_type` (exact | analog | in_silico), `score` (+ `score_kind`), `matched` (n/N), `shifted` (picchi spostati), `reverse`, `explained`, `delta_ppm`, `delta_mz`, `delta_formula` (analoghi), `n_compounds`, `library`, `confidence` (livello calcolato, §8), `rt_dir_ok` (se c'è `rt`). Un parametro per scelta: `min_score`, `min_matched`, `top_compounds` (10), `analog_max_delta` (200 Da).
- **CLI**: `--set ISTANZA.library_preset=…`, `--set ISTANZA.search_mode=…` ecc. (Regola CLI, `docs/agenti/cli.md`).

## 4. Pannello laterale MSⁿ
- Il pannello «Identification with the libraries» (`ident-pannello.js`, scheda della barra informazioni): spettro MS² della cella attiva, librerie caricate con selezione automatica delle compatibili, «Load file…» / «Load folder…», menu del catalogo (`ident-catalogo.js`), tabella dei candidati. La ricerca passa da `LIB.call("search", {ids})` nel worker, non ancora da `LibSet`.
- **Comportamento da costruire**: aprendo uno spettro MSⁿ (n ≥ 2) parte subito `LibSet.search` su tutte le librerie attive del preset con la polarità dello spettro; il pannello mostra prima i match esatti, poi gli analoghi, in < 100 ms percepiti (identità) e senza bloccare (analoghi in coda, annullati se si cambia spettro: `AbortSignal` nel worker, `cancel(token)` nel motore). Nessun pulsante «Cerca» da premere.
- MS³+: la query usa lo spettro e il precursore del livello aperto; le librerie MSⁿ (MSnLib, `msnlib/1`) passano per `spectra.match` `tree`, fuori da questo motore.
- LR (Principio 1): la ricerca resta («Cerca nelle librerie…»), ma `confidence` è vuoto e nessun badge suggerisce un'identità; tolleranze del profilo (0,5 Da / 0,3 Da). HR: tolleranze del profilo dello strumento (precursore ppm, picchi 10 ppm, minimo 0,005 Da).
- Interfaccia del pannello: chiavi in `lang/it.js`/`lang/en.js` (il pannello sta nella vista Dati, non in Tabui).

## 5. Libreria: formato, cartella, rilevamento, deduplicazione
- **Formato**: `.mzlib` resta (già memory-mapped nel desktop, letto a fette nel browser). **v2** solo per tre aggiunte: (1) `ri` (`f32`, indice di Kovats) nel record con `FLAG_HAS_RI`; (2) sezione `SEC_LIBINFO` (JSON breve: `kind` experimental/in_silico, `tech` lc_msms/gc_ei/msn, `analyzers` (contatori per Orbitrap/QTOF/QqQ/IT/EI), `pol` (contatori), `source` (id del catalogo), `version`, `sha256` della sorgente); (3) `inchikey14` come `u64` (hash dei primi 14 caratteri) in una sezione per spettro, per raggruppare senza leggere i metadati.
  Il lettore v2 legge v1 (`MIN_READER` resta 1). *Perché*: niente reimportazione obbligatoria; le informazioni che servono a preset e badge stanno nel file.
- **Rilevamento automatico** (in `parse.rs`/`build.rs`, scritto in `SEC_LIBINFO`): polarità da `Ion_mode`/`IONMODE`/addotto; tecnica `gc_ei` se manca il precursore e `Spectrum_type`/`IONIZATION` = EI o c'è `RI`; `in_silico` da nome/commenti noti (LipidBlast, «in-silico», `PREDICTED`) o dal catalogo; analyzer da `Instrument_type` (parole chiave). *Perché*: l'utente non deve dichiarare nulla.
- **Cartella**: invariata (§1). Aggiornamento = nuovo SHA-256 della sorgente → «Da indicizzare»; pacchetto del catalogo con `versione` nuova → «Aggiornamento disponibile». Mai unire i file scaricati.
- **Deduplicazione senza unire** (`libset.rs`): i file restano separati, con una maschera `u16` (bit = libreria) e un sidecar (`MZLS`) costruito una volta per combinazione di file: per ogni spettro di ogni libreria la chiave `(inchikey14, pol, round(prec, 0.001), hash dei 6 picchi più alti)`; i duplicati ricevono un `group_id` comune. Il risultato mostra un composto una volta con le librerie che lo contengono. *Perché*: MassBank, MoNA e GNPS condividono molti spettri; ripeterli in Top 10 toglie posto ad altri composti.
- **Guida all'utente**: nessun paragrafo; il pannello mostra la cartella, il pulsante «Choose folder», l'elenco e il menu del catalogo (§10) con link al sito di ogni libreria «solo link».
- **Memoria a 1 milione di spettri** (browser): in RAM restano `Light` (precursore `u32` + scala `f32` = 8 MB), directory dei blocchi e delle pagine (< 2 MB), accumulatori piatti della ricerca aperta (`f32` + `u16` = 6 MB), cache LRU 64 MB (32 MB su telefono), maschera e `group_id` (6 MB). Totale ≈ 90 MB: ammesso su desktop, al limite su iPad; su telefono il pannello non esiste (`telefono.js`). Su disco: ≈ 320 B per spettro sintetico da 22 picchi (320 MB per 1 milione); misura su MassBank/GNPS reali da fare.

## 6. Motore, indice e misure wasm
Banco (scratch, non committato): libreria sintetica deterministica di `identity_bench.rs` (precursori 100–900, 5–40 picchi da un pool di 400 frammenti), 200 query (50 per le ricerche aperte), stessa macchina (4 CPU), Node 22 per il wasm (`wasm32-unknown-unknown`, `opt-level 3`, LTO), libreria intera in memoria.

| ricerca (ms per query) | 100k unit | 100k HR | 1M unit nativo | 1M unit wasm | 1M HR nativo | 1M HR wasm |
|---|---|---|---|---|---|---|
| `identity` (finestra precursore, top 10) | 0,13 / wasm 0,31 | 0,013 / wasm 0,03 | 1,4 | 2,5–3,3 | 0,04 | 0,12–0,15 |
| `analogs` (ibrida ±200 Da + coseno mod.) | 3,4 / wasm 5,2 | 2,7 / wasm 4,1 | 45 | 66 | 27 | 35 |
| ibrida aperta su tutto l'indice (`flash_scores`) | 6,9 / wasm 9,1 | 5,2 / wasm 7,0 | 106 | 140 | 60 | 81 |
| prototipo con accumulatori piatti (stessa ricerca) | – | – | 21 | 33 | 11 | 17 |

- **Lettura**: l'identità rispetta l'obiettivo < 5 ms per spettro su 1 milione anche in wasm. La ricerca aperta/ibrida usa gli accumulatori piatti (`FlashAcc`: `f32` per spettro + elenco dei toccati, conflitti uno-a-uno risolti solo per i toccati con conflitto, `score_pairs` per quelli): ×2–5 sulla versione a coppie. Il pool sintetico è patologico (ogni frammento compare in migliaia di spettri): su librerie reali le voci visitate sono molte meno (MassBank ≈ 100k e GNPS da misurare). Costruzione: 1M spettri in 10–13 s nativo, 15–17 s wasm.
- **Regole**: (1) accumulatori piatti a parità di risultato con la versione a coppie (parità bit per bit sui punteggi); (2) l'ibrida limitata dalla finestra `analog_max_delta` (le ricerche aperte su tutto l'indice solo su richiesta); (3) analoghi in coda nel pannello (35–66 ms tollerabili se non bloccano); (4) multi-libreria (`libset`): ogni libreria è cercata a turno con gli stessi accumulatori, poi fusione dei top per punteggio (nessun indice globale: *perché* le librerie si aggiornano una alla volta).
- Obiettivo di accettazione: identità ≤ 5 ms e analoghi ≤ 50 ms per query in wasm su 1M sintetico; tempi reali in `docs/agenti/misure-rust.md`.

## 7. Algoritmi per tecnica
| tecnica | predefinito | perché | esiste? |
|---|---|---|---|
| LC-MS/MS identità | entropia spettrale (Li 2021), pulizia Flash | più robusta del coseno su spettri rumorosi; già `spectral_entropy` | sì |
| analoghi / TP | **entropia ibrida** (picchi diretti + spostati di Δprec, Flash hybrid) per i candidati, poi coseno modificato per ordinare + ΔFormula | la ricerca ibrida è nell'indice; il coseno modificato riproduce GNPS e dà i picchi spostati per lo specchio | sì (`analogs`, `hybrid_entropy`) |
| riproduzione GNPS | coseno GNPS (`gnps_cosine`) | confronto con i risultati GNPS/FBMN | sì |
| GC-EI | coseno pesato di Stein, pesi I^a·(m/z)^b con forward e reverse; ΔRI gaussiano (σ predefinito 10 unità RI) moltiplicato al punteggio | standard NIST; il precursore non c'è: ricerca aperta sui frammenti o finestra sul picco più alto | nel motore (`stein_cosine`, `ri_factor` in `stein.rs`), senza interfaccia |
| MSⁿ | `tree` (`msngraph/search.rs`) | alberi, non spettri singoli | sì, fuori dal motore |

- Pesi Stein: predefiniti a = 0,53, b = 1,3 (Kim et al. 2012, Bioinformatics 28:1158), σ RI 10; Stein & Scott 1994 usavano I^0,6·m/z³. Tutti parametri di `SteinOpts`.
- **Fuori**: Spec2Vec e MS2DeepScore (modelli addestrati da decine di MB, inferenza ONNX/TensorFlow: nuove dipendenze pesanti e niente parità semplice); da riconsiderare solo come ordinamento secondario nell'app desktop.

## 8. Risultato `libmatch/1`
- Per query: `query` (id riga o scansione), `params` (tolleranze, algoritmo, preset), `compounds[]` ordinati, al massimo `top_compounds` (10). Un composto = primo blocco InChIKey (o nome normalizzato se manca): `inchikey14`, `name`, `formula`, `smiles`, `badge` (exact | analog | in_silico), `best` (lo spettro migliore), `spectra[]` (tutti gli spettri del composto con punteggio, espandibili), `libraries` (maschera e nomi).
- Per spettro: `library`, `accession`, `score` e `score_kind`, `reverse`, `explained` (intensità della query spiegata), `matched` n e `of` N (picchi della query), `direct` e `shifted` (coppie dirette e spostate), `delta_ppm` del precursore, `delta_mz`, per gli analoghi `delta_formula[]` (§9), `adduct`, `ce`, `instrument`, `licence`, `pairs[]` (query_mz, library_mz, kind direct|shifted|loss: lo specchio disegna da qui, già nel formato di `matched_peaks`).
- `confidence`: livello calcolato da regole (Schymanski/MSI), mai deciso; `matchres` lo lascia null. Nel Flow lo dà `annot.level` (cascata Rust, `livelli-1.json`); nel pannello lo calcola il JS (`LIVELLI.assess` in `mzlab/web/libreria.js`, stesse regole, `livelli-1.js`). Esporre la cascata di `mzlab-feat` come funzione pura `level_of(evidence)` nel wasm delle librerie regge solo se il tetto 614 400 B lo permette: decisione 1 (§13). Esatto sperimentale HR → 2a; analogo → 3b; in silico → mai 2a; LR → vuoto.
- Badge e colonne neutre per segnalazioni (es. `chimeric` dal passo `spectra.chimeric`): nessun testo d'avviso.

## 9. Analoghi: ΔFormula e modifiche note
- **Tabella unica** (da fare) `mzlab-core/data/modifiche-1.json` (spostata da `COMMON_DELTAS`, che diventa un lettore): voci `{id, label, delta_formula, mass, kind}` con almeno +O 15,9949, ±CH2 14,0157, −2H 2,0157 (−H2), ±H2O 18,0106, −Cl+H −33,9610, −Cl+OH −19,9720, +CF2 49,9968, +SO3 79,9568, +glucuronide 176,0321, + le 42 esistenti e le `delta_formula` di `regole_tp_2.json` (una sola fonte; *perché* Attributio e Tabui hanno elenchi separati).
- **Decomposizione vincolata al padre** (da fare): con la formula del padre (porta `parent`, o del composto di libreria per gli analoghi) le proposte che tolgono un elemento assente o più atomi di quelli presenti si scartano (non si perde Cl se il padre non lo ha); ordine: modifiche note entro `delta_formula_mda` (5 mDa HR), poi combinazioni di due modifiche note, poi ricerca libera CHNOPS(+F,Cl,Br) entro la stessa tolleranza, la più semplice prima, al massimo 3. In LR nessuna ΔFormula (massa nominale).
- I picchi spostati del coseno modificato (Δ = prec_query − prec_lib) sono la prova strutturale dell'analogo: lo specchio li disegna tratteggiati.

## 10. Kit di proprietà attese (solo sul candidato, da fare)
- Da formula: RDBE (esiste), NOSC (da scrivere, 10 righe in `mzlab-core`), KMD CH2 e CF2 (`kendrick.rs`), pattern isotopico atteso contro osservato (`isoscore.rs`). Da struttura: TPSA e Crippen logP (`mzlab-mol/descr.rs`).
- ΔRT contro polarità: lavoro in comune con il modulo TP. Innesto: `rt.predict` (già passo) dà `rt_ok` e direzione; `regole_tp_2.json` dà `direzione_rt` per regola; `table.attribute` legge l'uscita di `rt.predict` e scrive `rt_dir_ok` (RPLC: logP minore → RT minore; HILIC: inverso, parametro `chrom_mode`). Nessuna nuova logica di RT nel motore spettrale.
- Le proprietà sono colonne dell'uscita, mai filtri predefiniti.

## 11. Catalogo e preset
- **Catalogo** `librerie/2` (`catalogo.json`, generato da `tools/librerie_pacchetti.py` più l'elenco versionato `tools/librerie_catalogo.json` scritto a mano, schema controllato da `--check-catalog`, dettagli in `librerie.md`): per voce `id`, `nome`, `url` (pagina), `download` (file), `licenza` (SPDX), `formato`, `dimensione`, `sha256` o null, `kind`, `tech`, `pol`, `doi`, `verificato` (data della verifica con fetch reale), `modo` (`site` = pezzi sul sito, solo licenze che permettono la ridistribuzione; `external` = solo app desktop; `link` = solo collegamento al sito).
- Il browser scarica solo `site` (CSP invariata); `external` resta solo desktop (decisione 4, §13).
- Menu: gruppo base (MassBank, MS-DIAL, GNPS/MoNA) e «Specialistiche / in silico» chiuso; voci `link` per le librerie non ridistribuibili; escluse NIST commerciale, mzCloud, METLIN a pagamento. Licenze, dimensioni e verifiche stanno in `tools/librerie_catalogo.json`.
- **Preset** (`mzlab-lib/data/preset-1.json`, `preset.rs`): vista filtrata, non copia: `experimental` (kind experimental), `environmental` (MassBank, DIMSpec PFAS, MultiMS2, MS-DIAL), `metabolomics` (MS-DIAL, MoNA, GNPS, LipidBlast), `natural_products` (GNPS, ISDB se presente), `forensic` (SWGDRUG se presente); polarità e analyzer dallo spettro, automatici. Il preset si traduce in una maschera `u16` sulle librerie aperte.

## 12. Lavori aperti
| lavoro | dipende da | file principali |
|---|---|---|
| nodo Attribution: `table.match` multi-libreria (`library` `.many()`, `library_preset`, `search_mode`, `hybrid_entropy`/`stein_cosine`, blocco `libmatch`) + `table.attribute@1` + CLI | `libset`, `matchres` (fatti) | `flow_attribute.rs` in `crates/mzlab-cli/src/`, `flow_tablematch.rs`, `mzlab-feat/src/steps/` |
| wasm `LibSet.search` + pannello laterale su `libmatch/1` (ricerca all'apertura dello spettro, analoghi in coda annullabili) | `libset`, `matchres` (fatti) | `crates/mzlab-lib-wasm`, `libreria-worker.js`, `ident-pannello.js` |
| analoghi: `modifiche-1.json`, ΔFormula vincolata al padre, kit di proprietà (NOSC, `rt_dir_ok`) | `matchres`; le colonne attendono `table.attribute` e il pannello | `mzlab-core`, `matching.rs` |
| banco su MassBank e GNPS reali (disco, voci visitate) | – | `examples/libset_bench.rs`, `misure-rust.md` |

## 13. Decisioni aperte
1. **Livello nel pannello**: lo calcola il JS (`LIVELLI.assess` in `mzlab/web/libreria.js`); *consigliata* cascata Rust nel wasm delle librerie se sta nel tetto; alternative: (b) resta `LIVELLI.assess` in JS; (c) alzare il tetto del wasm librerie.
2. **Ricerca aperta su tutto l'indice** (oltre ±200 Da): *consigliata* solo su richiesta (voce «Open search»); alternative: (b) sempre nel Flow, mai nel pannello; (c) mai.
3. **GC-EI**: il motore ha Stein + ΔRI; *consigliata* nessuna interfaccia finché non serve; alternative: (b) lasciarlo solo nel motore senza piani; (c) interfaccia completa subito con il pannello.
4. **Download esterni nel browser**: *consigliata* solo pacchetti ridistribuiti dal sito (licenze libere); alternative: (b) anche URL esterni se la CSP/CORS verificata lo permette (CSP da allargare); (c) solo link, nessun download nel browser.
5. **Analoghi in LR**: *consigliata* niente analoghi né ΔFormula a risoluzione unitaria (Principio 1 e massa nominale); alternative: (b) analoghi senza ΔFormula; (c) come in HR.
