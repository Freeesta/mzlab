# Tabui: come mostrare la tabella delle feature

Studio dell'Architetto per Federico: rivedere da zero come Tabui mostra le feature (ioni, composti, prodotti di trasformazione) trovate nei campioni. Solo stato e proposte. Prompt delle chat DEV: `docs/studi/PROMPT_TABUI_FEATURE_TABLE/` (ordine in `LEGGIMI.md`).
Vincoli di Federico: semplice, poco denso, fluido; niente finestre piene di dettagli; ogni cosa con un aiuto chiaro in italiano e in inglese; Tabui esiste solo con la modalità amministratore (5 clic sul logo, a interruttore); nessun lucchetto o blocco nascosto oltre a quella.

## 1. Cosa c'è oggi: quattro tabelle, quattro grammatiche
Osservazioni da immagini Playwright su dati sintetici (serie HR di `tools/feat_mirato.py`): tema chiaro e scuro, finestra larga e a 390 px, telefono (`?telefono=1`: niente Tabui, corretto). T1 è descritta dal codice: lo scavo LR sintetico non ha prodotto righe in tempo. Il tema scuro è leggibile.

| | Righe | Dove | Colonne di default | Apertura di una riga | Filtri | Ordinamento |
|---|---|---|---|---|---|---|
| **T1 Scavo LR** | progenitore, candidati, inattesi (decine) | `#tp-t`, `rows()` in `TP_Mine/js/20_tpmine.js` | #, Lvl, Score, Candidate, Formula, m/z, RT, t max, Max area, Kinetics | il dettaglio si disegna in fondo alla colonna (`#tp-det`, dopo la scheda MRM): la riga resta in alto, il dettaglio va cercato scorrendo | 4 caselle (strong, possible, unexpected, weak) | nessuno (ordine del motore) |
| **T2 Prodotti HR** | candidati dopo i filtri (decine-centinaia) | `#hr-t`, `TPTAB` in `42_tp_table.js`, colonne in `21_tpmine_hr.js` | Lvl, Score, m/z, Formula (ppm), Change, RT, Kinetics, t0, Blank (+ 8 dal menu «Columns») | sotto la riga (una alla volta), tastiera completa | livello minimo, testo, MS², regione, non in t0, non nel bianco, ordinamento a tendina | intestazioni-pulsante (`aria-sort`) |
| **T3 Feature table della Pipeline** | tutte le righe del `.mzfeat` (migliaia) | `#ft-g`, `PIPET` in `37_feature_table.js`; lista delle identità `PIPEV` in `33_pipeline_view.js` | ID, m/z, RT, poi un'area per file (titolo tagliato a 22 caratteri) con la lettera dello stato (T, F, A) | il dettaglio di `PIPER.detail` (XIC/MS2 dal file aperto in Dati) sotto la lista | 8 campi numerici (normalizzazione, rilevata in N file, area, bianco, m/z, RT) | nessuno (m/z crescente) |
| **T4 Tabelle esterne** | feature di mzMine/MS-DIAL/GNPS2 | finestra `#unt`, `27_untarget_ui.js` | Level, ID, m/z, RT, Max area, MS², Best candidate, Entropy, Δppm, External evidence | specchio e «What is it?» nella finestra | livello | nessuno |

In arrivo: «MS2 matches» e colonna MS2 in T1 (M4 di `TABUI_MS2_LR.md`), la tabella dei livelli di Attributio (`51_attributio_table.js`, AT-04 di `ATTRIBUTIO.md`), le colonne isotopi/identità di T3 (MZ-05).

## 2. Cosa deve vedere e fare un chimico
Sei azioni, in quest'ordine; ognuna deve costare al più un clic senza perdere il posto.
1. **Che cosa è cambiato?** Le righe che compaiono o crescono col trattamento, assenti nel bianco e al t0, in cima. Il progenitore sempre visibile come riferimento.
2. **Quanto ci credo?** Un livello per riga (scala di Schymanski, «L3»), con accanto, mai confusa, la previsione («Prev. 2»), e le prove in parole brevi. Il 2b lo scrive solo il chimico.
3. **Fammi vedere.** XIC di tutti i campioni, spettro MS1 con gli isotopi, MS2 con i frammenti spiegati (uguali/spostati/perdite), abbinamento MS2 (M4): sotto la riga, senza una finestra nuova.
4. **Confronta i campioni.** Andamento nel tempo (cinetica), presenza nei controlli (t0, bianco, buio), e per ogni cella se il picco è stato **rilevato**, **cercato** (mirato) o **riempito** (area riletta dove nessun picco era stato trovato).
5. **Decidi e annota.** Livello deciso, nota, «non è un TP»: restano nella riga e nelle esportazioni.
6. **Porta via.** Esporta ciò che vedo (righe filtrate, colonne visibili, ordine) per Excel o per un altro programma; i formati per altri programmi restano a parte.

## 3. Problemi concreti
**Grammatica**
- P1 Quattro tabelle con nomi, filtri, apertura e tastiera diversi (T1 `<table>` cliccabile senza tastiera; T2 pulsante per riga; T3 `role=grid` di `div` con le celle non raggiungibili; T4 `<table>`). Chi impara una non sa usare le altre.
- P2 Nomi diversi per la stessa cosa: «Lvl»/«Level», «Max area» calcolata in modi diversi, «Kinetics» solo in T1/T2, «ID» con prefisso «n» per gli inattesi in T1.

**Colonne e densità**
- P3 T1 e T2 mettono insieme Score e livello: due numeri che dicono quasi la stessa cosa (in T2 si legge «4» accanto al badge «4»), e il punteggio sembra una probabilità.
- P4 T3 mostra un'area per file: con 13 file la riga è larga 1 500 px, i nomi dei file sono tagliati a 22 caratteri, i numeri a 6 cifre non si confrontano a occhio; nessuna colonna resta ferma scorrendo in orizzontale.
- P5 T2 mette i «flag» dentro la cella «Change» (righe alte e irregolari); T4 mette le prove esterne in testo libero.
- P6 Formati dei numeri diversi: m/z a 2 decimali in T1, 4 in T2/T3; RT a 2 decimali in T3, come arriva in T1/T2.
- P7 Sopra la tabella HR ci sono 6 contatori, una nota lunga sui livelli, gli avvisi, una barra con 8 controlli e il menu delle colonne; sotto, rete delle differenze di massa, imbuto dei filtri, allineamento in RT e albero MSn tutti aperti uno dopo l'altro (l'albero è un grande riquadro quasi vuoto). La riga aperta di T2 è alta circa 1 000 px (metadati, cinetica, 10 componenti del punteggio quasi tutte 0.00, criteri, specchio MS2, tabella di somiglianza) e allontana le altre righe; la barra in alto, ferma, copre la tabella scorrendo. In T1, sopra la tabella, un titolo giocoso («Vein found!»), contatori animati, coriandoli e 5 viste (Table, Gems, Reaction map, Kinetics, Film).

**Stato delle celle**
- P8 Rilevata/mirata/riempita si vede solo in T3 (una lettera piccola prima dell'area) e in T2 («○N» accanto alla cinetica); T1 non lo dice; nessuna legenda vicino alla tabella.

**Identità e livello**
- P9 Il badge di Schymanski mostra la sola cifra («3», «2a») e la previsione è «P1…P4» / «Pred.»: contro `docs/agenti/livelli.md` (etichette «L3» e «Prev. 2»). «◆» (prove diagnostiche) non ha legenda in tabella.
- P10 Nessun posto per il livello deciso dal chimico in T1/T2/T3 (esiste solo in T4).

**Collegamenti**
- P11 In T1 il dettaglio appare in fondo, sotto la scheda MRM: su una serie lunga il clic «non fa niente» finché non si scorre.
- P12 In T3 XIC e MS2 vengono «dal file aperto in Dati»: se la scheda Dati ha un altro file il dettaglio è vuoto senza dire perché.
- P13 MS2 abbinate (M4) e albero di frammentazione stanno in schede separate dalla riga.

**Ordinamento, filtri, confronto**
- P14 T1, T3 e T4 non si ordinano; T2 ha sia le intestazioni sia una tendina «Sort by» che si contraddicono.
- P15 I filtri sono di tre tipi (caselle per etichetta, tendina di livello, campi numerici). In T3 un filtro numerico non dice quante righe toglie finché non si legge la riga sotto.
- P16 Il confronto fra campioni è una sparkline senza assi in T1/T2 e una matrice di numeri in T3: nessuna vista «campioni» leggibile.

**Aiuti e lingua**
- P17 Gli aiuti sono attributi `title`: il suggerimento globale (`eventi.js`, `#qtip`) compare solo col mouse dopo 1,25 s, mai con la tastiera né al tocco (WCAG 1.4.13); informazioni utili stanno solo lì («+N» alternative, ppm, rapporti t0/bianco).
- P18 Tutto il testo di Tabui è solo in inglese (`docs/agenti/mzfinder.md`); Federico vuole gli aiuti in italiano e inglese.

**Esportazione**
- P19 T1 ha Excel/CSV/JSON/«Copy summary» in cima, T2 Excel/CSV/JSON/Inclusion list, T3 sette pulsanti, T4 i suoi: nessuno esporta «quello che vedo» (filtri e colonne).

**Finestra stretta**
- P20 Tabui non c'è sul telefono (giusto: `telefono.js`), ma in una finestra di 390-700 px (tablet, metà schermo) T2 e T3 scorrono in orizzontale senza colonne ferme; in T2 formule e «Change» vanno a capo su 2-3 righe, contatori e barra dei filtri si spezzano.

## 4. Come lo fanno gli strumenti noti (solo principi)
- **MZmine**: colonne fisse (ID, m/z, RT), poi un gruppo richiudibile per campione; lo stato della cella ha un'icona. Principio: **colonne di riga ferme, gruppi di colonne per campione richiudibili, stato come simbolo**.
- **MS-DIAL**: accanto all'elenco, barre dei campioni per gruppo e MS2 contro la libreria; «Fill %» dice quanto è riempito. Principio: **il confronto fra campioni è un piccolo grafico per gruppo, non una fila di numeri**.
- **Compound Discoverer**: tabelle figlie che si aprono sotto la riga; «Annot. Source» a simboli per fonte; colonna «Checked» del chimico. Principio: **riga madre corta, dettaglio sotto la riga, prove come simboli compatti per fonte, decisione del chimico nella riga**.
- **Skyline**: «Document Grid» con viste di colonne salvate ed esportate come si vedono. Principio: **ciò che si esporta è la vista scelta**.
- Comune a tutti: livello in una colonna corta; numeri a destra con decimali fissi; ordinamento dalle intestazioni; la tabella non diventa un modulo.

## 5. Regole di progetto (accessibilità e densità)
- **Una tabella vera** (`<table>`, `<th scope=col>`, `aria-sort`), anche quando è virtualizzata (righe vuote di spaziatura sopra e sotto, `aria-rowcount`/`aria-rowindex`): lo screen reader la legge come tabella e le celle si raggiungono.
- **Colonne fisse**: le prime due (stato/livello e identità) restano ferme in orizzontale (`position: sticky`), l'intestazione resta ferma in verticale.
- **Poche colonne di default** (≤ 7 a 1 280 px), le altre in insiemi nominati e non in un elenco di 17 caselle.
- **Righe a altezza costante** (28-32 px, 40 px al tocco), una riga di testo per cella; ciò che non ci sta va nella riga aperta.
- **Riga che si apre** sotto sé stessa (uno alla volta), con schede corte (XIC · Spectra · Samples · Evidence) e non una pagina lunga.
- **Numeri**: allineati a destra, cifre tabulari, m/z a 4 decimali in HR e 2 in LR (risoluzione unitaria), RT a 2 decimali, aree in notazione breve (1,2e6) col valore intero nell'aiuto.
- **Colori** solo da variabili CSS, Okabe-Ito per le categorie, scala sequenziale leggibile dai daltonici (cividis) per le aree; mai il colore da solo: c'è sempre un simbolo o un testo.
- **Aiuti** apribili con mouse, tastiera e tocco, che restano finché si leggono e si chiudono con Esc (WCAG 1.4.13); testi di una o due righe.
- **Bersagli** ≥ 24×24 px (40 px al tocco), fuoco visibile, `prefers-reduced-motion` (niente contatori animati né coriandoli con il movimento ridotto).
- **Ogni grafico nuovo** (sparkline, barre per campione, mappa colorata) ha la sua tabella dei dati (`TDATI`).

## 6. Tre disegni

### A. Una tabella, una riga che si apre (consigliato)
La stessa tabella per T1, T2, T3 (e per T4 e Attributio quando arrivano), costruita da un solo componente (`TPTAB`). Sopra: una riga di filtri a gettoni e un selettore di insiemi di colonne. Ogni riga: stato e livello, identità, m/z, RT, andamento, presenza nei controlli, prove a simboli. La riga si apre sotto di sé con quattro schede.
```
 Tabui · 23 of 140 ions    [Changed by treatment ✓] [Not in blank ✓] [With MS2] [L4 or better]   Columns: Essential ▾   Export view ▾   (?)
┌────┬──────────────────────────┬──────────┬───────┬──────────────┬──────────┬────────────────┐
│ L  │ Identity                 │    m/z   │   RT  │ Trend (t)    │ t0 · Bl  │ Evidence       │
├────┼──────────────────────────┼──────────┼───────┼──────────────┼──────────┼────────────────┤
│ ★  │ Parent  C10H12N2O3S      │ 241.0641 │  8.42 │ ▇▆▄▂▁        │ ●  ○     │                │  ← sempre in cima
│ L3 │ +O   C10H12N2O4S         │ 257.0590 │  7.10 │ ▁▃▆▇▅  ●●●○● │ ○  ○     │ MS2 ✓ · Prev.2 │
│ L4 │ −CH2  C9H10N2O3S         │ 227.0485 │  6.88 │ ▁▂▄▆▇        │ ○  ○     │ Prev.4         │
│▸L5 │ m/z 199.05 (unexpected)  │ 199.0536 │  5.31 │ ▁▁▃▇▆  ●●◐●● │ ○  ◐     │ isotope?       │
├────┴──────────────────────────┴──────────┴───────┴──────────────┴──────────┴────────────────┤
│  [ XIC ]  Spectra   Samples   Evidence                                       Decided level: [ — ▾]│
│  XIC of all the samples (±5 ppm)…  (grafico + tabella dei dati)                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
 ● detected   ◐ filled (area re-read where no peak was found)   ◌ targeted   ○ absent        L = level (Schymanski)  Prev. = prediction (Löffler)
```
- Insiemi di colonne: **Essential** (quelle sopra), **Samples** (identità + una colonna stretta per campione, vedi C), **Evidence** (score, formula/ppm, ΔRT dal progenitore, Prev., MS2, regione, classe), **All**. L'insieme scelto è ricordato (`localStorage` `qqq.tabui.cols`).
- Pro: un solo modo di lavorare; XIC/MS2 vicino alla riga; pochi colori; vale per tutte le tabelle. Contro: per la matrice dei campioni si cambia insieme di colonne.

### B. Elenco e pannello accanto
Elenco stretto a sinistra (livello, identità, m/z, RT), pannello fisso a destra con XIC, spettri e prove della riga scelta (come MS-DIAL).
```
┌──────────────────────────────┬───────────────────────────────────────────────┐
│ L3 +O   257.0590  7.10       │  +O  C10H12N2O4S   L3 · Prev.2                │
│▶L4 −CH2 227.0485  6.88       │  ┌ XIC ───────────────────────────────┐       │
│ L5 m/z 199.05  5.31          │  └────────────────────────────────────┘       │
│ …                            │  ┌ MS2 vs parent ─────────────────────┐       │
│                              │  └────────────────────────────────────┘       │
└──────────────────────────────┴───────────────────────────────────────────────┘
```
- Pro: grafici subito visibili. Contro: due zone che scorrono, metà schermo sempre piena di dettagli, inutilizzabile sotto i 900 px.

### C. Matrice dei campioni
La tabella come mappa: righe = feature, colonne strette = campioni raggruppati (bianco · t0 · trattati nel tempo), cella colorata con l'area (cividis, log) e un simbolo per lo stato.
```
                         Blank  t0  │  5   15   30   60  120 min
 L3 +O   257.0590  7.10    ○    ○   │  ◐   ▓●   █●   █●   ▓●
 L4 −CH2 227.0485  6.88    ○    ○   │  ○   ░●   ▒●   ▓●   █●
 L5 m/z 199.05     5.31    ◐    ○   │  ○   ○    ▒●   █●   ▓●
```
- Pro: campioni e stato delle celle a colpo d'occhio, anche con molti file. Contro: da sola non dice identità e prove; con 30 campioni le colonne sono di 12 px.

## 7. Consiglio
**A come tabella di tutto Tabui, con C come suo insieme di colonne «Samples»** (non una vista separata). B no: occupa metà schermo con dettagli sempre aperti e non regge le finestre strette.
Motivi: un solo modo di leggere, filtrare, aprire ed esportare (P1-P2); «fammi vedere» senza perdere il posto (P11-P13); densità scelta con un insieme di colonne (P3-P7); confronto fra campioni con lo stato (P4, P8, P16); si costruisce sul `TPTAB` che esiste, senza toccare i motori.

## 8. Specifica del disegno consigliato
**Componente** (`TP_Mine/js/42_tp_table.js`, `TPTAB`): riceve colonne, insiemi, righe, gettoni di filtro, testi di aiuto e le funzioni di apertura; non calcola numeri.
- Colonne con `key`, `label`, `help` (chiave dei testi), `num`, `sort`, `cell`, `text` (valore per l'esportazione) e `sticky`.
- Insiemi (`sets`): `essential`, `samples`, `evidence`, `all`; il primo è quello di default.
- Gettoni (`chips`): `{key, label, help, test(row)}`, attivi o no, contatore «N of M» in un `role=status`.
- Virtualizzazione sopra 300 righe (spaziatori `<tr>`, `aria-rowcount`/`aria-rowindex`).
- Riga di riferimento fissata in cima (`pin`: il progenitore), esclusa da filtri e ordinamento.
- Riga che si apre: un `<tr class="tpt-exp">` con le schede (`tabs: [{key, label, fill(row, box)}]`), una sola aperta; Esc chiude e rimette il fuoco sul pulsante della riga.
- «Export view»: CSV `;` con BOM e Excel delle righe e colonne visibili nell'ordine visibile (`text` delle colonne), con una riga `#` che elenca i filtri; i formati per altri programmi restano dove sono.
- Finestra < 700 px: insieme `essential` forzato, prime due colonne ferme, il resto scorre; la riga aperta va a tutta larghezza.

**Simboli dello stato della cella** (uguali in tutte le tabelle, legenda sotto la tabella e nell'aiuto): ● rilevata · ◐ riempita · ◌ mirata (cercata, non trovata dal rilevamento) · ○ assente. Nella matrice C il simbolo sta sopra il colore.

**Livelli** (`docs/agenti/livelli.md`): badge «L3», «L2a»… (colori di `livelli.js`), previsione «Prev. 2» con un'altra forma (contorno, non pieno); «◆» diventa il gettone «diagnostic» con aiuto; il livello deciso è una tendina nella riga aperta («—», L2b, L3, L4, L5, «not a TP») e, se c'è, sostituisce il badge proposto con un contorno doppio. Il programma non scrive mai 2b.

**Ordine di default**: progenitore fissato; poi livello migliore, poi punteggio (T1/T2); m/z in T3/T4. Un clic su un'intestazione cambia l'ordine e la tendina «Sort by» sparisce.

**Testi e aiuti**: etichette brevi in inglese (m/z, RT, XIC, MS2 non si traducono); ogni intestazione, gettone, simbolo e badge ha un aiuto di 1-2 righe in italiano e in inglese secondo la lingua della pagina, da un catalogo di Tabui scaricato solo con la modalità amministratore (`TP_Mine/js/40_tabui_testi.js`); l'aiuto è un pulsante «?» o l'intestazione stessa, aperto da mouse, tastiera e tocco.

**Sopra la tabella** (tutte): una riga sola con il riassunto in parole («23 of 140 ions · parent t½ 18 min»), i gettoni, l'insieme di colonne, «Export view», «?». Animazioni e coriandoli solo senza movimento ridotto. Rete delle differenze di massa, imbuto, allineamento in RT, albero MSn, MRM e «MS2 matches» vanno in una riga di schede chiuse sotto la tabella («More: Mass differences · Filters applied · RT alignment · MSn tree · MRM · MS2 matches»), una aperta alla volta.

**Schede della riga aperta**
| Scheda | T1 LR | T2 HR | T3 Pipeline |
|---|---|---|---|
| XIC | XIC di tutti i campioni ±Da | XIC ±5 ppm | XIC del file scelto (tendina dei file della serie, non «il file aperto in Dati») |
| Spectra | MS2 abbinate (M4), frammenti, transizioni MRM consigliate | specchio MS2 col progenitore, isotopi | MS1 e MS2 del file scelto |
| Samples | tabella delle aree con stato e cinetica | cinetica (`#hr-kin`) con stato | celle per file con stato |
| Evidence | criteri ✓ ✗, «In words», livello deciso | componenti del punteggio, localizzazione, Prev., livello deciso | prove (Contributions, 1/N), motivi di scarto |

## 9. Decisioni da prendere (i prompt usano la scelta (a))
- D1 Lingua: (a) etichette in inglese, aiuti in italiano e inglese dal catalogo di Tabui (`40_tabui_testi.js`, solo con la modalità amministratore); (b) anche le etichette nelle due lingue.
- D2 Punteggio: (a) colonna nascosta di default, nell'insieme «Evidence» e nell'ordine; (b) resta visibile.
- D3 Viste giocose di T1 (Gems, Reaction map, Film): (a) restano, raggiunte da «More»; (b) si tolgono.
- D4 Livello deciso dal chimico in T1-T3: (a) nella riga aperta, ricordato nel browser (`qqq.tabui.decisions`, per impronta della serie) ed esportato; (b) solo nelle esportazioni.
- D5 Matrice dei campioni: (a) insieme di colonne «Samples» della stessa tabella; (b) vista separata.

## 10. Pacchetti
Ogni pacchetto è una chat DEV (Sonnet) con il suo prompt in `docs/studi/PROMPT_TABUI_FEATURE_TABLE/`. Nessuno tocca Rust, i `.wasm`, i workflow o `mzlab/web/teoria/`.

| Pacchetto | Cosa | File toccati | e2e | Dipende da |
|---|---|---|---|---|
| FT-01 | Catalogo dei testi di Tabui (it/en) e aiuto accessibile `TPHELP` | `TP_Mine/js/40_tabui_testi.js`, `41_tabui_aiuto.js` (nuovi), `TP_Mine/tests/test_tabui_testi.py`, `docs/agenti/mzfinder.md` | `e2e_tabui_aiuto.py` (nuovo) | — |
| FT-02 | Componente `TPTAB`: insiemi di colonne, gettoni, colonne ferme, simboli dello stato e legenda, virtualizzazione, riga fissata, schede nella riga aperta, «Export view» | `TP_Mine/js/42_tp_table.js`, `docs/agenti/mzfinder-tabella.md` | `e2e_tpmine_tabella.py` (resta verde), `e2e_tabui_tptab.py` (nuovo, pagina di prova) | — (usa `TPHELP` se c'è) |
| FT-03 | T2 HR sul disegno A: colonne, gettoni, «L»/«Prev.», sopra la tabella in una riga, «More» | `TP_Mine/js/21_tpmine_hr.js`, `docs/agenti/{mzfinder,mzfinder-tabella,livelli}.md` | `e2e_tpmine_tabella.py`, `e2e_tpmine_ms2.py`, `e2e_tpmine_rtalign.py` | FT-01, FT-02; in serie con M5 (stesso file) |
| FT-04 | T1 LR sul disegno A: tabella `TPTAB`, riga che si apre con le schede, progenitore fissato, «More» | `TP_Mine/js/20_tpmine.js`, `docs/agenti/{mzfinder,mzfinder-lr}.md` | `e2e_tpmine_lr_tabella.py` (nuovo), `e2e_tpmine_ms2_lr.py` (di M4), `e2e_tpmine_lr_rust.py`, `e2e_tpmine_albero.py` | FT-01, FT-02, M4 unita; in serie con M5 e RV-11 (stesso file) |
| FT-05 | T3 Pipeline sul disegno A: righe della vista in `TPTAB` virtualizzata, filtri come gettoni + «More filters», file scelto nella riga aperta | `TP_Mine/js/37_feature_table.js`, `TP_Mine/js/34_pipeline_run.js` (solo l'aggancio del dettaglio), `docs/agenti/mzfeat-wasm.md` | `e2e_pipeline.py` | FT-01, FT-02, MZ-04 e MZ-05 unite |
| FT-06 | Insieme «Samples» (matrice C) nel componente | `TP_Mine/js/44_tp_matrix.js` (nuovo), una riga in `42_tp_table.js`, `docs/agenti/mzfinder-tabella.md` | `e2e_tabui_tptab.py`, `e2e_axe_grafici.py` | FT-02 |
| FT-07 | Livello deciso dal chimico (D4) in T1-T3 | `TP_Mine/js/45_tabui_decisioni.js` (nuovo), aggancio in `20_tpmine.js`, `21_tpmine_hr.js`, `37_feature_table.js` (una riga ciascuno) | `e2e_tabui_decisioni.py` (nuovo) | FT-03, FT-04, FT-05 |
| FT-08 | T4 e tabella di Attributio sul componente | `TP_Mine/js/27_untarget_ui.js`, `51_attributio_table.js` | `e2e_untarget.py`, `e2e_attributio.py` | FT-02, AT-04 unita |

**Onde**
- Onda 1: FT-01 e FT-02 insieme (file diversi).
- Onda 2: FT-03 (se M5 non è aperta) e FT-06; FT-04 appena M4 è unita (e M5/RV-11 non sono aperte); FT-05 appena MZ-05 è unita.
- Onda 3: FT-07; FT-08 dopo AT-04.
- Conflitti con gli altri studi: M3 tocca solo Rust (nessuno); M4 e M5 toccano `20_tpmine.js`/`21_tpmine_hr.js` (FT-04/FT-03 in serie con loro); MZ-04/MZ-05 toccano `32/34/35_pipeline*.js` e `37_feature_table.js` (FT-05 dopo); AT-02 tocca `livelli-1.js`/`livelli-1.json` (nessun pacchetto FT li tocca: «L» e «Prev.» sono solo presentazione in `TP_Mine/js/`); AT-04 crea `51_attributio_table.js` (FT-08 dopo).

## 11. Trappole
- Tabui senza modalità amministratore non lascia traccia: nessun testo, chiave `lang/`, stile o file della pagina che lo nomini; il catalogo dei testi sta in `TP_Mine/js/` e si scarica solo con la modalità.
- `tools/controlla_i18n.py` salta `TP_Mine/` per l'italiano: il catalogo di Tabui va provato da `TP_Mine/tests/test_tabui_testi.py` (stesse chiavi in `it` e `en`, nessun testo vuoto).
- `e2e_tpmine_tabella.py` e `e2e_tpmine_ms2.py` cercano `#hr-t`, `button.tpt-tg`, `tr.tpt-exp`, `#tp-det`, `#hr-kin`, `#hr-ms2f`, `#hr-cnt`: i selettori restano, o il test si aggiorna nella stessa PR.
- Il nodo `#tp-det` è unico e si sposta sotto la riga: le schede della riga aperta si disegnano dentro di lui.
- Una tabella virtualizzata non ha tutte le righe nel DOM: «Export view» legge le righe dal modello, non dal DOM.
