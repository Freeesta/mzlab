# Tabui: la tabella delle feature e la riga che si espande

Studio dell'Architetto per Federico. Solo stato e proposte. Prompt delle chat DEV: `docs/studi/PROMPT_TABUI_FEATURE_TABLE/` (ordine in `LEGGIMI.md`).
Vincoli di Federico: semplice, poco denso, fluido; niente finestre piene di dettagli; ogni cosa con un aiuto chiaro in italiano e in inglese; Tabui esiste solo con la modalità amministratore (5 clic sul logo, a interruttore); nessun lucchetto o blocco nascosto oltre a quella.

## 1. La decisione
**Tabui mostra la feature table come una tabella tipo mzmine: una riga per feature con poche colonne; un clic su una riga la espande sul posto e mostra tutto ciò che si sa di quel risultato.** La decisione è di Federico e non si rimette in discussione; questo studio definisce la riga chiusa e la riga espansa e scrive i pacchetti.

Dove era già scritta (si riprende così com'è):
- `docs/agenti/mzfinder-tabella.md:6`: nella tabella dei prodotti HR la riga si apre sotto sé stessa, una alla volta; il nodo `#tp-det` è unico e viene spostato sotto la riga (`tr.tpt-exp`); tastiera completa. È già il modello di questo studio.
- `docs/agenti/mzfinder-untargeted.md:27-28`: in «group by identity» il clic su una riga apre massa neutra, letture alternative e prove di ogni arco.
- `docs/studi/PROMPT_ATTRIBUTIO/AT-04.md:10`: Attributio mostra la tabella delle feature con livello e identità.
- `docs/studi/TABUI_MS2_LR.md` §7 e M4: gli abbinamenti MS2 a bassa risoluzione hanno la loro scheda «MS2 matches» in Tabui.

Dove oggi è diverso (da portare alla decisione):
- `docs/agenti/mzfeat-wasm.md:76-79`: nella Pipeline la griglia di base (`37_feature_table.js`, 80 righe) al clic chiama `PIPER.detail`, che disegna il dettaglio in un riquadro separato sotto la lista dei risultati (`#pv-det`), con XIC e MS2 «dal file aperto in Data».
- Lo Scavo LR (`20_tpmine.js`) disegna il dettaglio in fondo alla colonna, dopo la scheda MRM.

Schizzo (le alternative «elenco con pannello accanto» e «matrice dei campioni» non si fanno: la seconda diventa la scheda «Samples» della riga espansa):
```
 Feature table · 412 of 3 180 rows   [Not in blank ✓] [Detected in ≥ 3 files] [With MS2] [L4 or better]   Columns ▾   Export view ▾   (?)
┌──────┬─────────────────────────────┬──────────┬───────┬────────────┬────────┬──────────┐
│ Lvl  │ Identity                    │    m/z   │  RT   │ Trend      │ Cells  │ Max area │
├──────┼─────────────────────────────┼──────────┼───────┼────────────┼────────┼──────────┤
│ ★    │ Parent  C10H12N2O3S [M+H]+  │ 241.0641 │  8.42 │ ▇▆▄▂▁      │ ●●●●●  │   4.1e7  │
│▾ L3  │ +O  C10H12N2O4S [M+H]+      │ 257.0590 │  7.10 │ ▁▃▆▇▅      │ ●●●◐●  │   2.3e6  │
├──────┴─────────────────────────────┴──────────┴───────┴────────────┴────────┴──────────┤
│ L3 · Prev. 2 · Decided: —   +O on the parent; MS2 shares 5 fragments; absent in blank and t0 │
│ [ Overview ]  Samples  Identity  Quality                                                  │
│ ┌ XIC, all samples (peak borders shaded) ┐  ┌ MS1 at apex (isotopes) / MS2 ┐            │
│ └────────────────────────────────────────┘  └──────────────────────────────┘            │
├──────┬─────────────────────────────┬──────────┬───────┬────────────┬────────┬──────────┤
│▸ L5  │ m/z 199.0536 (no hypothesis)│ 199.0536 │  5.31 │ ▁▁▃▇▆      │ ○●◐●●  │   8.0e5  │
 ● detected   ◐ filled   ◌ targeted   ○ absent      L = level (Schymanski)   Prev. = prediction (Löffler)
```

## 2. Cosa c'è oggi
| | Righe | Dove | Colonne | Dettaglio | Filtri e ordinamento |
|---|---|---|---|---|---|
| **T1 Scavo LR** | progenitore, candidati, inattesi | `#tp-t`, `rows()` in `20_tpmine.js` | #, Lvl, Score, Candidate, Formula, m/z, RT, t max, Max area, Kinetics | in fondo alla colonna, dopo la scheda MRM | 4 caselle; nessun ordinamento |
| **T2 Prodotti HR** | candidati dopo i filtri | `#hr-t`, `TPTAB` (`42_tp_table.js`), colonne in `21_tpmine_hr.js` | Lvl, Score, m/z, Formula, Change, RT, Kinetics, t0, Blank (+8 da «Columns») | sotto la riga, una alla volta | livello, testo, MS2, regione, t0, bianco; intestazioni e tendina «Sort by» |
| **T3 Feature table della Pipeline** | tutte le righe del `.mzfeat` | `#ft-g`, `PIPET` (`37_feature_table.js`, griglia di `div` `role=grid`), lista delle identità `PIPEV` (`33_pipeline_view.js`), controllo qualità `36_feature_qc.js`, esportazioni `38_feature_export.js` | ID, m/z, RT, un'area per file con la lettera dello stato (T, F, A) | `PIPER.detail` in `#pv-det` | 8 campi numerici; nessun ordinamento |
| **T4 Tabelle esterne** | feature di mzMine/MS-DIAL/GNPS2 | finestra `#unt`, `27_untarget_ui.js`; «group by identity» in `29_untarget_identity.js` | Level, ID, m/z, RT, Max area, MS2, Best candidate, Entropy, Δppm, External evidence | nella finestra | livello |

Il foglio delle righe del motore (`export.rs:94`) ha id, m/z, RT, bordi del picco (`rt_range:min/max`), carica, poi area e stato per file. Il `.mzfeat` ha anche i riassunti di riga (`docs/agenti/mzfeat-tabella.md:14-17`: forma, S/N, FWHM, bit `qreason`, M+1/M, file della MS2 migliore e purezza) e l'identità (`:19-21`: massa neutra, addotto, fonte), che oggi la pagina non mostra.

Osservazioni dalle immagini Playwright (dati sintetici HR, tema chiaro e scuro, 1 366 e 390 px; telefono senza Tabui, corretto): in T2 il punteggio «4» sta accanto al badge «4»; la riga aperta è alta circa 1 000 px (metadati, cinetica, 10 componenti del punteggio quasi tutte 0.00, criteri, specchio MS2, tabella di somiglianza) e allontana le altre righe; la barra ferma in alto copre la tabella; a 390 px formule e filtri vanno a capo su 2-3 righe. Il tema scuro è leggibile. T1 è descritta dal codice (lo scavo LR sintetico non ha dato righe in tempo).

## 3. Problemi da risolvere
- P1 Quattro grammatiche: nomi, filtri, apertura e tastiera diversi; T3 è una griglia di `div` con le celle non raggiungibili.
- P2 Il dettaglio sta lontano dalla riga (T1 in fondo, T3 in un riquadro sotto la lista) o è troppo lungo (T2).
- P3 Il dettaglio di T3 usa «il file aperto in Data»: con un altro file aperto è vuoto.
- P4 T3 mette un'area per file nella riga (con 13 file la riga è larga 1 500 px, nomi tagliati a 22 caratteri); nessuna colonna resta ferma.
- P5 Score e livello affiancati (due numeri quasi uguali); previsione scritta «P1…P4»/«Pred.» invece di «Prev. 2» (`docs/agenti/livelli.md`); nessun posto per il livello deciso dal chimico in T1-T3.
- P6 Bordi del picco, qualità, addotto e massa neutra esistono nel `.mzfeat` ma non si vedono.
- P7 Rilevata/mirata/riempita: una lettera in T3, «○N» in T2, niente in T1; nessuna legenda.
- P8 Ordinamento solo in T2; filtri di tre tipi; esportazioni diverse e nessuna di «quello che vedo».
- P9 Aiuti solo come `title` (mouse, 1,25 s, mai tastiera né tocco: WCAG 1.4.13) e solo in inglese.
- P10 Sopra le tabelle troppe cose aperte (contatori, note, imbuto, rete delle differenze, albero, viste giocose).

## 4. Regole (da mzmine, MS-DIAL, Compound Discoverer, Skyline e WCAG 2.2 AA)
- Colonne di riga ferme a sinistra, intestazione ferma in alto; stato della cella come simbolo, mai solo colore (mzmine).
- Il confronto fra campioni è un piccolo grafico per gruppo, non una fila di numeri (MS-DIAL).
- Dettaglio sotto la riga madre; decisione del chimico nella riga (Compound Discoverer, colonna «Checked»).
- Si esporta la vista che si vede (Skyline, «Document Grid»).
- Una `<table>` vera (`th scope=col`, `aria-sort`), anche virtualizzata (`aria-rowcount`/`aria-rowindex`); righe di altezza costante (30 px, 40 al tocco); numeri a destra con cifre tabulari; m/z a 4 decimali in HR e 2 in LR, RT a 2, aree brevi (2.3e6) con il valore intero nell'aiuto.
- Colori da variabili CSS, Okabe-Ito per le categorie, viridis per il tempo dei campioni (come `paintFiles` in Dati), bersagli ≥ 24×24 px (40 al tocco), `prefers-reduced-motion`; ogni grafico con la sua tabella dei dati (`TDATI`).

## 5. La riga chiusa
Poche colonne chiave, una riga di testo ciascuna, le prime due ferme.
| Colonna | Contenuto | T1 LR | T2 HR | T3 Pipeline | T4/Attributio |
|---|---|---|---|---|---|
| Lvl (ferma) | badge «L3»; con il livello deciso il badge ha il contorno doppio | ✓ | ✓ | ✓ (da MZ-05/AT; vuoto senza) | ✓ |
| Identity (ferma) | nome o cambio («+O»), formula, addotto; senza ipotesi «m/z 199.0536» | ✓ | ✓ | ✓ | ✓ |
| m/z | ione | ✓ | ✓ | ✓ | ✓ |
| RT | min | ✓ | ✓ | ✓ | ✓ |
| Trend | sparkline nel tempo dei campioni trattati | ✓ | ✓ | ✓ con i tempi dei campioni | — |
| Cells | un simbolo per campione (● ◐ ◌ ○), al più 12, poi «+N» | ✓ | ✓ | ✓ | ✓ |
| Max area | area più alta | ✓ | — | ✓ | ✓ |

Facoltative dal menu «Columns»: Score, Prev., t0, Blank, ppm, ΔRT dal progenitore, MS2, carica, classe, regione. Il progenitore (o lo standard) è fissato in cima, fuori da filtri e ordinamento.

## 6. La riga espansa
**Una sola riga espansa alla volta.** Il clic su un'altra riga chiude la prima e apre la nuova mantenendo la stessa scheda; la riga cliccata resta dov'era sullo schermo (la tabella scorre solo se il pannello uscirebbe in basso). Motivi: un solo `#tp-det` (già così in T2), nessun «muro» di dettagli; il confronto fra righe si fa con la scheda «Samples» e i gettoni, non aprendo tutto.

**Testata** (sempre visibile, una o due righe): livello «L3», previsione «Prev. 2», «Decided: —» (tendina, solo il chimico; `docs/agenti/livelli.md`), poi una frase di prove in parole («+O on the parent; MS2 shares 5 fragments; absent in blank and t0»), poi i pulsanti «Show in Data» (apre il file con l'XIC) e «Copy».

**Schede** (`role=tablist`), in quest'ordine; si apre «Overview»:
1. **Overview**: a sinistra l'XIC di tutti i campioni sovrapposti (colori del tempo, bordi del picco ombreggiati, linea all'RT della riga); a destra lo spettro: in HR MS1 all'apice con gli isotopi (M+1/M osservato contro atteso) e, se c'è, la MS2 a specchio col progenitore; in LR la MS2 abbinata di M4 («MS2 matches» resta la scheda completa sotto la tabella). Sotto i grafici una riga: «RT 7.10 min (6.95-7.31) · FWHM 0.08 min · charge 1 · MS2 from t60_r1 (purity 0.92)».
2. **Samples**: tabella per campione: tipo e tempo (`campioni/2`), area, stato (● ◐ ◌ ○ con la parola), RT dell'apice, bordi, area normalizzata se la vista è normalizzata; sopra, la mini-matrice a colori (viridis per l'area, simbolo sopra il colore) per gruppo (bianco/buio · t0 · tempi · altri). Sostituisce la fila di aree della riga chiusa di oggi.
3. **Identity**: massa neutra e addotto (`ident`), candidati ordinati (formula, Δppm, addotto, fonte: ipotesi, libreria, tabella esterna, Attributio) con livello e previsione di ciascuno, prove ✓/✗ in parole (criteri di oggi; componenti del punteggio solo se diverse da 0), alternative «+N». Qui stanno la tendina «Decided level» e la nota.
4. **Quality**: forma, S/N, FWHM in scansioni, motivi `qreason` in parole («shape», «S/N», «few points»), M+1/M, presenza nel bianco e nel t0 con i rapporti, motivi di scarto dei filtri della vista.

**Altezza**: il pannello sta in ~380 px a 1 280 px di larghezza; i due grafici affiancati da 900 px in su, uno sotto l'altro sotto; le tabelle interne oltre 10 righe scorrono dentro il pannello. Niente altro dentro: albero di frammentazione, rete delle differenze di massa, imbuto, allineamento in RT, MRM e «MS2 matches» stanno in una riga di schede chiuse sotto la tabella («More»), una aperta alla volta.

**Dati**: ogni numero viene dai motori. T3: rotta Rust nuova `table_row` (sessione della tabella, schema `riga/1`: riassunti, identità, celle per file con apice e bordi) e XIC/spettri letti dai `.mzidx` della serie (come `export_ms2` e il dettaglio LR, `finder-lr.js:112`), non dal file aperto in Data. T2: `tp_table` e la scheda di oggi. T1: `detail` dello scavo LR. T4: il modello `UNTM`.

**Tastiera e accessibilità**: ogni riga ha un pulsante (`aria-expanded`, `aria-controls`); Invio/Spazio apre e chiude; ↑ ↓ Pag Inizio Fine fra le righe; Esc chiude e rimette il fuoco sul pulsante; ←/→ fra le schede (`role=tab`, `aria-selected`); il pannello è `role=region` con `aria-labelledby` = identità della riga; «N of M rows» e l'apertura annunciati in un `role=status`. Grafici con titolo, descrizione e `TDATI`.

**Finestra stretta e telefono**: Tabui non c'è sul telefono (`telefono.js`); sotto 700 px restano ferme Lvl e Identity, il resto scorre, il pannello va a tutta larghezza con i grafici uno sotto l'altro e le schede scorrono in orizzontale.

**Tema chiaro e scuro**: solo variabili CSS (`--panel`, `--line`, `--sel`, `--accent`, colori di `livelli.js`); ombre dei bordi del picco e mini-matrice si provano nei due temi (axe e immagine).

**Livelli** (`docs/agenti/livelli.md`): due scale che non si confondono, «L3» (Schymanski, badge pieno) e «Prev. 2» (Löffler, badge col contorno); il programma propone e mostra le prove, il livello deciso resta vuoto finché il chimico non sceglie; **2b solo dal chimico**, mai scritto dal programma; le voci della tendina sono quelle di `livelli.md` più «not a TP». Il livello deciso si ricorda nel browser e va nelle esportazioni.

**Con gli altri studi**: M4 costruisce «MS2 matches» in `20_tpmine.js`: la riga espansa di T1 mostra la MS2 abbinata e rimanda alla scheda (FT-07 dopo M4). MZ-04/MZ-05 producono righe e identità della Pipeline e AT-02 i livelli: la riga li mostra quando ci sono, senza calcolarli (FT-03/FT-05 dopo MZ-05). AT-04 crea la tabella di Attributio: FT-09 la porta sul componente.

## 7. Testi e aiuti
Etichette brevi in inglese (m/z, RT, XIC, MS2 non si traducono); ogni intestazione, gettone, simbolo, scheda e badge ha un aiuto di 1-2 righe in italiano e in inglese secondo la lingua della pagina (`I18N.lang`), da un catalogo di Tabui scaricato solo con la modalità amministratore (`TP_Mine/js/40_tabui_testi.js`, nessuna chiave in `lang/`). L'aiuto è un pulsante «?» o l'intestazione stessa, aperto con mouse, tastiera e tocco, chiuso con Esc, che resta finché si legge.

## 8. Decisioni minori (i prompt usano la scelta (a))
- D1 Lingua: (a) etichette in inglese, aiuti in italiano e inglese; (b) anche le etichette nelle due lingue.
- D2 Score: (a) colonna facoltativa, nella scheda Identity e nell'ordine di default; (b) visibile di default.
- D3 Viste giocose di T1 (Gems, Reaction map, Film): (a) restano, in «More»; (b) si tolgono.
- D4 Livello deciso: (a) ricordato nel browser (`qqq.tabui.decisions`, per impronta della serie) ed esportato; (b) solo nelle esportazioni.

## 9. Pacchetti
Ogni pacchetto è una chat DEV (Sonnet) con il prompt in `docs/studi/PROMPT_TABUI_FEATURE_TABLE/`. Solo FT-03 tocca Rust; nessuno tocca i workflow o `mzlab/web/teoria/`.

| Pacchetto | Cosa | File toccati | e2e | Dipende da |
|---|---|---|---|---|
| FT-01 | Catalogo dei testi (it/en) e aiuto accessibile `TPHELP` | `TP_Mine/js/40_tabui_testi.js`, `41_tabui_aiuto.js` (nuovi), `TP_Mine/tests/test_tabui_testi.py`, `docs/agenti/mzfinder.md` | `e2e_tabui_aiuto.py` (nuovo) | — |
| FT-02 | Componente `TPTAB`: riga chiusa (colonne ferme, gettoni, «Columns», riga fissata, virtualizzazione), riga espansa (testata, schede, tastiera), simboli, legenda, «Export view» | `TP_Mine/js/42_tp_table.js`, `docs/agenti/mzfinder-tabella.md` (fatto: menu «Columns», gettoni ed «Export view» solo se la pagina li chiede con `columns`, `chips`, `exportName`) | `e2e_tpmine_tabella.py` (verde), `e2e_tabui_tptab.py` (nuovo) | — |
| FT-03 | Rotta Rust `table_row` (`riga/1`) e XIC/spettri dai `.mzidx` della serie (fatto) | `crates/mzlab-feat/src/riga.rs` (nuovo), una riga in `lib.rs`, `crates/mzlab-feat-table-wasm/src/lib.rs`, `TP_Mine/js/36_feature_qc.js` (`row`, `traces`), `feat-worker.js`, `feat-desktop-pool.js`, `crates/mzlab-desktop/src/feat.rs`, `docs/agenti/{mzfeat-tabella,mzfeat-wasm}.md` | cargo test | MZ-05 unita |
| FT-04 | Pannello della riga espansa `44_tp_riga.js` (`TPRIGA`: modello di riga, testata, Overview, Samples, Identity, Quality; fatto) | `TP_Mine/js/44_tp_riga.js`, `docs/agenti/mzfinder-tabella.md` | `e2e_tabui_tptab.py` (esteso) | FT-02 |
| FT-05 | T3 Pipeline: `PIPET` su `TPTAB` con la riga espansa; `#pv-det` non serve più per questa tabella (fatto) | `TP_Mine/js/37_feature_table.js`, `34_pipeline_run.js` (solo l'aggancio), `docs/agenti/mzfeat-wasm.md` | `e2e_pipeline.py` | FT-01..FT-04, MZ-04, MZ-05 |
| FT-06 | T2 HR sulla riga espansa a schede (fatto) | `TP_Mine/js/21_tpmine_hr.js`, `docs/agenti/{mzfinder,mzfinder-tabella,livelli}.md` | `e2e_tpmine_tabella.py`, `e2e_tpmine_ms2.py`, `e2e_tpmine_rtalign.py` | FT-01, FT-02, FT-04; in serie con M5 |
| FT-07 | T1 LR su `TPTAB` con la riga espansa | `TP_Mine/js/20_tpmine.js`, `docs/agenti/{mzfinder,mzfinder-lr}.md` | `e2e_tpmine_lr_tabella.py` (nuovo), `e2e_tpmine_ms2_lr.py`, `e2e_tpmine_lr_rust.py`, `e2e_tpmine_albero.py` | FT-01, FT-02, FT-04, M4; in serie con M5 e RV-11 |
| FT-08 | Livello deciso dal chimico | `TP_Mine/js/45_tabui_decisioni.js` (nuovo), una riga in `44_tp_riga.js` | `e2e_tabui_decisioni.py` (nuovo) | FT-04 e una fra FT-05/06/07 |
| FT-09 | T4 e tabella di Attributio sul componente | `TP_Mine/js/27_untarget_ui.js`, `29_untarget_identity.js`, `51_attributio_table.js` | `e2e_untarget.py`, `e2e_attributio.py` | FT-02, FT-04, AT-04, MZ-05 |

**Onde**
- Onda 1: FT-01 e FT-02 (file diversi); FT-03 appena MZ-05 è unita.
- Onda 2: FT-04 dopo FT-02.
- Onda 3: FT-05 (dopo FT-03), FT-06 (se M5 non è aperta), FT-07 (dopo M4, con M5 e RV-11 chiuse), FT-08: file diversi, possono correre insieme; il documento comune `mzfinder-tabella.md` si unisce con `git merge origin/main` tenendo le righe di tutti.
- Onda 4: FT-09 dopo AT-04.
- Conflitti: M3 solo Rust del LR (nessuno); M4/M5/RV-11 toccano `20_tpmine.js`/`21_tpmine_hr.js` (FT-06/FT-07 in serie); MZ-04/MZ-05 toccano `32-35_pipeline*.js`, `37_feature_table.js`, `29_untarget_identity.js` e `crates/mzlab-feat` (FT-03/FT-05/FT-09 dopo); AT-02 tocca `livelli-1.*` (nessun FT li tocca: «L» e «Prev.» sono presentazione).

## 10. Trappole
- Tabui senza modalità amministratore non lascia traccia: nessun testo, chiave `lang/`, stile o file della pagina lo nomina.
- `tools/controlla_i18n.py` salta `TP_Mine/`: il catalogo si prova con `TP_Mine/tests/test_tabui_testi.py` (stesse chiavi in `it` e `en`, nessun testo vuoto).
- Gli e2e cercano `#hr-t`, `button.tpt-tg`, `tr.tpt-exp`, `#tp-det`, `#hr-kin`, `#hr-ms2f`, `#hr-cnt`, `#ft-g`, `#ft-n`: i selettori restano, o l'e2e si aggiorna nella stessa PR.
- `#tp-det` è unico e si sposta: il pannello si disegna dentro di lui; una riga virtualizzata aperta resta nel DOM anche fuori dalla finestra.
- «Export view» legge le righe dal modello, non dal DOM.
- `build_site.py` carica `TP_Mine/js/*.js` in ordine di nome: `40`-`45` vengono dopo `37` e prima di `50`; i nomi globali nuovi (`TT`, `TPHELP`, `TPRIGA`, `TPDEC`) non devono esistere già (`grep -rn`).
- Il `.wasm` della tabella ha il suo SHA-256 nel manifest: FT-03 lo ricostruisce con `tools/build_site.py` e aggiorna i tetti solo se la misura lo chiede.
