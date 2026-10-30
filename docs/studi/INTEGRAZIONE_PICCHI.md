# Integrazione automatica dei picchi: quale algoritmo per l'alta risoluzione

Domanda di Federico: il nostro algoritmo di integrazione è il migliore? Prima di costruire l'integrazione automatica di tutti i picchi di un XIC ad alta risoluzione (un clic sull'area del pannello XIC integra tutti i picchi del tratto mostrato), questo studio descrive gli algoritmi che mzLab ha già (§1), le alternative della letteratura e dei programmi di laboratorio (§2), li misura su tracce sintetiche con aree note (§3) e dà la raccomandazione (§4). In §5 ciò che NON è stato verificato.

## 0. Verdetto
- **Per l'HR si sostituisce**: nessuno dei tre metodi che mzLab ha già è adatto a integrare da solo tutti i picchi di una traccia. Il migliore sul banco è un metodo nuovo (F2, §3.2) costruito con i pezzi Rust che ci sono già (AsLS, Savitzky-Golay, prominenza): errore medio assoluto sull'area **5,4 %** su 510 picchi veri contro 24,5 % del clic della pagina, 36,0 % della prominenza e 37,1 % della CWT; **0 falsi picchi**, 32 mancati (30 sono la spalla Rs 0,5 che nessun metodo separa).
- **La bassa risoluzione resta com'è** (istruzione di Federico): il banco mostra però che i bordi di `autoEdges` con la retta per le medie di 3 punti perdono area quando il rapporto S/N è basso (−5,6 % a S/N 100, −33,6 % a S/N 10). È un dato per una decisione futura, non una modifica di questo lavoro.
- **Nessuna dipendenza nuova**: tutto in Rust dentro `mzlab-core`; il `.wasm` del motore cresce di circa 7 KB (2 524 401 byte, tetto 2 744 320 non toccato).

## 1. Cosa fa oggi mzLab
### 1.1 Il clic della pagina (bassa e alta risoluzione, un picco per clic)
- `autoEdges(s, x, ref)` (`mzlab/web/picchi.js:38`): lisciatura Savitzky-Golay grossolana (mezza finestra 0,015 min) per trovare il massimo entro ±0,25 min dal clic e la sua FWHM; poi Savitzky-Golay quadratica di mezza finestra 30 % della FWHM (`PK.minHalf` … 60 punti) e la sua derivata prima.
- Rumore: MAD dei residui grezzo − lisciato entro ±`PK.win` (1,2 min) dall'apice (o RMS, o una zona scelta dallo studente). Fondo locale = 10° percentile della traccia lisciata nella stessa finestra.
- Ogni bordo cammina dall'apice finché il lisciato scende sotto fondo + max(`PK.edgeFrac` = 3 % dell'altezza, `PK.areaNoise` = 3 × rumore), o la derivata torna a salire per `PK.multi` = 50 % della mezza finestra (una valle), o si superano `PK.tail` = 1,5 min.
- Area: `integ(s, a, b)` (`picchi.js:5`) = trapezi della traccia grezza meno la retta fra le medie di 3 punti ai due bordi; altezza sopra la stessa retta. Parametri in `PK_DEF` (`picchi.js:34`), finestra «Parametri di rilevamento dei picchi…».
### 1.2 Il rilevatore CWT (`detect_peaks`, `crates/mzlab-picchi/src/peaks.rs:142`)
- Trasformata a ondine (cappello messicano) su più scale, creste e S/N ≥ 5 sul fondo AsLS (`signal::asls_baseline`, λ 1e5); limiti dove la traccia scende al 5 % dell'apice. Lo usa la tabella delle feature; la pagina non lo usa per i bordi.
### 1.3 La prominenza (`detect_prominence`, `crates/mzlab-core/src/peakq.rs:159`)
- Massimi della traccia lisciata (finestra 5), prominenza ≥ 5 × rumore e ≥ 1 % del punto più alto, almeno 3 scansioni, con un controllo del rumore locale (`local_snr`) contro le increspature sulla cima. Limiti: discesa monotona fino al 5 % dell'apice o alla valle condivisa. Era il primo prototipo dell'integrazione HR di un clic (`auto_limits`), qui misurato come «C».

## 2. Le alternative
- **Savitzky-Golay + derivata** (Savitzky e Golay 1964; i sistemi HPLC classici, Chromeleon e OpenLab): apice dove la derivata prima passa per zero, bordi dove la pendenza torna sotto una soglia. È la famiglia di `autoEdges`. Robusta ma la soglia di pendenza va legata al rumore e alla larghezza.
- **CentWave / XCMS** (Tautenhahn et al. 2008): regioni di interesse in m/z, poi CWT per i picchi; area sul fondo stimato sui bordi della regione. È la base di 1.2: buona per trovare i picchi, i limiti al 5 % tagliano le code.
- **MZmine ADAP e «local minimum»** (Myers et al. 2017; Pluskal et al. 2010): ADAP = CWT con controllo della forma e della linea di base; local minimum = minimo locale in una finestra di RT con altezza minima e rapporto cima/bordo. Il local minimum è parente della prominenza (1.3).
- **Skyline e MassHunter** (MacLean et al. 2010; Agilent): bordi da valle a valle (o al fondo), linea di base retta fra i due bordi; le coppie non separate si tagliano con una verticale («drop line») o una tangente. Provato come «D» (prominenza + drop line).
- **Adattamento di forme** (gaussiana, EMG: Grushka 1972; Di Marco e Bombi 2001): l'area viene dai parametri della curva; dà le spalle e le code, ma è lento, chiede un modello di forma e può non convergere. Non provato (§5).
- **Fondo AsLS** (Eilers e Boelens 2005): minimi quadrati asimmetrici penalizzati; segue derive e gobbe larghe senza entrare nei picchi se λ è grande. Già nel motore (1.2); provato come «E» e dentro F.

## 3. Banco su tracce sintetiche
### 3.1 Il banco
- `crates/mzlab-core/tests/integrazione_bench.rs`, comando `cargo test --release -p mzlab-core --test integrazione_bench -- --nocapture` (stampa le tabelle in meno di un secondo; le asserzioni tengono i numeri di F2). Il banco e `peakq_auto.rs` arrivano con la PR di costruzione `claude/integrate-hr-auto`.
- Tracce: 600 punti a 0,01 min, σ = 0,05 min (FWHM ≈ 0,12 min, 24 punti), fondo 1e3, rumore additivo 1e3 più 3 % moltiplicativo sul segnale (statistica degli ioni); 30 semi per caso. Area vera = h·σ·√(2π) (per l'EMG quella della gaussiana); limiti veri dove la forma pulita è ≥ 1 % del massimo.
- Casi: singolo S/N 100; S/N 10; S/N 5; coda (EMG τ/σ 1,6); coppia Rs 1,5 (1:0,6); coppia Rs 0,8; spalla Rs 0,5 (1:0,4); fondo in deriva; gobba larga sotto il picco (gaussiana σ 0,6 min alta 2e4); cinque picchi da 1e5 a 5e3; nessun picco.
- Misure: errore assoluto medio e scarto medio (bias) dell'area in %, errore dei bordi in σ, picchi mancati e falsi, µs per traccia. Un picco vero è trovato se una finestra contiene il suo apice.
- Candidati: A = CWT (1.2) + `integ`; B = il clic della pagina (porting Rust di `autoEdges` e delle regole di non sovrapposizione, uguale alla pagina su 10 tracce, cliccando sugli apici veri); C = prominenza (1.3) + `integ`; D = prominenza + drop line; E = prominenza + area sopra il fondo AsLS; F0/F1/F2 = metodo nuovo (3.2) in tre varianti.
### 3.2 Il metodo F
- Fondo AsLS di tutta la traccia (λ 1e7, p 0,01) alzato della mediana dei punti di fondo (residuo < 3 rumori): AsLS da solo sta sotto il rumore.
- Lisciatura Savitzky-Golay quadratica di mezza finestra m = 30 % della FWHM del picco più alto (2-30 punti); apici dalla prominenza (1.3, `local_snr` 5).
- Ogni bordo cammina dall'apice finché il lisciato sopra il fondo scende a k volte il SUO rumore (rumore grezzo × √ del peso centrale del filtro), o alla valle condivisa con il vicino (F0: k = 0; F1 e F2: k = 1).
- F2 aggiunge due arresti: oltre metà altezza, pendenza sotto il 3 % della più ripida del picco per più di m punti (il picco è finito su un fondo che non è piatto: una gobba, la coda di un altro picco); al massimo 5 FWHM. Lì l'estremo della linea di base è il livello lisciato, sul fondo e alle valli è il fondo AsLS.
- Area: trapezi della traccia grezza meno la retta fra i due estremi della linea di base (quello che la pagina disegna).
### 3.3 Risultati per caso (errore assoluto medio dell'area %, scarto %, errore dei bordi in σ, mancati/veri, falsi)
| caso | A CWT | B clic pagina | C prominenza | E prom.+AsLS | **F2** |
|---|---|---|---|---|---|
| singolo S/N 100 | 10,4 | 5,6 (−5,6) | 13,3 | 2,9 | **1,3** (−1,2; 0,24 σ) |
| S/N 10 | 67,3 | 33,6 (−33,6) | 67,4 | 27,7 | **3,4** (−0,3; 0,49 σ) |
| S/N 5 | 87,8 | 52,7 | 84,7 (2/30 mancati) | 47,3 | **8,1** (−0,5; 0,82 σ) |
| coda τ/σ 1,6 | 27,7 | 10,2 | 28,3 | 13,6 | **3,0** (−3,0) |
| coppia Rs 1,5 | 15,4 | 6,3 (18/60 mancati) | 14,9 | 6,5 | **1,7** |
| coppia Rs 0,8 | 54,3 | 40,8 (30/60) | 55,4 | 9,0 | **3,1** |
| spalla Rs 0,5 | 31,3 (29/60) | 32,4 (30/60) | 28,2 (30/60) | 35,3 (30/60) | 38,6 (30/60) |
| fondo in deriva | 10,4 | 3,0 | 2,0 | 1,6 | **1,4** |
| gobba sotto il picco | 6,0 | 36,2 (+36,2) | 1,6 | 5,0 | **1,8** |
| cinque picchi | 42,8 (6/150, 5 falsi) | 25,9 | 43,0 (7/150, 5 falsi) | 17,8 | **3,9** (2/150) |
| nessun picco | 0 falsi | 30 falsi (clic alla cieca) | 0 | 0 | **0** |
- Totale su 510 picchi veri (errore assoluto medio dell'area, mancati, falsi): A 37,1 % (35, 8); B 24,5 % (78, 30); C 36,0 % (39, 7); D 31,4 % (39, 7); E 15,7 % (39, 7); F0 18,4 % (32, 0); F1 16,5 % (32, 0); **F2 5,4 % (32, 0)**.
- F1 sbaglia di +180 % sulla gobba (il bordo scende lungo la gobba): l'arresto sul fianco piatto di F2 è ciò che la toglie. Con una soglia di pendenza del 2 % invece del 3 % il totale è 5,7 % (gobba 6,9 %): resta il 3 %.
- Velocità (µs per traccia di 600 punti, compilato release): F2 120-180, CWT 350-500, prominenza 40-55, clic della pagina circa 25 per picco.
### 3.4 Perché i metodi esistenti perdono area
- Prominenza (C) e CWT (A): i limiti si fermano al 5 % dell'apice o al primo dente di rumore (discesa monotona con lisciatura a 5 punti); le code restano fuori. A S/N 10 l'area esce dimezzata.
- Clic della pagina (B): la retta passa per i punti al bordo, che stanno al 3 % dell'altezza o a 3 rumori sopra il fondo, e toglie anche quella fetta (−6 % a S/N 100); a S/N basso il bordo si ferma presto. Sulla gobba il bordo scende lungo la gobba e l'area la include.
- E (stessi limiti di C con il fondo AsLS): l'errore scende a 15,7 % ma i limiti restano corti. F mette il fondo e i bordi d'accordo: il bordo cammina fino al fondo che poi si sottrae.

## 4. Raccomandazione
- **HR: sostituire** il motore del clic unico con F2: `peakq_auto::auto_integrate` (`crates/mzlab-core/src/peakq_auto.rs`), nel `.wasm` del motore come `auto_integrate` (sito e app desktop usano lo stesso `.wasm` nel `peaks-worker.js`). Ogni integrazione porta la sua linea di base (`base = [t0, y0, t1, y1]`) che la pagina disegna e sottrae; i bordi restano trascinabili.
- Parametri nella finestra dei picchi, sezione Alta risoluzione: quelli della prominenza (lisciatura 5, S/N 5, altezza minima 1 %, larghezza minima 3 scansioni) più la mezza finestra dei bordi (0 = dalla larghezza) e l'arresto sul fianco piatto (3 %).
- **Spalle (Rs ≤ 0,5)**: nessun metodo provato le separa; F2 integra il gruppo (area del gruppo giusta entro il 3 % nella prova Rust), la spalla resta dentro il picco principale. Una separazione vera chiede la derivata seconda o l'adattamento EMG: lavoro a parte, se Federico lo vuole.
- **LR**: invariata. Se in futuro si vuole migliorare l'area dei picchi piccoli in LR, la strada indicata dal banco è la stessa linea di base (fondo + bordo al rumore del lisciato) dietro il clic singolo.

## 5. Cosa NON è stato verificato
- **Dati veri del laboratorio**: non disponibili in questa sessione (repository privato non presente). I numeri sono solo sintetici; la forma reale del rumore HR (zeri dove l'ione manca, picchi isolati, campionamento irregolare del DDA) non è nel banco.
- **Taratura sugli stessi casi**: le soglie di F2 (1 rumore, 3 %, 5 FWHM, 30 % della FWHM) sono state scelte guardando questo banco; un banco diverso potrebbe dare numeri peggiori. Le larghezze provate sono una sola (σ 0,05 min); tracce con picchi molto più larghi e più stretti nella stessa traccia usano la mezza finestra del picco più alto.
- **Non provati**: adattamento gaussiano/EMG, deconvoluzione delle spalle con la derivata seconda, la tangente (skim) per i picchi piccoli sulla coda di uno grande, il confronto diretto con XCMS, MZmine o Skyline sugli stessi file.
