# Posizione della sostituzione (classi compatibili con la MS2)

Codice: `crates/mzlab-feat/src/chem/posizioni.rs` e `posizioni/` (feature `chem`, sopra `grafo::Mol`); prove: `crates/mzlab-feat/tests/posizioni.rs`. Il pannello «Substitution position» di Tabui (amministratore) sta in `TP_Mine/js/44_posizioni.js`; prova: `tests_e2e/e2e_posizioni.py`. Studio: `docs/studi/CHIMICA_COMPUTAZIONALE.md`.

## Scopo
- Dato un genitore (SMILES), una trasformazione e la MS2 osservata del TP, elenca gli atomi dove la modifica può stare, li raggruppa e dice quali gruppi lo spettro sostiene, esclude o non distingue. Non sceglie mai una posizione e non assegna livelli: `proposal.sublevel` è una proposta 3a/3b per il chimico (nessun campo si chiama `level`).
- Un frammento con la modifica dice in quale PEZZO sta; dentro un anello intero le posizioni restano indistinguibili (livello 3a/3b, multicentro del Disegno).

## API Rust
- `sites(mol, transform) -> Result<Vec<Site>>`: `+O` (`aromatic_oh`: C-H aromatico; `aliphatic_oh`: C-H sp3; `n_oxide`: N terziario neutro non acilato o N aromatico a due legami; `s_oxide`: solfuro), `-CH2` (`demethyl`: metile su N, O o S), `+O-2H` (`carbonyl`: CH2 sp3), `+Cl-H` (`aromatic_cl`), `+Br-H` (`aromatic_br`: C-H aromatico). Spazi, `−` e `+O-H2` accettati. Errori: `error.posiz.transform`, `error.posiz.noSites`, `error.posiz.formula` (`check_formulas`: i prodotti devono avere tutti la stessa formula).
- `symmetry_classes(mol) -> Vec<u32>`: raffinamento iterativo di (elemento, grado, H, aromaticità, carica) con i vicini (ordine di legame, classe). Limite: su rari grafi regolari può unire atomi non equivalenti; l'effetto è un candidato in meno, mai una prova tolta.
- `expected_fragments(mol, site, pol) -> Vec<Frag>`: ogni rottura di `Mol::cleavages()` (anelli chiusi, idrolisi con OH sull'altro lato), i due lati con formula e m/z di [F+H]+ ([F−H]− in negativo); il lato col sito prende la modifica. Con `-CH2` i legami del metile perso sono saltati. Cl e Br: isotopo più abbondante.
- `match_spectrum(mol, sites, peaks, tol, pol) -> Result<Report>`; `Tol::Da` (LR, tipico 0,5) o `Tol::Ppm` (HR).
- `positions_json(smiles, transform, peaks_json, tol, ppm, positive) -> Result<String>`: picchi `[[mz,int],..]` o `[{"mz","i"},..]`; `ppm` false = Da. `error.posiz.peaks`, `error.posiz.tol`.

## Contratto `posiz/1`
`{format:"posiz", version:1, parent, transform, tp_formula, tp_mz, pol:"+|-", tol, tol_unit:"da|ppm", sites:[{atom, symmetry, kind}], classes:[{id, atoms, kind, status, support:[{mz, frag_formula, side_atoms, iso_mz?}], against:[..]}], ox_type:{h2o_loss, minus_o}|null, unexplained:[mz], suggest:[..], proposal:{sublevel:"3a|3b", why}}`.
- `atom` è l'indice nell'ordine del SMILES; `symmetry` la classe di simmetria; `kind` di una classe è `mixed` se unisce tipi diversi. `iso_mz`: picco +2 (³⁷Cl, ⁸¹Br) trovato per un frammento con alogeno. `why`: `none_excluded`, `excluded_few_left`, `many_classes_left`.
- `ox_type` solo per `+O` (altrimenti `null`): perdita di H2O dal precursore e [M+H−O]+ osservati o no; sono solo prove mostrate, non filtrano. Ramanathan et al. 2000 (doi:10.1021/ac9911692): in ESI l'OH alifatico perde acqua più del fenolo; [M+H−O]+ è tipico degli N-ossidi, soprattutto in APCI.
- `suggest`: `hdx` se restano insieme candidati C-OH e N-/S-ossido (Cajka et al. 2026, doi:10.3390/analytica7030057); `ms3` se un frammento con la modifica è comune a più classi (Murakami et al. 2018, doi:10.1007/s11419-018-0410-4); `standard` sempre.

## Regole di confronto
- Picchi usati: i 30 più intensi sopra l'1 % del picco base dello spettro, tolto il precursore; «intenso» = almeno il 10 % del picco base.
- Un candidato (siti con stessa classe di simmetria e stesso tipo) spiega un picco CON la modifica (frammento che la porta) o SENZA (frammento che non la porta).
- Classe = candidati con lo stesso insieme di picchi spiegati con la modifica e la stessa esclusione: lo spettro non li distingue.
- `excluded`: un picco intenso è spiegato con la modifica da altri candidati e il candidato non lo spiega in nessun modo (né con né senza). `supported`: un picco spiegato con la modifica appartiene, tra i candidati non esclusi, esattamente ai membri della classe. Altrimenti `undistinguished`.
- `proposal.sublevel` = 3a solo se almeno una classe è esclusa e ne restano al più 3; altrimenti 3b. `unexplained`: picchi che nessun candidato spiega (né picco +2 di un frammento alogenato).

## Pannello «Substitution position» (Tabui, solo `uipAdminOn()`)
- Motore: `positions(smiles, transform, peaks_json, tol, ppm, positive)` del `.wasm` `grafo` (`crates/mzlab-feat-grafo-wasm`, `version()` = `mzfeat-grafo/4`; stesso JSON di `positions_json`, prova `tests/positions.rs`); nell'app la rotta `feat/grafo` con `kind: "positions"` (`smiles`, `transform`, `peaks` come testo JSON, `tol`, `ppm`, `positive`; `feat_grafo.rs`, `positions()` del pool `feat-desktop-pool.js`). Il `.wasm` pesa 158 151 B prima e 234 654 B dopo (`wasm-opt -Oz`), sotto il tetto comune di 512 000 B.
- Pagina: `TPPOS.mount(host, modello)` aggiunge in fondo alla scheda di una riga un riquadro con il pulsante; `TPPOS.eligible(modello)` è vero solo con modalità amministratore, SMILES del genitore, MS2 con almeno un picco e Δ della riga tra `+O`, `-CH2`, `+O-2H` (anche `+O-H2`), `+Cl-H`, `+Br-H` (spazi e «−» accettati); altrimenti nessun pulsante.
- Punti d'innesto: LR in `show()` di `20_tpmine.js` (picchi = frammenti `[mz, rel]` di `d.ms2` abbinata da `ms2_links/1`, tolleranza 0,5 Da); HR in `fill()` di `21_tpmine_hr.js` (picchi della MS2 del candidato `d.ms2.candidate`, 10 ppm). Lo SMILES è quello del campo `#tp-smi`, la polarità viene dall'addotto scelto.
- Contenuto: proposta («Proposal: Schymanski 3a|3b — N position classes remain; the chemist decides», nessun pulsante salva un livello); tabella delle classi (id, atomi come simbolo + indice 1-based dello SMILES, tipo, stato scritto in parole con ✓ ✗ ≈, frammenti a favore e contro con m/z e formula, riga «unexplained»); spettro MS2 a barre con marcatore e tratteggio per classe (colori Okabe-Ito da variabili CSS), tabella `TDATI`, zoom con pulsanti e tasti (+ − ← → Home, niente trascinamento); riquadri «Oxidation type» e «Suggested checks» con DOI.

## Limiti
- Nessuna apertura d'anello, nessuna stereochimica; con risoluzione unitaria un m/z coincide per caso più spesso (tolleranza 0,5 Da). Convenzione dei frammenti = quella di `Mol::cleavages()` (ogni lato prende un H), non una previsione del meccanismo.
- Una stima di reattività (se arriverà) si chiamerà «Prev.» e non riordina né toglie classi.
