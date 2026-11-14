# Studio spettrale: idee di confronto MS/MS misurate su dati sintetici

Misura, contro il motore di oggi, le quattro idee di DAG-OT-IDF e MORPHO-MS che la revisione ha giudicato da misurare (null calibrato, p-value di coincidenza e pesi informativi, validazione per sotto-formule, analoghi a più spostamenti; E opzionale: Tree-W1 sull'energia di collisione). Solo misure: nessun default, punteggio, formato, passo o pezzo del sito cambia. I bracci sperimentali vivono solo nel codice di test e di esempio; `crates/mzlab-lib/src/` resta identico byte per byte.

## Dove
- Moduli in `crates/mzlab-lib/tests/studio_spettri/`: `gen.rs` (generatore), `scen.rs` + `analogue.rs` (scenari E0-E6 con verità), `arms.rs` (bracci), `metrics.rs`, `mod.rs` (impianto comune, numeri fissati).
- Test `crates/mzlab-lib/tests/studio_spettri_<scenario>.rs`: libreria di 600 voci, 120 query, seme 1, numeri di oggi fissati con tolleranza 1e-3 (arrotondamento a 4 decimali); ogni test gira due volte (JSON identico byte per byte) e verifica che il braccio entropia dia lo stesso miglior bersaglio e lo stesso q di `decoy::search_fdr`. Meno di 1 s ciascuno in debug.
- Esempio `cargo run --release -p mzlab-lib --example studio_spettri -- <scenario> <n_lib> <n_query> <seme>`: una riga JSON per braccio (metriche totali e per sottogruppo, `ms` della ricerca, `det`: scenario costruito due volte con gli stessi byte di libreria e le stesse query, braccio rieseguito con le stesse righe).
- `python3 tools/studio_spettri.py [--scenari e0,...] [--n-lib 20000] [--n-query 2000] [--semi 1,2,3] [--json OUT]`: esegue l'esempio per 3 semi e stampa le righe `MISURA studio_<scenario>` (media; tra parentesi min-max di Recall@1 e AUPRC). Circa 1 minuto per tutti gli scenari.
- Un cambio del motore o del generatore che sposta un numero aggiorna insieme il test e la riga MISURA.

## Generatore (`gen.rs`)
- Casuale solo da `decoy::XorShift` con seme fisso, un flusso per oggetto (una voce della libreria di base ha il suo: un sottocampione tiene le stesse voci); masse da `mzlab_core::elements`, exp/ln/cos da `libm`.
- Precursori: formula neutra CHNO con P (5 %), S (10 %), Cl (12 %), RDBE intero >= 0 (Senior: somma delle valenze pari), [M+H]+ 120-700.
- Albero di frammentazione: profondità 2-5, ramificazione 1-3 (la radice ramifica sempre, gli altri ioni con probabilità 0,6), al più 25 frammenti; ogni figlio è il padre meno una perdita di un elenco fisso di 18 (H2O, CO, CO2, NH3, CH2O, C2H4, HCN, CH3OH, C2H2O, HCl, SO2, CH4, C3H6, C2H2, CH2O2, H2S, C4H8, C2H3N); uno ione è valido con C >= 1, H >= 1, RDBE >= -0,5, m/z >= 50, mai due volte nello stesso albero.
- Intensità cinetiche: ogni arco ha (A, E0), A log-uniforme 10^0,5-10^3, E0 0,9-2,6; k = A·exp(-E0/T(CE)), T = 0,15 + 0,012·CE. Uno ione decade nel tempo unitario nei figli in proporzione a k; quel che resta si vede. I rapporti di ramificazione cambiano con la CE (nessuna invarianza presupposta).
- Rumore: m/z 2 ppm (sigma), intensità lognormale 8 %, 0-5 picchi di rumore allo 0,5-2 % del picco base. Un insieme di 40 ioni «scaffold» (80-200) è condiviso dal 30 % delle voci (la radice è scaffold + un resto, con l'arco radice → scaffold).
- Libreria: n_lib voci a CE 35 (test 600, esempio 20 000) con formula, accessione `SYN-<id>`, più le voci che ogni scenario aggiunge; ogni bersaglio ha la sua esca `DecoyKind::Fragments` (quella di `spectra.match decoys=spectrum`), `.mzlib` costruito con `build::build_in_memory`.

## Scenari (verità per query)
- **E0** replica pulita: nuova misura a CE 35 di una voce.
- **E1** piccoli: [M+H]+ < 200, 2-3 frammenti veri; la voce di libreria ha anche 5 picchi all'1-2 %; due concorrenti isobarici (stessa formula) nella finestra: uno con tutti i frammenti tranne uno (cinetica propria), uno indipendente.
- **E2** isomeri posizionali A e B in libreria: picco base comune al 90 % dell'intensità, 2 ioni diagnostici al 3 % ciascuno, gli altri frammenti comuni al 4 %; la query è A.
- **E3** chimerico (1-η)·Q + η·I, η 0,1-0,6, interferente isobarico (Δprec 0,02-0,5 Da) o isomero, assente dalla libreria; sottogruppi `eta=…/isobaric|isomer`.
- **E4** query a CE 10/20/50/65 contro la libreria a CE 35.
- **E5** analogo con due modifiche su sottoalberi disgiunti: +O su uno, -Cl+H (se tutti gli ioni del sottoalbero hanno Cl) o -CH2 sull'altro; la radice e gli antenati portano le modifiche dei loro discendenti. Il composto modificato non è in libreria; la verità della ricerca degli analoghi è la voce non modificata.
- **E6** per stile E0-E3 metà query in libreria e metà trappole (stessa formula di una voce, altro albero, assente); sottogruppi `eN/true|trap`.

## Metriche (`metrics.rs`)
- Recall@1 e MRR sulle query con verità in libreria; `hit` quota di query con almeno un candidato; `score` mediana del punteggio del primo candidato.
- `pauprc`: area sotto la curva precisione-richiamo dei primi candidati ordinati per punteggio (gruppi di punteggi uguali insieme) fino all'ultimo taglio con FDP vero <= 1 %; il massimo è il richiamo a FDP 1 %.
- Calibrazione: a 20 soglie τ log-spaziate in [0,001, 0,1] si accettano i colpi con q <= τ; `ece` è la media di |FDP vero - q massimo accettato|, `fdp_q01` l'FDP vero a q <= 0,01. q = `decoy::lib_fdr` sul miglior bersaglio e la miglior esca di ogni query, come `best_scores_with`.
- `gate`: quota di query il cui primo candidato passa il cancello `shared` di `livelli-1.js` (>= `picchi_min` 6, o 3 quando la voce ha meno di 6 picchi).
- `q_true` / `q_trap`: q mediano dei colpi veri e delle trappole. Per braccio: `ms` (solo esempio) e `det`.

## Bracci di base
- `entropy` (predefinito), `cosine`, `modified_cosine`: tutti i candidati della finestra di 10 ppm (`search::identity_with`, frammenti 0,02 Da, i valori predefiniti di `spectra.match` su una query HR), ordinati per punteggio quantizzato (`decoy::quantize`).
- `analogs`: `search::analogs_with` con `AnalogOpts` predefiniti (Flash ibrido, coseno modificato, `min_delta` 0,5 Da); misurato su E5 e su E0, dove ogni analogo restituito è falso.

## Misure (n_lib 20 000, 2000 query, semi 1-3, media)
- MISURA studio_e0: entropy/cosine/modified_cosine Recall@1 1,000, AUPRC 1,000, ECE 0, `gate` 0,566, `score` 0,924 / 0,985 / 0,985; analogs (falsi analoghi) `hit` 0,162, `score` 0,393, ms 711 contro 49-64 dell'identità.
- MISURA studio_e1: entropy Recall@1 0,959 [0,956-0,962], MRR 0,978, AUPRC 0,070 [0,034-0,116], ECE 0,036, `fdp_q01` 0,041, `gate` 0,0003, `score` 0,715; cosine Recall@1 0,984, AUPRC 0,317 [0-0,748], `fdp_q01` 0,016; modified_cosine come cosine (AUPRC 0,235 [0-0,704]).
- MISURA studio_e2: tutti e tre Recall@1 1,000, AUPRC 1,000, `gate` 0,443; `score` entropy 0,856, cosine 0,978.
- MISURA studio_e3: entropy Recall@1 0,9995, AUPRC 0,9995, `gate` 0,548; `score` per η (isobarico / isomero): 0,1 0,839 / 0,834; 0,2 0,785 / 0,779; 0,3 0,741 / 0,748; 0,4 0,692 / 0,702; 0,5 0,649 / 0,654; 0,6 0,603 / 0,614. Cosine e modified_cosine Recall@1 0,9993.
- MISURA studio_e4: entropy Recall@1 0,998, AUPRC 0,997; per CE 10/20/50/65: Recall@1 0,991 / 0,999 / 1 / 1, `score` 0,651 / 0,842 / 0,900 / 0,879, `gate` 0,165 / 0,456 / 0,519 / 0,475; cosine Recall@1 0,993.
- MISURA studio_e5: analogs Recall@1 0,291 [0,282-0,303], MRR 0,295, AUPRC 0,206 [0,186-0,232], `hit` 0,317, ECE 0,081, `gate` 0,152; -CH2 Recall@1 0,293, -Cl+H 0,280.
- MISURA studio_e6: entropy Recall@1 0,993, AUPRC 0,680 [0,652-0,720], ECE 0,339, `fdp_q01` 0,367, `q_true` 0, `q_trap` 0,005; per stile `q_trap` e0 1, e1 0,001, e2 0, e3 1; cosine e modified_cosine AUPRC 0,641, `fdp_q01` 0,366.
- Determinismo: `det` vero per ogni braccio, scenario e seme.

## Findings
- Le esche `Fragments` non separano le trappole che condividono i frammenti con la voce della stessa formula: in E6 il q mediano delle trappole E1 ed E2 è 0 e l'FDP vero a q <= 0,01 è 0,37 (ECE 0,34). Le trappole senza frammenti comuni (E0, E3) restano a q 1. È il limite inferiore già scritto in `lib.fdr.note`; obiettivo del braccio A.
- Piccoli (E1): l'entropia sbaglia il 4 % dei primi candidati e gli errori stanno in cima (AUPRC@1 % 0,07); il coseno sbaglia l'1,6 % (AUPRC 0,32, molto variabile tra i semi). Il cancello `shared` lascia passare lo 0,03 % (3 picchi condivisi contro 6, perché la voce ha 5 picchi deboli in più): stesso esito di MISURA livelli_ms2_piccoli. Obiettivo del braccio B.
- Chimerici (E3): il primo candidato resta giusto fino a η 0,6, ma l'entropia scende da 0,92 (E0) a 0,84 con η 0,1 e a 0,60-0,61 con η 0,6, senza differenza tra interferente isobarico e isomero. Obiettivo del braccio C.
- Analoghi a due spostamenti (E5): la ricerca degli analoghi trova il genitore nel 29 % dei casi e nel 68 % non restituisce nessun candidato (`hit` 0,32: meno di 3 coppie ibride o meno di 4 picchi accoppiati); in E0 restituisce un analogo falso per il 16 % delle query. Obiettivo del braccio D.
- Energia (E4): il primo candidato resta giusto (>= 0,99 a ogni CE), il punteggio scende a 0,65 a CE 10 e il cancello `shared` passa per il 16 %.
- Isomeri posizionali (E2): con rumore 2 ppm e 8 % i due diagnostici al 3 % bastano a tutti e tre i punteggi (Recall@1 1).
- Regola di decisione per i bracci A-D (nessuna azione dallo studio, decide il proprietario): un braccio «batte» la base solo se sul suo scenario Recall@1 sale di >= 5 punti e AUPRC@FDR<=1 % di >= 0,05, con E0 che perde <= 1 punto, su tutti e 3 i semi e identico byte per byte alla riesecuzione.
- Per ogni vincitore si scrive: byte in più nel wasm `librerie` (tetto 614 400, misurato 502 678), cambio del formato `.mzlib` se serve, costo per rendere l'aritmetica deterministica tra nativo e wasm (niente exp/ln f64 grezzi in un punteggio; quantizzare come `decoy::quantize`), come ogni termine comparirebbe in `score_terms`.
