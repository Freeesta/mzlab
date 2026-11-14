# Studio spettrale: idee di confronto MS/MS misurate su dati sintetici

Misura, contro il motore di oggi, le quattro idee di DAG-OT-IDF e MORPHO-MS che la revisione ha giudicato da misurare (null calibrato, p-value di coincidenza e pesi informativi, validazione per sotto-formule, analoghi a più spostamenti; E opzionale: Tree-W1 sull'energia di collisione). Solo misure: nessun default, punteggio, formato, passo o pezzo del sito cambia. I bracci sperimentali vivono solo nel codice di test e di esempio; `crates/mzlab-lib/src/` resta identico byte per byte.

## Dove
- Moduli in `crates/mzlab-lib/tests/studio_spettri/`: `gen.rs` (generatore), `scen.rs` + `analogue.rs` (scenari E0-E6 con verità), `arms.rs` (bracci di base), `null.rs` (braccio A), `metrics.rs`, `mod.rs` (impianto comune, numeri fissati).
- Test `crates/mzlab-lib/tests/studio_spettri_<scenario>.rs` (bracci di base) e `studio_spettri_null.rs` (braccio A su E6, E1, E3): libreria di 600 voci, 120 query, seme 1, numeri di oggi fissati con tolleranza 1e-3 (arrotondamento a 4 decimali); ogni test gira due volte (JSON identico byte per byte) e verifica che il braccio entropia dia lo stesso miglior bersaglio e lo stesso q di `decoy::search_fdr`. Meno di 1 s ciascuno in debug.
- Esempio `cargo run --release -p mzlab-lib --example studio_spettri -- <scenario> <n_lib> <n_query> <seme> [base|null]`: una riga JSON per braccio (metriche totali, per sottogruppo e per prefisso prima di `/`; `ms` dell'insieme di bracci diviso per i bracci; `det`: scenario costruito due volte con gli stessi byte di libreria e le stesse query, bracci rieseguiti con lo stesso JSON).
- `python3 tools/studio_spettri.py [--bracci base|null] [--scenari e0,...] [--n-lib 20000] [--n-query 2000] [--semi 1,2,3] [--json OUT]`: esegue l'esempio per 3 semi e stampa le righe `MISURA studio_<scenario>` (`studio_null_<scenario>` per il braccio A; media, tra parentesi min-max di Recall@1 e AUPRC). Circa 1 minuto per i bracci di base, 2 per il braccio A.
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
- `recall_q01`: quota dei colpi giusti accettati a q <= 0,01 sulle query con verità in libreria (la potenza del null: un null che rifiuta tutto ha ECE 0). `q_auc`: probabilità che un primo candidato sbagliato (trappola o errore) abbia q più alto di uno giusto, pari a metà; 1 separa del tutto, 0,5 per niente.
- Calibrazione: a 20 soglie τ log-spaziate in [0,001, 0,1] si accettano i colpi con q <= τ; `ece` è la media di |FDP vero - q massimo accettato|, `fdp_q01` l'FDP vero a q <= 0,01. q = `decoy::lib_fdr` sul miglior bersaglio e la miglior esca di ogni query, come `best_scores_with`.
- `gate`: quota di query il cui primo candidato passa il cancello `shared` di `livelli-1.js` (>= `picchi_min` 6, o 3 quando la voce ha meno di 6 picchi).
- `q_true` / `q_trap`: q mediano dei colpi veri e delle trappole. Per braccio: `ms` (solo esempio) e `det`.

## Bracci di base
- `entropy` (predefinito), `cosine`, `modified_cosine`: tutti i candidati della finestra di 10 ppm (`search::identity_with`, frammenti 0,02 Da, i valori predefiniti di `spectra.match` su una query HR), ordinati per punteggio quantizzato (`decoy::quantize`).
- `analogs`: `search::analogs_with` con `AnalogOpts` predefiniti (Flash ibrido, coseno modificato, `min_delta` 0,5 Da); misurato su E5 e su E0, dove ogni analogo restituito è falso.

## Braccio A: null calibrato (`null.rs`)
- Ordinamento sempre dell'entropia del motore (Recall@1 e AUPRC non cambiano per costruzione); cambia solo il null. Ogni tipo di esca costruisce il suo `.mzlib` (bersagli più un'esca ciascuno, accessione `DECOY:` come `decoy::make_decoy`); q di `decoy::lib_fdr` e, per ogni tipo, `<tipo>+1` con q = (1 + #esche >= s) / max(1, #bersagli >= s), poi il minimo progressivo.
- Tipi: `fragments` (oggi), `precursor` (l'altro tipo del motore), `swap_near` (picchi della voce più vicina per precursore, anche un isomero nella finestra), `swap_out` (la più vicina oltre 20 ppm), sui picchi del donatore sotto entrambi i precursori - 1,6 Da; `subformula` (tante sotto-formule casuali valide della radice quanti picchi, mai su un picco vero, intensità tenute); `sead` (le perdite consecutive dal precursore rimescolate e risommate, ogni rango tiene la sua intensità; resta fermo il frammento più basso).

## Misure (n_lib 20 000, 2000 query, semi 1-3, media)
- MISURA studio_e0: entropy/cosine/modified_cosine Recall@1 1,000, AUPRC 1,000, ECE 0, `gate` 0,566, `score` 0,924 / 0,985 / 0,985; analogs (falsi analoghi) `hit` 0,162, `score` 0,393, ms 711 contro 49-64 dell'identità.
- MISURA studio_e1: entropy Recall@1 0,959 [0,956-0,962], MRR 0,978, AUPRC 0,070 [0,034-0,116], ECE 0,036, `fdp_q01` 0,041, `gate` 0,0003, `score` 0,715; cosine Recall@1 0,984, AUPRC 0,317 [0-0,748], `fdp_q01` 0,016; modified_cosine come cosine (AUPRC 0,235 [0-0,704]).
- MISURA studio_e2: tutti e tre Recall@1 1,000, AUPRC 1,000, `gate` 0,443; `score` entropy 0,856, cosine 0,978.
- MISURA studio_e3: entropy Recall@1 0,9995, AUPRC 0,9995, `gate` 0,548; `score` per η (isobarico / isomero): 0,1 0,839 / 0,834; 0,2 0,785 / 0,779; 0,3 0,741 / 0,748; 0,4 0,692 / 0,702; 0,5 0,649 / 0,654; 0,6 0,603 / 0,614. Cosine e modified_cosine Recall@1 0,9993.
- MISURA studio_e4: entropy Recall@1 0,998, AUPRC 0,997; per CE 10/20/50/65: Recall@1 0,991 / 0,999 / 1 / 1, `score` 0,651 / 0,842 / 0,900 / 0,879, `gate` 0,165 / 0,456 / 0,519 / 0,475; cosine Recall@1 0,993.
- MISURA studio_e5: analogs Recall@1 0,291 [0,282-0,303], MRR 0,295, AUPRC 0,206 [0,186-0,232], `hit` 0,317, ECE 0,081, `gate` 0,152; -CH2 Recall@1 0,293, -Cl+H 0,280.
- MISURA studio_e6: entropy Recall@1 0,993, AUPRC 0,680 [0,652-0,720], ECE 0,339, `fdp_q01` 0,367, `q_true` 0, `q_trap` 0,005; per stile `q_trap` e0 1, e1 0,001, e2 0, e3 1; cosine e modified_cosine AUPRC 0,641, `fdp_q01` 0,366.
- MISURA studio_null_e6 (`ece` / `fdp_q01` / `recall_q01` / `q_auc`): fragments 0,339 / 0,367 / 0,993 / 0,847; precursor 0,370 / 0,392 / 0,993 / 0,781; swap_near 0 / 0 / 0,002 / 0,943; swap_out 0,209 / 0,238 / 0,971 / 0,949; subformula 0,266 / 0,280 / 0,993 / 0,904; sead 0,155 / 0,214 / 0,934 / 0,960. Con +1 cambiano di <= 0,011.
- MISURA studio_null_e6 per stile (`fdp_q01` / `recall_q01`): e0 fragments 0,191 / 1, swap_out 0,018 / 1, subformula 0,043 / 1, sead 0,011 / 0,992; e1 0,472 / 0,972, 0,179 / 0,912, 0,329 / 0,972, 0,126 / 0,835; e2 0,5 / 1 per tutti (sead 0,469); e3 0,160 / 1, 0,001 / 0,973, 0,020 / 1, 0 / 0,908.
- MISURA studio_null_e1 (`ece` / `fdp_q01` / `recall_q01`): fragments 0,036 / 0,041 / 0,959; precursor 0,039 / 0,041 / 0,959; swap_near 0,089 / 0,026 / 0,057; swap_out 0,023 / 0,039 / 0,895; subformula 0,020 / 0,034 / 0,839; sead 0,077 / 0,034 / 0,828 (sead+1 ECE 0,017).
- MISURA studio_null_e3 (`fdp_q01` / `recall_q01`): fragments 0,0005 / 0,999; swap_near 0 / 0,0005; swap_out 0,0005 / 0,987; subformula 0,0005 / 0,999; sead 0 / 0,898 (a η 0,6: 0,728; swap_out 0,947).
- Determinismo: `det` vero per ogni braccio, scenario e seme.

## Findings
- Le esche `Fragments` non separano le trappole che condividono i frammenti con la voce della stessa formula: in E6 il q mediano delle trappole E1 ed E2 è 0 e l'FDP vero a q <= 0,01 è 0,37 (ECE 0,34). Le trappole senza frammenti comuni (E0, E3) restano a q 1. È il limite inferiore già scritto in `lib.fdr.note`; obiettivo del braccio A.
- Piccoli (E1): l'entropia sbaglia il 4 % dei primi candidati e gli errori stanno in cima (AUPRC@1 % 0,07); il coseno sbaglia l'1,6 % (AUPRC 0,32, molto variabile tra i semi). Il cancello `shared` lascia passare lo 0,03 % (3 picchi condivisi contro 6, perché la voce ha 5 picchi deboli in più): stesso esito di MISURA livelli_ms2_piccoli. Obiettivo del braccio B.
- Chimerici (E3): il primo candidato resta giusto fino a η 0,6, ma l'entropia scende da 0,92 (E0) a 0,84 con η 0,1 e a 0,60-0,61 con η 0,6, senza differenza tra interferente isobarico e isomero. Obiettivo del braccio C.
- Analoghi a due spostamenti (E5): la ricerca degli analoghi trova il genitore nel 29 % dei casi e nel 68 % non restituisce nessun candidato (`hit` 0,32: meno di 3 coppie ibride o meno di 4 picchi accoppiati); in E0 restituisce un analogo falso per il 16 % delle query. Obiettivo del braccio D.
- Energia (E4): il primo candidato resta giusto (>= 0,99 a ogni CE), il punteggio scende a 0,65 a CE 10 e il cancello `shared` passa per il 16 %.
- Isomeri posizionali (E2): con rumore 2 ppm e 8 % i due diagnostici al 3 % bastano a tutti e tre i punteggi (Recall@1 1).
- Braccio A: nessun tipo di esca calibra le trappole. Il migliore in E6 è `sead` (FDP vero a q <= 0,01 da 0,37 a 0,21, ECE da 0,34 a 0,15, `q_auc` da 0,85 a 0,96) e paga potenza: `recall_q01` da 0,993 a 0,934, 0,90 sui chimerici (0,73 a η 0,6), 0,83 sui piccoli; `swap_out` scende a 0,24 con meno perdita (0,971). Le trappole isomeriche (stile E2, 90 % del picco base in comune) restano a FDP 0,47-0,5 con ogni esca: non sono errori casuali e nessun null per esche le vede.
- Braccio A: `swap_near` è inutilizzabile: l'esca di un isomero nella finestra porta i picchi veri della query, q mediano dei colpi veri 0,20 in E6 e 0,33 in E3, `recall_q01` <= 0,06. La correzione +1 sposta l'FDP di <= 0,011 con 2000 query; l'ECE pure, tranne `sead` in E1 (0,077 → 0,017, perché le soglie più basse restano vuote).
- Braccio A e regola: Recall@1 e AUPRC sono quelli dell'entropia per costruzione, quindi nessun tipo «batte» la base secondo la regola; il guadagno di calibrazione di `sead` e `swap_out` è reale (3 semi, `det` vero) ma resta lontano da FDP 1 % e costa potenza. Decide il proprietario.
- Regola di decisione per i bracci A-D (nessuna azione dallo studio, decide il proprietario): un braccio «batte» la base solo se sul suo scenario Recall@1 sale di >= 5 punti e AUPRC@FDR<=1 % di >= 0,05, con E0 che perde <= 1 punto, su tutti e 3 i semi e identico byte per byte alla riesecuzione.
- Per ogni vincitore si scrive: byte in più nel wasm `librerie` (tetto 614 400, misurato 502 678), cambio del formato `.mzlib` se serve, costo per rendere l'aritmetica deterministica tra nativo e wasm (niente exp/ln f64 grezzi in un punteggio; quantizzare come `decoy::quantize`), come ogni termine comparirebbe in `score_terms`.
