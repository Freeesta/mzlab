# Tooltip census

Every tooltip of the site (`mzlab/web`, Tabui `TP_Mine`, Flow `flow/lazy`): i18n keys of tips and titles in `lang/it` and `lang/en`, plus the literal (English-only) tooltips of Tabui and Flow. Built from `python3 tools/inventario_tooltip.py` (sites) and the catalogs (keys named `*.title`, `*.tip`, `tip.*`, `*.hint`). Mechanism and the generated per-area tables: `docs/inventari/TOOLTIP.md`.

## Rule (Federico)
A tooltip says only what the button or the click does, in one short sentence (at most 90 characters, a keyboard shortcut stays in parentheses). It never carries a privacy or policy statement («nothing is sent», «we do X instead of Y»), marketing, a tutorial, a restatement of the label, a «candidate, not an identification» disclaimer, or a dynamic value glued to the explanation. A warning is allowed only when the control itself is destructive (deletes, overwrites). Facts that were in a tooltip belong in the help panel or in the visible notes. `tests/test_tooltip_corti.py` enforces length and banned phrases (IT and EN) on every catalog key ending `.tip`, `.title` or `Title` and on the literal `title="..."` of the pages; the exemptions are in `tests/tooltip_ammessi.txt`, each with a reason (none today).

## Numbers
443 tooltip keys checked by the guard (IT and EN, same set). Second pass: 57 keys rewritten (keys unchanged), 9 literal tooltips of Tabui shortened or cleaned, 0 exemptions. Earlier pass: 153 shortened, 1 deleted (`canali.tip.time`). Chart hover labels that show data (`kmd.point.tip`, `pfas.point.tip`, `pca.sample.tip`, `mappa.lbl.tip`) carry values by design, not explanations.

## Channel picker
- The channel name no longer carries the scan mode: `MS¹ EMS` is shown as `MS¹`, with `EMS` in the «Experiment» column (`canali.js` `bare`). The engine name (`et` of `api/canali`, `mzlab-cli channels`) also omits the scan mode, so the legend, site, desktop and CLI show the same label; ids are unchanged.
- Row tooltip: only what the channel is (TIC, MS¹, data-dependent MS², BPC). The time range (`canali.tip.time`) and the raw engine ids are gone: time and points have their own columns.

## Exceptions
None: the six tips that carried a warning (`cmd.idn.tip`, `cmd.subref.tip`, `disp.pdaAlign.title`, `llm.on.title`, `xic.adduct.title`) now say only what the control does; the facts live in `lib.an.note`, `llm.off.hint` and the chromatogram help (`help.pnlChrom.body`). `load.demo.*.title` (list of the example files) and `int.tbl.par.title` (names the three parameters) stay: they describe what the click loads or shows.

## Follow-up
- SETTINGS-PANEL and ADD-LOSS keys are reviewed against the rule; `settings.hrPpm.title` and `predef.settings.title` already say only what the control does.

## Keys
Current = Italian text before this census; Len = its length in characters; Proposed = new IT / EN.

| Key | Where | Len | Verdict | Current (IT) | Proposed IT / EN |
|---|---|---|---|---|---|
| `aff.grip` | affianca.js:184 | 29 | keep | Sposta o affianca il pannello |  |
| `app.title` | built at runtime | 18 | keep | {app} • Analisi MS |  |
| `av.cancel.title` | avanzamento.js:38 | 23 | keep | Ferma questa operazione |  |
| `bar.addMap.title` | index.html:32 | 68 | shorten | Mappa di tutti gli ioni nel tempo: ogni pixel è un m/z a un certo RT | Ogni pixel è un m/z a un certo RT / Each pixel is an m/z at a given RT |
| `bar.addXic.title` | index.html:32 | 71 | shorten | Estrai il cromatogramma di uno ione (XIC): scegli tu la finestra di m/z | Cromatogramma di uno ione nella finestra di m/z che scegli / Chromatogram of one ion in the m/z window you choose |
| `bar.closeAll.title` | index.html:33 | 30 | keep | Chiudi tutti i pannelli aperti |  |
| `bar.method.title` | index.html:34 | 115 | shorten | Come sono stati acquisiti questi dati: tipo di esperimento, intervallo di massa, transizioni e parametri del metodo | Tipo di esperimento, intervallo di massa, transizioni e parametri del metodo / Experiment type, mass range, transitions and method settings |
| `bar.next` | index.html:31 | 32 | keep | File successivo (freccia destra) |  |
| `bar.prev` | index.html:31 | 34 | keep | File precedente (freccia sinistra) |  |
| `bgauto.back.title` | fondo-spettro.js:61 | 26 | keep | Torna al tratto automatico |  |
| `bgauto.pick.title` | fondo-spettro.js:63 | 160 | shorten | Scelto dal programma: dopo lo scarico e il fronte del solvente (fino a {lead} min), il tratto più quieto del TIC. Sceglierne un altro, o scrivere i due estremi. | Scelto in automatico: il tratto più quieto del TIC dopo {lead} min / Chosen automatically: the quietest stretch of the TIC after {lead} min |
| `blank.title` | built at runtime | 22 | keep | Sottrazione del bianco |  |
| `bn.add.title` | built at runtime | 28 | keep | Aggiungi un grafico impilato |  |
| `bn.addBtn.title` | banco.js:279 | 97 | shorten | Aggiungi un grafico impilato sullo stesso asse del tempo (TIC, BPC o XIC di un tipo di scansione) | Aggiunge un grafico impilato sullo stesso asse del tempo / Adds a chart stacked on the same time axis |
| `bn.drop` | banco.js:285 | 24 | keep | Togli il grafico: {name} |  |
| `bn.f.placeholder` | banco-file.js:41 | 30 | keep | Segnaposto registrato nel file |  |
| `bn.filt.title` | banco.js:260 | 107 | shorten | Tipo di scansione di questa cella (filtro di scansione): ogni tipo ha il suo cromatogramma e i suoi spettri | Filtro di scansione: ogni tipo ha il suo cromatogramma e i suoi spettri / Scan filter: each type has its own chromatogram and spectra |
| `bn.idn.dl` | ident-catalogo.js:25 | 14 | keep | Scarica {name} |  |
| `bn.idn.dlNo` | ident-catalogo.js:26 | 29 | keep | Formato non ancora supportato |  |
| `bn.idn.site` | ident-catalogo.js:27 | 22 | keep | Apri il sito di {name} |  |
| `bn.iso.cdTitle` | banco.js:456 | 128 | shorten | Più stati di carica (protonati o deprotonati) pesati con una curva centrata sulla carica più abbondante; vuoto = una sola carica | Più stati di carica pesati attorno al più abbondante; vuoto = una carica / Several charge states weighted around the most abundant; empty = one charge |
| `bn.iso.defTitle` | banco.js:461 | 36 | keep | Come è definita la larghezza scritta |  |
| `bn.iso.delAd` | banco.js:449 | 20 | keep | Togli questo addotto |  |
| `bn.iso.hint` | built at runtime | 120 | n/a (visible text) | Risoluzione vuota = quella della scansione; 0 = tutta la struttura fine. Potere risolutivo: scende con la radice di m/z. |  |
| `bn.iso.newTitle` | banco.js:462 | 43 | keep | Il profilo in una cella nuova, sotto questa |  |
| `bn.iso.overTitle` | banco.js:462 | 47 | keep | Disegna il profilo sopra lo spettro della cella |  |
| `bn.iso.ptsTitle` | banco.js:458 | 39 | keep | Punti per ogni larghezza a metà altezza |  |
| `bn.iso.repTitle` | banco.js:462 | 58 | keep | Al posto dello spettro osservato resta solo la simulazione |  |
| `bn.l.click` | banco.js:408 | 49 | keep | Clic su una riga: la cella va a quella scansione. |  |
| `bn.more` | banco.js:580 | 24 | keep | Altre azioni della barra |  |
| `bn.norm.title` | banco.js:283 | 89 | shorten | Porta ogni grafico impilato sulla scala 0-100 (l'intensità massima resta scritta come NL) | Scala 0–100 per ogni grafico; il massimo resta scritto come NL / 0–100 scale for each chart; the maximum stays written as NL |
| `bn.pin.off` | banco.js:291 | 88 | shorten | Fissa la cella (puntina): uno spettro fissato non segue più il cursore del cromatogramma | Fissa la cella: lo spettro non segue più il cursore / Pin the cell: the spectrum stops following the cursor |
| `bn.pin.on` | banco.js:291 | 66 | shorten | Cella fissata (puntina): non segue il cursore. Clic per sganciarla | Cella fissata: clic per sganciarla / Cell pinned: click to unpin it |
| `bn.scan.title` | banco.js:258 | 66 | shorten | Apri una scansione: tutte le scansioni del file in ordine di tempo | Tutte le scansioni del file in ordine di tempo / Every scan of the file in time order |
| `bn.stack.title` | banco.js:282 | 81 | shorten | Il tipo di cromatogramma è scelto per ogni grafico (pulsante «× …» per toglierlo) | Ogni grafico ha il suo tipo di cromatogramma; «×» lo toglie / Each chart has its own chromatogram type; «×» removes it |
| `bn.sub.title` | built at runtime | 37 | keep | XIC da formula: tutte le sottoformule |  |
| `calc.back` | index.html:100 | 27 | keep | Cancella l'ultimo carattere |  |
| `calc.clear` | index.html:100 | 6 | keep | Azzera |  |
| `calc.delta.title` | calc-addotti-html.js:54 | 42 | keep | Scrivi la differenza tra due m/z osservati |  |
| `calc.eq` | index.html:104 | 17 | keep | Risultato (Invio) |  |
| `calc.in.title` | index.html:94 | 191 | follow-up (ADD-LOSS) | Un conto: numeri e + − × ÷ ( ), anche con la virgola. Una formula bruta: anche con parentesi (C9H10N2O) e in minuscolo (c9h10n2o). «Ans» = l'ultimo risultato. Invio = risultato, Esc = azzera. |  |
| `calc.nl.title` | index.html:96 | 52 | keep | Cerca questo valore nel pannello Perdite neutre (Δm) |  |
| `calc.tape.title` | eventi.js:613 | 54 | keep | Rimette il risultato nel campo per continuare il conto |  |
| `calc.toLosses` | eventi.js:648 | 42 | keep | Cerca Δm = {v} nel pannello Perdite neutre |  |
| `calc.xic.title` | calc-addotti-html.js:35 | 24 | keep | Estrae questo ione (XIC) |  |
| `canali.btn.title` | canali.js:172 | 42 | keep | Scegli i canali da tracciare per ogni file |  |
| `canali.col.n.title` | canali.js:167 | 38 | keep | Numero di punti (scansioni) del canale |  |
| `canali.col.rt.title` | canali.js:167 | 41 | keep | Intervallo di tempo del canale, in minuti |  |
| `canali.files.title` | canali.js:139 | 24 | keep | File che hanno il canale |  |
| `canali.miss.title` | canali.js:276 | 28 | keep | Questo file non ha il canale |  |
| `canali.sgl.EMC` | built at runtime | 38 | keep | Scansione a carica multipla potenziata |  |
| `canali.sgl.ER` | built at runtime | 34 | keep | Scansione a risoluzione potenziata |  |
| `canali.sgl.NG` | built at runtime | 45 | keep | Scansione di acquisto neutro (acquisto fisso) |  |
| `canali.sgl.NL` | built at runtime | 43 | keep | Scansione di perdita neutra (perdita fissa) |  |
| `canali.sgl.PREC` | built at runtime | 50 | keep | Scansione di ioni precursori (ione prodotto fisso) |  |
| `canali.sgl.TDF` | built at runtime | 36 | keep | Scansione a frammentazione ritardata |  |
| `canali.tip.dd` | built at runtime | 81 | shorten | TIC delle scansioni di frammentazione dell'acquisizione dipendente dai dati (MS²) | TIC delle MS² dipendenti dai dati / TIC of the data-dependent MS² scans |
| `canali.tip.ms1` | built at runtime | 45 | shorten | TIC delle sole scansioni di rilevamento (MS¹) | TIC delle sole scansioni MS¹ / TIC of the MS¹ scans only |
| `canali.tip.tic` | built at runtime | 54 | shorten | Corrente ionica totale: la somma di tutte le scansioni | Corrente ionica totale di tutte le scansioni / Total ion current of all scans |
| `canali.tip.time` | built at runtime | 17 | delete | Tempo {a}–{b} min | (time range has its own column; text and key removed) |
| `cart.open` | index.html:53 | 13 | keep | Apri cartella |  |
| `cmd.addloss.tip` | built at runtime | 122 | follow-up (ADD-LOSS) | Addotti di una formula o di un m/z osservato, differenze tra addotti, perdite neutre più comuni e differenze di massa (Δm) |  |
| `cmd.alb.tip` | built at runtime | 48 | keep | Albero dei precursori di un file d'infusione MSn |  |
| `cmd.bio.tip` | built at runtime | 38 | keep | Apre BioTransformer in un'altra scheda |  |
| `cmd.calc.tip` | built at runtime | 22 | keep | Calcolatrice e addotti |  |
| `cmd.chrom.tip` | built at runtime | 46 | keep | Apre un pannello cromatogramma sotto gli altri |  |
| `cmd.closeall.tip` | built at runtime | 42 | keep | Chiude tutti i pannelli aperti dei grafici |  |
| `cmd.cmp.tip` | built at runtime | 69 | keep | Formule compatibili con un m/z, con errore, prove e isotopi osservati |  |
| `cmd.disp.affianca.tip` | built at runtime | 62 | keep | Mette il pannello in uso accanto a un altro, nella stessa riga |  |
| `cmd.disp.giu.tip` | built at runtime | 52 | keep | Sposta il pannello in uso di una riga verso il basso |  |
| `cmd.disp.intera.tip` | built at runtime | 65 | keep | Il pannello in uso lascia la sua riga e occupa tutta la larghezza |  |
| `cmd.disp.name` | banco.js:289 | 26 | keep | Opzioni di visualizzazione |  |
| `cmd.disp.ripristina.tip` | built at runtime | 53 | keep | Ogni pannello torna nella sua riga, a tutta larghezza |  |
| `cmd.disp.su.tip` | built at runtime | 50 | keep | Sposta il pannello in uso di una riga verso l'alto |  |
| `cmd.disp.tip` | built at runtime | 81 | shorten | Opzioni di visualizzazione del grafico attivo (decimali di m/z, etichette, scala) | Decimali di m/z, etichette e scala del grafico attivo / m/z decimals, labels and scale of the active chart |
| `cmd.disp.uguali.tip` | built at runtime | 83 | shorten | I pannelli affiancati nella riga del pannello in uso tornano della stessa larghezza | Stessa larghezza ai pannelli affiancati di questa riga / Same width for the side-by-side panels of this row |
| `cmd.fil.tip` | built at runtime | 43 | keep | File e strumento: ciò che dice il file mzML |  |
| `cmd.file.tip` | built at runtime | 60 | shorten | Elenco dei file caricati: scegli, nascondi, rinomina, colora | Scegli, nascondi, rinomina e colora i file caricati / Choose, hide, rename and colour the loaded files |
| `cmd.form.tip` | built at runtime | 56 | shorten | Formule: composizione elementare e simulazione isotopica | Composizione elementare e simulazione isotopica / Elemental composition and isotope simulation |
| `cmd.gra.tip` | built at runtime | 50 | keep | Frammenti di una molecola nota in un'infusione MSn |  |
| `cmd.gradient.tip` | built at runtime | 84 | shorten | Mostra o nasconde il gradiente %B sopra il cromatogramma attivo, con l'asse a destra | Gradiente %B sopra il cromatogramma attivo, con l'asse a destra / %B gradient over the active chromatogram, axis on the right |
| `cmd.hdr.tip` | built at runtime | 45 | keep | Intestazione della scansione sotto il cursore |  |
| `cmd.idn.tip` | built at runtime | 99 | shorten | Cerca lo spettro nelle librerie e identifica tutte le MS2: candidati con prove, mai identificazioni | Cerca ogni MS2 nelle librerie: candidati con prove, non identificazioni / Library search of every MS2: candidates with evidence, not identifications |
| `cmd.iso.tip` | built at runtime | 75 | keep | Profilo isotopico simulato di una formula, alla risoluzione dello strumento |  |
| `cmd.labcont.tip` | built at runtime | 68 | keep | Elenco dei contaminanti del laboratorio (importa, esporta, aggiungi) |  |
| `cmd.lists.tip` | built at runtime | 65 | keep | Liste di riferimento e contaminanti noti (laboratorio e sospetti) |  |
| `cmd.lst.tip` | built at runtime | 31 | keep | Elenco delle scansioni del file |  |
| `cmd.map.tip` | built at runtime | 68 | shorten | Mappa di tutti gli ioni nel tempo: ogni pixel è un m/z a un certo RT | Ogni pixel è un m/z a un certo RT / Each pixel is an m/z at a given RT |
| `cmd.map3mz.tip` | built at runtime | 63 | shorten | Vista di lato: intensità contro m/z, come uno spettro (tasto 3) | Intensità contro m/z, come uno spettro (tasto 3) / Intensity against m/z, like a spectrum (key 3) |
| `cmd.map3rt.tip` | built at runtime | 67 | shorten | Vista di lato: intensità contro RT, come un cromatogramma (tasto 2) | Intensità contro RT, come un cromatogramma (tasto 2) / Intensity against RT, like a chromatogram (key 2) |
| `cmd.map3top.tip` | built at runtime | 44 | keep | Guarda il piano RT × m/z dall'alto (tasto 1) |  |
| `cmd.mapcopy.tip` | built at runtime | 47 | keep | Copia RT, m/z e intensità del mirino come testo |  |
| `cmd.mapfeat.tip` | built at runtime | 142 | shorten | Solo alta risoluzione: segna sulla mappa le feature che Tabui ha già trovato in questo file; un clic mette il riquadro su una (spento di base) | Le feature di Tabui di questo file sulla mappa (alta risoluzione) / This file's Tabui features on the map (high resolution) |
| `cmd.maphelp.tip` | built at runtime | 74 | keep | Tabella dei gesti col mouse, il trackpad, il tocco e la tastiera (tasto ?) |  |
| `cmd.maplock.tip` | built at runtime | 78 | shorten | Ferma il mirino su questo punto: il mouse è libero e XIC e spettro restano (L) | Ferma il mirino qui: XIC e spettro restano (L) / Freezes the sight here: XIC and spectrum stay (L) |
| `cmd.mapmark.tip` | built at runtime | 95 | shorten | Segna questo punto sulla mappa con il suo numero, RT e m/z nella tabella (come i punti marcati) | Segna il punto con numero, RT e m/z nella tabella / Marks the point with number, RT and m/z in the table |
| `cmd.mapreset.tip` | built at runtime | 90 | shorten | Riporta entrambi gli assi all'intervallo con dati (tasto 0, Home o Backspace; doppio clic) | Assi sull'intervallo con dati (0, Home o doppio clic) / Axes back to the data range (0, Home or double click) |
| `cmd.mapspec.tip` | built at runtime | 91 | shorten | Spettro di massa della scansione più vicina all'RT del mirino, in un nuovo pannello spettro | Spettro della scansione più vicina al mirino, in un nuovo pannello / Spectrum of the scan closest to the sight, in a new panel |
| `cmd.mapxic.tip` | built at runtime | 99 | shorten | XIC dell'm/z del mirino (finestra unitaria, o in ppm in alta risoluzione), in un nuovo pannello XIC | XIC dell'm/z del mirino, in un nuovo pannello / XIC of the sight's m/z, in a new panel |
| `cmd.mapzin.tip` | built at runtime | 39 | keep | Ingrandisce la mappa (tasto +; rotella) |  |
| `cmd.mapzout.tip` | built at runtime | 34 | keep | Riduce la mappa (tasto −; rotella) |  |
| `cmd.marker.tip` | built at runtime | 70 | keep | Mostra o nasconde i triangoli delle scansioni MS² sullo spettro attivo |  |
| `cmd.method.tip` | built at runtime | 115 | shorten | Come sono stati acquisiti questi dati: tipo di esperimento, intervallo di massa, transizioni e parametri del metodo | Tipo di esperimento, intervallo di massa, transizioni e parametri del metodo / Experiment type, mass range, transitions and method settings |
| `cmd.more.name` | barra.js:518 | 15 | keep | Altri strumenti |  |
| `cmd.more.tip` | built at runtime | 76 | shorten | Altri strumenti: PubChem, BioTransformer e quelli che non stanno nella barra | PubChem, BioTransformer e gli strumenti che non stanno nella barra / PubChem, BioTransformer and the tools that do not fit in the bar |
| `cmd.mrm.tip` | built at runtime | 39 | keep | Apre un pannello con le transizioni MRM |  |
| `cmd.nocont.tip` | built at runtime | 88 | shorten | Toglie dai picchi dello spettro attivo quelli della lista dei contaminanti (annullabile) | Toglie dallo spettro attivo i picchi dei contaminanti (annullabile) / Removes contaminant peaks from the active spectrum (undoable) |
| `cmd.openms2.tip` | built at runtime | 58 | keep | Apre la scansione MS² del picco scelto con il tasto destro |  |
| `cmd.panel.tip` | index.html:13 | 49 | keep | Mostra o nasconde il pannello laterale (Ctrl/⌘+B) |  |
| `cmd.pressure.tip` | built at runtime | 93 | shorten | Mostra o nasconde la pressione della pompa sopra il cromatogramma attivo, con l'asse a destra | Pressione della pompa sopra il cromatogramma attivo, con l'asse a destra / Pump pressure over the active chromatogram, axis on the right |
| `cmd.ptable.tip` | built at runtime | 57 | keep | Tavola periodica con masse esatte e abbondanze isotopiche |  |
| `cmd.pubchem.tip` | built at runtime | 84 | shorten | Apre PubChem in un'altra scheda: cerca il tuo composto per nome, formula o struttura | Cerca il composto per nome, formula o struttura, in un'altra scheda / Look up the compound by name, formula or structure, in another tab |
| `cmd.qaqc.tip` | built at runtime | 105 | shorten | Bianchi, standard interni, deriva, accuratezza di massa e rapporto campione/bianco (misure, non verdetti) | Bianchi, standard interni, deriva e accuratezza di massa: misure, non verdetti / Blanks, internal standards, drift and mass accuracy: measures, not verdicts |
| `cmd.report.tip` | built at runtime | 67 | keep | Rapporto di studio stampabile (HTML) con file, strumento, parametri |  |
| `cmd.scan.tip` | built at runtime | 65 | shorten | Scansione: intestazione, elenco delle scansioni, file e strumento | Intestazione, elenco delle scansioni, file e strumento / Header, scan list, file and instrument |
| `cmd.search.tip` | built at runtime | 62 | keep | Cerca un numero, una formula o un comando (Ctrl+K, ⌘K sul Mac) |  |
| `cmd.settings.tip` | built at runtime | 48 | keep | Tema, lingua, dimensione del testo, tour guidato |  |
| `cmd.sideL.tip` | built at runtime | 53 | keep | Sposta il pannello laterale a sinistra (Alt+Maiusc+←) |  |
| `cmd.sideR.tip` | built at runtime | 51 | keep | Sposta il pannello laterale a destra (Alt+Maiusc+→) |  |
| `cmd.smooth.tip` | built at runtime | 94 | shorten | Attiva o disattiva lo smoothing del cromatogramma attivo (cambia solo il disegno, mai le aree) | Smoothing del cromatogramma attivo: cambia il disegno, mai le aree / Smoothing of the active chromatogram: changes the drawing, never the areas |
| `cmd.spec.tip` | built at runtime | 49 | keep | Apre un pannello spettro di massa sotto gli altri |  |
| `cmd.stat.tip` | built at runtime | 94 | shorten | Metadati dei campioni, profilo nel tempo di uno ione, sottrazione del bianco e PCA esplorativa | Metadati, profilo di uno ione nel tempo, sottrazione del bianco e PCA / Metadata, time profile of an ion, blank subtraction and PCA |
| `cmd.subref.tip` | built at runtime | 124 | shorten | Mostra in tutti i grafici la differenza tra ogni file e il riferimento (il bianco o il primo tempo); un altro clic la toglie | Differenza di ogni file dal riferimento (bianco o primo tempo), ovunque / Difference of each file from the reference (blank or first time), everywhere |
| `cmd.xic.tip` | built at runtime | 71 | shorten | Estrai il cromatogramma di uno ione (XIC): scegli tu la finestra di m/z | Cromatogramma di uno ione nella finestra di m/z che scegli / Chromatogram of one ion in the m/z window you choose |
| `common.close` | fair.js:138, libreria-finestra.js:91 | 6 | keep | Chiudi |  |
| `common.help` | conversione.js:269, picchi.js:240 | 5 | keep | Aiuto |  |
| `comp.det.title` | built at runtime | 41 | keep | termini del punteggio isotopico per picco |  |
| `comp.elements.hint` | built at runtime | 99 | n/a (visible text) | (della formula dello ione, es. C:0-13,H:0-25; isotopi come elementi: 13C:0-2, 15N, 34S, 37Cl, 81Br) |  |
| `comp.filters.hint` | built at runtime | 39 | keep | (Seven Golden Rules, Kind e Fiehn 2007) |  |
| `comp.title` | built at runtime | 19 | keep | Formule compatibili |  |
| `cont.excl.back` | liste.js:684 | 16 | keep | Includi di nuovo |  |
| `conv.cancel.title` | conversione-carta.js:18 | 36 | keep | Annulla la conversione dei file .raw |  |
| `conv.dl.title` | conversione.js:22, shell.js:110 | 101 | shorten | Salva l'mzML convertito: è quello che il programma usa, e riaperto si legge senza convertire di nuovo | Salva l'mzML convertito: riaperto non va riconvertito / Save the converted mzML: reopened, it needs no new conversion |
| `conv.dlall` | conversione.js:270 | 22 | keep | Scarica tutti gli mzML |  |
| `conv.help.hint` | built at runtime | 43 | n/a (visible text) | Hai caricato file {types}: convertili così. |  |
| `conv.q.saveTitle` | conversione.js:270 | 55 | keep | Scegli una cartella: ci vanno tutti gli mzML già pronti |  |
| `corr.button.title` | built at runtime | 37 | keep | Correzione del fondo (una alla volta) |  |
| `corr.file.pick` | pannelli.js:589 | 17 | keep | File da sottrarre |  |
| `corr.fileNone.desc` | pannelli.js:591 | 23 | keep | Servono almeno due file |  |
| `corr.fondo.title` | built at runtime | 16 | keep | Sottrai il fondo |  |
| `corr.info.title` | built at runtime | 28 | keep | Correzioni attive: riepilogo |  |
| `corr.snipw.title` | pannelli.js:683 | 58 | shorten | Larghezza della finestra: deve essere più larga dei picchi | Deve essere più larga dei picchi / It must be wider than the peaks |
| `corr.sub.title` | built at runtime | 15 | keep | Sottrai un file |  |
| `cq.ad.title` | built at runtime | 7 | keep | Addotti |  |
| `cq.ad.title.field` | calc-qui-pannello.js:19 | 60 | keep | Scrivi un numero (anche un conto come 100+114) o una formula |  |
| `ctl.cpar.titleMap` | pannelli.js:709, pannelli.js:709 | 111 | shorten | Opzioni della mappa: scala e palette dei colori, soglia del rumore, normalizzazione e cosa mostra la differenza | Scala e palette dei colori, soglia del rumore, normalizzazione e differenza / Colour scale and palette, noise threshold, normalisation and difference |
| `ctl.files.title` | pannelli.js:684 | 87 | shorten | Scegli al volo quali file mostrare (vale per tutti i grafici, come la lista a sinistra) | File mostrati in tutti i grafici, come la lista a sinistra / Files shown in all the charts, like the list on the left |
| `ctl.kind.bpc.title` | built at runtime | 112 | shorten | Picco base: lo ione più intenso di ogni scan. Non esiste nei file MRM (si registrano solo le transizioni scelte) | Picco base: lo ione più intenso di ogni scansione / Base peak: the most intense ion of each scan |
| `ctl.kind.pda.title` | pannelli.js:710 | 89 | shorten | Segnale del rivelatore a serie di diodi (PDA/UV): non dipende dallo spettrometro di massa | Rivelatore a serie di diodi (UV), indipendente dallo spettrometro / Photodiode array detector (UV), independent of the mass spectrometer |
| `ctl.kind.reg.title` | built at runtime | 77 | keep | Temperature, tensioni, vuoto e campi per scansione registrati dallo strumento |  |
| `ctl.kind.title` | canali.js:172 | 21 | keep | Tipo di cromatogramma |  |
| `ctl.log.title` | pannelli.js:270, pannelli.js:802 | 17 | keep | Scala logaritmica |  |
| `ctl.map.file` | pannelli.js:711 | 16 | keep | File da mostrare |  |
| `ctl.map.follow` | pannelli.js:711 | 109 | shorten | La mappa segue il file scelto nella barra (un solo file spuntato): scegliendone un altro qui cambia anche lì. | Segue il file scelto nella barra: cambiarlo qui lo cambia anche lì / Follows the file chosen in the bar: changing it here changes it there too |
| `ctl.map.level.title` | pannelli.js:710 | 64 | shorten | Livello di scansione della mappa (MS1: full scan; MS2: prodotti) | MS1: full scan; MS2: ioni prodotto / MS1: full scan; MS2: product ions |
| `ctl.map.norm.need` | pannelli.js:712 | 64 | keep | Serve solo per la differenza: scegli prima il file da sottrarre. |  |
| `ctl.map.norm.title` | pannelli.js:712 | 196 | shorten | Prima di sottrarre due esperimenti: «al massimo» divide ogni mappa per il suo punto più intenso; «al totale» per la somma delle intensità (per mille); «assoluta» sottrae i conteggi così come sono. | Prima della differenza: dividi per il massimo, per il totale o niente / Before subtracting: divide by the maximum, by the total or not at all |
| `ctl.map.open3d.title` | mappa.js:974 | 74 | keep | Apre il visore 3D a tutto schermo con il file e la regione di questa mappa |  |
| `ctl.map.scale.title` | pannelli.js:713 | 65 | shorten | Scala dei colori: la radice quadrata fa emergere i segnali deboli | La radice quadrata fa emergere i segnali deboli / The square root brings out weak signals |
| `ctl.mode.title` | pannelli.js:663 | 169 | shorten | Come disegnare più tracce: una sopra l'altra; una per riga; a cascata, in ordine di tempo, la prima davanti (solo per guardare: senza integrazione né lettura dei valori) | Sovrapposte, una per riga o a cascata (la cascata è solo da guardare) / Overlaid, one per row or cascaded (cascade is for viewing only) |
| `ctl.mrm.smooth.title` | built at runtime | 154 | shorten | Spento: le aree si calcolano sul segnale grezzo. Lo smoothing cambia solo il disegno, mai le aree integrate (nell'MRM si integra, per questo parte spento) | Cambia solo il disegno: le aree restano sul segnale grezzo / Changes only the drawing: areas stay on the raw signal |
| `ctl.ms2tr.title` | pannelli.js:689 | 103 | shorten | Transizione precursore > frammento: cromatogramma del frammento nelle sole scansioni di quel precursore | Cromatogramma del frammento nelle sole scansioni di quel precursore / Chromatogram of the fragment in the scans of that precursor only |
| `ctl.ms2tri.title` | built at runtime | 161 | shorten | Un triangolino in alto ogni volta che è partita una scansione MS2 (file con survey e MS2 insieme). Clic su un triangolino: apre quello spettro MS2 nel blocco MS2 | Un triangolo per ogni MS2 acquisita: clic per aprirla / One triangle per acquired MS2: click to open it |
| `ctl.mz.title` | pannelli.js:670 | 180 | shorten | Mostra il cromatogramma costruito solo con gli ioni in questo intervallo di m/z. Vuoto = tutti gli ioni. Serve per togliere dal TIC gli m/z che non ti interessano (solvente, fondo) | Solo gli ioni in questo intervallo di m/z; vuoto = tutti / Only the ions in this m/z range; empty = all |
| `ctl.noMs2` | pannelli.js:802 | 101 | shorten | Non si applica al cromatogramma dei precursori MS2: contiene solo gli scan MS2 del precursore scelto. | Non vale per il cromatogramma dei precursori MS2. / Does not apply to the MS2 precursor chromatogram. |
| `ctl.prec.title` | pannelli.js:676 | 91 | shorten | File MS2 (ioni prodotto): somma di tutti i precursori oppure solo gli scan di un precursore | Somma di tutti i precursori o le scansioni di uno solo / Sum of all precursors or the scans of one only |
| `ctl.range.clear` | pannelli.js:687 | 19 | keep | Azzera l'intervallo |  |
| `ctl.range.note` | pannelli.js:687 | 66 | shorten | Vuoto = tutto. Il precursore sceglie gli scan MS2 di un solo ione. | Vuoto = tutto; il precursore sceglie le MS2 di un solo ione. / Empty = everything; the precursor picks the MS2 scans of one ion. |
| `ctl.range.title` | pannelli.js:687 | 37 | keep | Intervallo di m/z e di tempo mostrato |  |
| `ctl.rt.title` | pannelli.js:671 | 134 | shorten | Intervallo di tempo mostrato (come ingrandire sull'asse del tempo). Scrivi i due estremi e premi Invio; vuoti = tutto il cromatogramma | Due estremi e Invio; vuoti = tutto il cromatogramma / Two ends and Enter; empty = the whole chromatogram |
| `ctl.smooth.title` | pannelli.js:270 | 64 | shorten | Smoothing: ammorbidisce la curva mostrata (le aree non cambiano) | Ammorbidisce la curva mostrata; le aree non cambiano / Smooths the curve shown; areas do not change |
| `ctl.spec.overlay` | barra-spettro.js:7 | 17 | keep | Sovrapponi i file |  |
| `ctl.spec.rt` | barra-spettro.js:10 | 19 | keep | Tempo di ritenzione |  |
| `ctl.split.title` | pannelli.js:698 | 25 | keep | Un pannello per ogni ione |  |
| `ctl.wl.title` | eventi.js:259 | 72 | shorten | Intervallo di lunghezze d'onda (nm): media dei canali del rivelatore DAD | Media dei canali DAD in questo intervallo di nm / Mean of the DAD channels in this nm range |
| `ctl.xlab.title` | pannelli.js:697 | 101 | shorten | Ione estratto. Il valore di m/z e l'intervallo (ppm o Da) si cambiano in «Opzioni di visualizzazione» | m/z e tolleranza si cambiano in «Opzioni di visualizzazione» / m/z and tolerance are changed in «Display options» |
| `ctl.xr.title` | pannelli.js:697 | 68 | shorten | Intervallo di m/z estratto (XIC). Scrivi i due estremi e premi Invio | Due estremi di m/z e Invio / Two m/z ends and Enter |
| `ctl.xt.title` | pannelli.js:695 | 20 | keep | Quale XIC modificare |  |
| `ctx.chrom.ms2AtRt.tip` | libreria.js:425 | 173 | shorten | Apre la scansione MS2 più vicina a questo tempo di ritenzione. Per scegliere tu il precursore, clicca il suo triangolino sul cromatogramma o la bandierina ▼ nella Full Scan. | Apre la MS2 più vicina a questo RT / Opens the MS2 closest to this RT |
| `ctx.spec.overlay.tip` | built at runtime | 90 | shorten | Aggiunge questo ione nello stesso grafico: per vedere se due ioni escono allo stesso tempo | Nello stesso grafico, per vedere se due ioni coeluiscono / In the same chart, to see whether two ions co-elute |
| `cx.expand` | elementare-pannello.js:55 | 26 | follow-up (ADD-LOSS) | Espandi: tutti i parametri |  |
| `cx.load.tip` | elementare-pannello.js:53 | 53 | keep | Sostituisce gli elementi qui sotto con questo insieme |  |
| `cx.pt.hint` | built at runtime | 110 | n/a (visible text) | Clic su un elemento: lo aggiunge agli elementi in uso o lo toglie (sono attivi quelli che il calcolo conosce). |  |
| `cx.remove` | elementare.js:45, elementare.js:48 | 10 | keep | Togli {el} |  |
| `cx.save.tip` | elementare-pannello.js:53 | 50 | keep | Salva gli elementi qui sotto come insieme con nome |  |
| `cx.score.tip` | built at runtime | 254 | follow-up (ADD-LOSS) | Punteggio (0–1, più alto è meglio; la lista è ordinata per questo): media pesata del termine di massa (una gaussiana dell'errore, σ = {s} ppm), dell'accordo del profilo isotopico e della quota di MS2 spiegata, con pesi {w}; un termine mancante è escluso. |  |
| `cx.title` | built at runtime | 23 | keep | Composizione elementare |  |
| `dda.add.tip` | dda.js:354 | 123 | shorten | Affianca un altro pannello MS2 sotto la stessa Full Scan: la prossima scelta (▼ o clic su una barra) va nel pannello attivo | Un altro pannello MS2 sotto la stessa Full Scan; la prossima scelta va lì / Another MS2 panel under the same Full Scan; the next choice goes there |
| `dda.avg.tip` | dda.js:352 | 91 | shorten | Media delle MS2 di questo ione entro ±{min} min (al massimo {max}, le più vicine nel tempo) | Media delle MS2 di questo ione entro ±{min} min (al massimo {max}) / Mean of the MS2 of this ion within ±{min} min (at most {max}) |
| `dda.avgTitle` | dda.js:300 | 23 | keep | Media di {n} MS2 · {mz} |  |
| `dda.follow.tip` | dda.js:355 | 114 | shorten | Crea l'XIC di questo ione: il cromatogramma mostra dove lo strumento ha preso le MS2 e con ← → nella MS2 le scorri | XIC di questo ione con i punti delle MS2; ← → le scorre / XIC of this ion with the MS2 points; ← → steps through them |
| `dda.kid.hint` | built at runtime | 23 | n/a (visible text) | Clic: mostra questa MS2 |  |
| `dda.kid.title` | built at runtime | 37 | keep | MS2 n. {i} di {n} da questa scansione |  |
| `dda.list.title` | dda.js:373 | 137 | shorten | Tutti gli ioni di cui lo strumento ha preso una MS2: m/z, tempo e numero di MS2. Un clic su una riga mostra la sua MS2 sotto la Full Scan | Ioni con una MS2: m/z, tempo e numero di MS2; clic per vederla / Ions with an MS2: m/z, time and number of MS2; click to see it |
| `dda.near.hint` | built at runtime | 50 | n/a (visible text) | Clic: la più vicina nel tempo. Tasto destro: media |  |
| `dda.near.title` | built at runtime | 33 | keep | Frammentato in un'altra scansione |  |
| `dda.show.tip` | dda.js:431 | 87 | shorten | Apre il pannello con la MS2 sotto la Full Scan: ▼ sui picchi, un clic mostra la sua MS2 | MS2 sotto la Full Scan: ▼ sui picchi, clic per vederla / MS2 under the Full Scan: ▼ on the peaks, click to see it |
| `disp.dots.title` | spettro.js:157 | 94 | shorten | Mostra solo i punti dello spettro, senza linea né bastoncini: i picchi isolati restano isolati | Solo i punti, senza linea né bastoncini / Points only, no line or sticks |
| `disp.gdelay.title` | pannelli.js:667 | 200 | shorten | Tempo che il gradiente impiega ad arrivare alla colonna (volume morto): lo imposti tu, il programma non lo stima. Con un ritardo si vedono la curva programmata (sottile) e quella alla colonna (piena). | Tempo del gradiente per arrivare alla colonna (volume morto), da impostare / Time the gradient takes to reach the column (dwell volume), set by you |
| `disp.marker.title` | built at runtime | 115 | shorten | Spenti, il triangolo compare solo sul picco sotto il mouse o il dito; di base si accendono da soli fino a 20 picchi | Spenti, il triangolo compare solo sul picco sotto il mouse / Off, the triangle shows only on the peak under the mouse |
| `disp.noCont.title` | built at runtime | 147 | shorten | Toglie dai picchi mostrati quelli compatibili con la lista dei contaminanti, con la tolleranza in ppm del Banco; riaccendi la casella per rivederli | Nasconde i picchi compatibili con i contaminanti (tolleranza del Banco) / Hides peaks matching the contaminants (Banco tolerance) |
| `disp.pdaAlign.title` | built at runtime | 110 | shorten | Sposta l'asse del tempo del PDA del ritardo, solo sullo schermo: i dati e i risultati restano quelli del file. | Sposta l'asse del PDA solo sullo schermo; i dati del file non cambiano / Shifts the PDA time axis on screen only; the file's data do not change |
| `disp.pdaDelay.title` | pannelli.js:668 | 159 | shorten | Il PDA sta in serie prima dello spettrometro di massa: i picchi arrivano alla massa un po' dopo. Il valore è misurato dai picchi del file e lo puoi correggere. | Ritardo del PDA rispetto alla massa, misurato dai picchi del file / PDA delay to the mass spectrometer, measured from the file's peaks |
| `disp.xmz.title` | built at runtime | 52 | keep | Valore di m/z al centro della finestra di estrazione |  |
| `disp.xtol.title` | built at runtime | 69 | keep | Mezza larghezza della finestra di estrazione, nell'unità scelta sopra |  |
| `draw.ann.tip` | built at runtime | 44 | keep | Doppio clic: modifica · tasto destro: azioni |  |
| `draw.bar.charge` | disegno-barra.js:17 | 6 | keep | Carica |  |
| `draw.bar.charge.plus` | disegno-barra.js:18 | 8 | keep | Carica + |  |
| `draw.bar.ring.benzene` | disegno-barra.js:14 | 7 | keep | Benzene |  |
| `draw.bar.ring.cyclobutane` | disegno-barra.js:15 | 11 | keep | Ciclobutano |  |
| `draw.bar.ring.cycloheptane` | disegno-barra.js:16 | 11 | keep | Cicloeptano |  |
| `draw.bar.ring.cyclohexane` | disegno-barra.js:14 | 10 | keep | Cicloesano |  |
| `draw.bar.ring.cyclopentane` | disegno-barra.js:15 | 12 | keep | Ciclopentano |  |
| `draw.bar.ring.library` | disegno-barra.js:16 | 21 | keep | Libreria di strutture |  |
| `draw.bar.rings` | disegno-barra.js:13 | 6 | keep | Anelli |  |
| `draw.center.title` | draw.js:155 | 28 | keep | Centra il disegno nella tela |  |
| `draw.copy.title` | draw.js:391 | 19 | keep | Copia negli appunti |  |
| `draw.ctx.breakBonds.title` | draw.js:203 | 85 | shorten | Cancella il legame (non gli atomi): i due frammenti hanno ciascuno la propria formula | Toglie il legame, non gli atomi: ogni pezzo ha la sua formula / Removes the bond, not the atoms: each piece gets its own formula |
| `draw.examples.title` | built at runtime | 17 | keep | Esempi di disegno |  |
| `draw.export.fileName.title` | index.html:84 | 147 | shorten | Nome dei file scaricati (senza estensione). Predefinito: disegno_data_ora. Solo lettere minuscole senza accento, numeri, trattino e trattino basso. | Senza estensione: minuscole senza accento, cifre, - e _ / Without extension: lowercase without accents, digits, - and _ |
| `draw.export.jpegNoAlpha` | draw.js:751 | 94 | shorten | Il JPEG non supporta la trasparenza: togli la spunta «Sfondo trasparente» oppure usa PNG o SVG | Il JPEG non ha trasparenza: togli «Sfondo trasparente» / JPEG has no transparency: untick «Transparent background» |
| `draw.export.metaTitle` | built at runtime | 7 | keep | Disegno |  |
| `draw.export.svgTitle.clear` | draw.js:701 | 42 | keep | Disegno {app}, {stamp}, sfondo trasparente |  |
| `draw.export.svgTitle.white` | draw.js:701 | 37 | keep | Disegno {app}, {stamp}, sfondo bianco |  |
| `draw.export.transparent.title` | index.html:83 | 142 | shorten | PNG e SVG con sfondo trasparente (il JPEG non lo supporta: con questa spunta il pulsante JPEG è disattivato). Senza spunta lo sfondo è bianco. | PNG e SVG senza sfondo; il JPEG non lo supporta / PNG and SVG without background; JPEG does not support it |
| `draw.font.title` | disegno-font.js:28 | 56 | keep | Cambia la dimensione dei simboli degli atomi nel disegno |  |
| `draw.frag.all.title` | built at runtime | 59 | keep | Tabella dei due frammenti di ogni legame fuori dagli anelli |  |
| `draw.frag.title` | built at runtime | 18 | keep | Rotture dei legami |  |
| `draw.labels.arrows.title` | index.html:73 | 157 | shorten | Spenta di base: le frecce tra due molecole non scrivono niente e il calcolo lo fa lo studente. Accesa, sopra ogni freccia compaiono la perdita neutra e il Δm | Scrive perdita neutra e Δm sopra ogni freccia / Writes neutral loss and Δm above each arrow |
| `draw.multi.add.title` | built at runtime | 112 | shorten | Multicentro: il sostituente selezionato sta su uno qualsiasi degli atomi d'anello selezionati (posizione ignota) | Il sostituente scelto sta su uno qualsiasi degli atomi d'anello scelti / The chosen substituent sits on any of the chosen ring atoms |
| `draw.part.add.title` | built at runtime | 101 | shorten | Struttura parziale: gli atomi selezionati tra parentesi con una modifica di posizione ignota (es. +O) | Atomi scelti tra parentesi con una modifica in posizione ignota (es. +O) / Chosen atoms in brackets with a change at an unknown position (e.g. +O) |
| `draw.props.da.title` | draw.js:448 | 32 | keep | donatori / accettori di legame H |  |
| `draw.props.neutral.title` | draw.js:449 | 140 | shorten | Con la spunta le proprietà sono calcolate sulla forma neutra (COO- diventa COOH, NH3+ diventa NH2); controioni come Cl- o Na+ sono scartati. | Calcola sulla forma neutra (COO- → COOH) e scarta i controioni / Computes on the neutral form (COO- → COOH) and drops counter-ions |
| `draw.side.title` | index.html:68 | 63 | shorten | Nascondi o mostra i riquadri a destra: più spazio per disegnare | Più spazio per disegnare / More room to draw |
| `etpk.tip.int` | built at runtime | 20 | keep | intensità <b>{y}</b> |  |
| `export.meta.title` | built at runtime | 6 | keep | Titolo |  |
| `fair.mb.title` | built at runtime | 15 | keep | Record MassBank |  |
| `fair.si.title` | built at runtime | 17 | keep | File per SIRIUS 6 |  |
| `feat.title` | built at runtime | 21 | keep | Tabella delle feature |  |
| `files.addPair.title` | explore.js:794 | 127 | shorten | Mostra (o chiude) il cromatogramma di questo precursore MS2: lo spettro degli ioni prodotto si apre con il doppio clic sul nome | Cromatogramma di questo precursore; doppio clic sul nome per lo spettro / Chromatogram of this precursor; double click the name for the spectrum |
| `files.precGo.title` | explore.js:794 | 79 | shorten | Clic: va a questo precursore. Doppio clic: apre lo spettro degli ioni prodotto. | Clic: va al precursore; doppio clic: spettro degli ioni prodotto / Click: go to the precursor; double click: product ion spectrum |
| `files.row.remove` | explore.js:792 | 12 | keep | Togli {name} |  |
| `files.showHide` | explore.js:790 | 17 | keep | Mostra o nascondi |  |
| `files.showHideGroup` | explore.js:811, explore.js:812 | 33 | keep | Mostra o nascondi tutto il gruppo |  |
| `files.showHidePrec` | explore.js:791 | 28 | keep | Mostra o nascondi precursori |  |
| `files.type.blk.title` | explore.js:771 | 6 | keep | Bianco |  |
| `files.type.std.title` | explore.js:771 | 8 | keep | Standard |  |
| `fondo.chip.auto.title` | grafici.js:223 | 137 | shorten | Linea di base automatica (SNIP): minimo e massimo del fondo tolto alla traccia, in cps. Lo zero del grafico è il livello medio del fondo. | Linea di base SNIP: fondo minimo e massimo tolto, in cps / SNIP baseline: minimum and maximum background removed, in cps |
| `fondo.flat.left` | fondo-piatto.js:65 | 62 | keep | Sposta la finestra del fondo a sinistra (Maiusc: cinque passi) |  |
| `fondo.flat.right` | fondo-piatto.js:66 | 60 | keep | Sposta la finestra del fondo a destra (Maiusc: cinque passi) |  |
| `fondo.zone.detect.title` | grafici.js:225 | 37 | keep | Cerca la fine del fronte del solvente |  |
| `fondo.zone.title` | grafici.js:225 | 136 | shorten | Parte iniziale della corsa (fronte del solvente), dalla prima scansione: esclusa dalla scala verticale e dalla linea di base automatica. | Fronte del solvente: escluso dalla scala e dalla linea di base / Solvent front: left out of the scale and of the baseline |
| `grad.delRow` | gradiente.js:49 | 13 | keep | Togli la riga |  |
| `grad.title` | built at runtime | 9 | keep | Gradiente |  |
| `groups.has.title` | built at runtime | 30 | keep | Raggruppa per parole contenute |  |
| `groups.move.menu` | gruppi-tab.js:13 | 19 | keep | Sposta nel gruppo ▸ |  |
| `groups.pos.title` | built at runtime | 35 | keep | Raggruppa per posizione del termine |  |
| `groups.tab.grip` | gruppi-tab.js:12 | 29 | keep | Trascina il file su un gruppo |  |
| `header.about` | index.html:18 | 21 | keep | Informazioni su {app} |  |
| `header.fullscreen` | index.html:16 | 14 | keep | Schermo intero |  |
| `header.newRun.title` | index.html:15 | 25 | keep | Inizia una nuova sessione |  |
| `help.close` | help.js:42 | 6 | keep | Chiudi |  |
| `help.compressione.title` | built at runtime | 22 | keep | Compressione dell'mzML |  |
| `help.filegrandi.title` | built at runtime | 38 | keep | File ad alta risoluzione e file grandi |  |
| `help.files.title` | built at runtime | 4 | keep | File |  |
| `help.language.title` | built at runtime | 23 | keep | Lingua dell'interfaccia |  |
| `help.liste.title` | built at runtime | 35 | keep | Liste di riferimento e contaminanti |  |
| `help.losses.title` | built at runtime | 19 | keep | Perdite neutre (Δm) |  |
| `help.mappa.title` | built at runtime | 23 | n/a (visible text) | Mappa: mouse e tastiera |  |
| `help.nav.title` | built at runtime | 15 | keep | Scorrere i file |  |
| `help.origine.title` | built at runtime | 26 | keep | Da dove viene questo ione? |  |
| `help.pnlChrom.title` | built at runtime | 13 | keep | Cromatogramma |  |
| `help.pnlMap.title` | built at runtime | 12 | keep | Mappa RT-m/z |  |
| `help.pnlMrm.title` | built at runtime | 15 | keep | Transizioni MRM |  |
| `help.pnlSpec.title` | built at runtime | 16 | keep | Spettro di massa |  |
| `help.pnlXic.title` | built at runtime | 19 | keep | Ione estratto (XIC) |  |
| `help.prop.title` | built at runtime | 18 | keep | Proprietà previste |  |
| `help.scorrimento.title` | built at runtime | 42 | keep | Scorciatoie e scorrimento fra le scansioni |  |
| `help.sn.title` | built at runtime | 29 | keep | Rapporto segnale/rumore (S/N) |  |
| `help.start.title` | built at runtime | 15 | keep | Caricare i dati |  |
| `help.toolbar.title` | built at runtime | 19 | keep | Aggiungere pannelli |  |
| `help.tools.title` | built at runtime | 17 | keep | Metodo e immagini |  |
| `ibk.chip.title` | grafici.js:223 | 186 | shorten | Fondo di un tratto: la media del tratto scelto, tolta a tutta la traccia. Vale se il fondo è piatto e il tratto non contiene l'analita; se il fondo sale, usa la linea di base automatica. | Media del tratto tolta a tutta la traccia: vale solo con fondo piatto / Mean of the stretch removed from the whole trace: only for a flat background |
| `ibk.leg.title` | built at runtime | 45 | keep | Clic per scegliere un altro tratto del bianco |  |
| `ibk.spec.title` | grafici.js:226 | 82 | shorten | Sottrae lo spettro medio dello stesso tratto dagli spettri di questo cromatogramma | Toglie lo spettro medio del tratto dagli spettri di questo cromatogramma / Subtracts the mean spectrum of the stretch from this chromatogram's spectra |
| `inf.avg.tip` | built at runtime | 54 | keep | Spettro di un nodo: media o somma delle sue scansioni. |  |
| `inf.ce.tip` | built at runtime | 66 | keep | Separate: un nodo per energia di collisione. Unite: energie unite. |  |
| `inf.col.tic.tip` | infusione-albero.js:93 | 70 | keep | Corrente ionica totale media per scansione del nodo; clic per ordinare |  |
| `inf.copy.tip` | infusione-albero.js:114 | 40 | keep | Copia la tabella (testo con tabulazioni) |  |
| `inf.frag.tip` | infusione-menu.js:20 | 52 | keep | Va al nodo figlio che ha questo ione come precursore |  |
| `inf.lv.tip` | built at runtime | 40 | keep | Livello MS massimo mostrato nell'albero. |  |
| `inf.sum.tip` | infusione-menu.js:33 | 38 | keep | Media o somma delle scansioni del nodo |  |
| `inf.tol.tip` | built at runtime | 64 | keep | Tolleranza entro cui due m/z del precursore sono lo stesso nodo. |  |
| `inf.up.tip` | infusione-menu.js:27 | 19 | keep | Va al nodo genitore |  |
| `int.tbl.del.title` | picchi.js:247 | 47 | keep | Elimina questa integrazione (anche dal grafico) |  |
| `int.tbl.overlap.title` | picchi.js:247 | 98 | shorten | Quest'integrazione si sovrappone a un'altra della stessa traccia (taccuino vecchio): eliminane una | Si sovrappone a un'altra integrazione della traccia: eliminane una / Overlaps another integration of the trace: delete one |
| `int.tbl.par.title` | picchi.js:241 | 130 | shorten | Larghezza a metà altezza, numero di piatti teorici N = 5.54 (tR / w1/2)^2 e fattore di coda USP a 5% dell'altezza (T = W0.05 / 2f) | Larghezza a metà altezza, piatti teorici N e fattore di coda USP / Width at half height, theoretical plates N and USP tailing factor |
| `int.tbl.pickNoise.title` | picchi.js:245 | 203 | shorten | Trascina sul grafico una zona senza picchi vicino al picco: il rumore è la deviazione standard dei residui del segnale intorno a una retta adattata in quella zona. Vale per tutti i file di questa traccia | Trascina su una zona senza picchi: vale per tutti i file della traccia / Drag over a zone without peaks: it applies to every file of the trace |
| `int.tbl.q.title` | picchi.js:242 | 173 | shorten | Misure per discutere la qualità di un'integrazione, non un giudizio: somiglianza a una gaussiana, asimmetria al 10 % dell'altezza e punteggio di rumore dei punti tra i bordi | Somiglianza a una gaussiana, asimmetria e rumore: misure, non giudizi / Gaussian similarity, asymmetry and noise: measures, not verdicts |
| `int.tbl.sn.title` | picchi.js:240 | 191 | shorten | Altezza del picco (la stessa colonna «Altezza», sopra la linea di base) divisa per la deviazione standard del rumore. Segni la zona di rumore una volta sola per traccia: vale per tutti i file | Altezza del picco divisa per la deviazione standard del rumore / Peak height divided by the standard deviation of the noise |
| `int.tbl.title` | built at runtime | 23 | keep | Integrazioni dei picchi |  |
| `kmd.feat.title` | built at runtime | 4 | keep | Ioni |  |
| `kmd.point.tip` | kmd-vista.js:37 | 42 | keep | m/z {mz} · RT {rt} · KMD {kmd} · serie {s} |  |
| `kmd.series.title` | built at runtime | 15 | keep | Serie candidate |  |
| `kmd.title` | built at runtime | 52 | keep | Difetto di massa di Kendrick (PFAS e serie omologhe) |  |
| `leg.fileAll.title` | grafici.js:270 | 113 | shorten | Clic: mostra o nascondi questo file in tutte le tracce. Il tratto distingue i file (il colore distingue gli ioni) | Mostra o nasconde il file in tutte le tracce; il tratteggio distingue i file / Shows or hides the file in all traces; the dash style tells files apart |
| `lib.all.max` | libreria-finestra.js:67 | 23 | keep | Ingrandisci la finestra |  |
| `lib.all.openBest` | libreria-finestra.js:415 | 45 | keep | Apre i candidati della sua scansione migliore |  |
| `lib.all.restore` | libreria-finestra.js:67 | 39 | keep | Ripristina la dimensione della finestra |  |
| `lib.all.title` | built at runtime | 32 | keep | Identifica tutte le MS2 del file |  |
| `lib.avail.ext.hint` | built at runtime | 65 | n/a (visible text) | Si scaricano dai siti ufficiali; dopo il download si indicizzano. |  |
| `lib.copy.title` | libreria-finestra.js:316, libreria.js:336 | 60 | shorten | Copia la tabella: incollata in Excel riempie righe e colonne | Incollata in Excel riempie righe e colonne / Pasted in Excel it fills rows and columns |
| `lib.droppedTitle` | built at runtime | 71 | keep | senza precursore {prec} · senza picchi {peaks} · blocchi rotti {broken} |  |
| `lib.pairs.title` | built at runtime | 77 | keep | {n, plural, one {Picco corrispondente (1)} other {Picchi corrispondenti (#)}} |  |
| `lib.search.title` | built at runtime | 20 | keep | Cerca nelle librerie |  |
| `lib.title` | built at runtime | 19 | keep | Librerie di spettri |  |
| `lib.what.group` | libreria-finestra.js:184 | 120 | n/a (visible text) | {n, plural, one {+# voce con lo stesso primo blocco di InChIKey} other {+# voci con lo stesso primo blocco di InChIKey}} |  |
| `lib.what.title` | built at runtime | 6 | keep | Cos'è? |  |
| `llm.consenso.title` | built at runtime | 23 | keep | Accendere l'assistente? |  |
| `llm.hideNames.title` | built at runtime | 69 | keep | Nei dati inviati i nomi dei file sono sostituiti da etichette neutre. |  |
| `llm.key.hint` | built at runtime | 158 | n/a (visible text) | Resta in memoria per questa sessione; se la ricordi, è salvata cifrata in questo browser. Protegge da una copia del disco, non da uno script di questa pagina. |  |
| `llm.level.L0.title` | built at runtime | 44 | keep | L'assistente propone soltanto; fai tutto tu. |  |
| `llm.level.L1.title` | built at runtime | 66 | keep | L'assistente chiede la tua conferma prima di ogni azione sul Flow. |  |
| `llm.level.L2.title` | built at runtime | 66 | keep | L'assistente agisce da solo, mostra ogni azione e puoi annullarla. |  |
| `llm.off.hint` | built at runtime | 75 | n/a (visible text) | L'assistente è spento: non si scarica nulla e nulla lascia questo computer. |  |
| `llm.on.title` | built at runtime | 133 | shorten | Un modello linguistico che può consigliare i passi del Flow. Nulla viene scaricato o inviato finché non lo accendi e dai il consenso. | Modello linguistico che consiglia i passi del Flow; spento non invia nulla / A language model that advises Flow steps; when off, nothing is sent |
| `load.cancel.title` | upload-rows.js:27 | 49 | keep | Ferma il caricamento: i file già caricati restano |  |
| `load.cols.bad` | colonne-campioni.js:136 | 19 | keep | Inserisci un numero |  |
| `load.demo.full.title` | index.html:53 | 59 | keep | 5 file Full Scan: 0, 5, 10, 15 e 30 minuti di irraggiamento |  |
| `load.demo.hrms.title` | index.html:53 | 114 | shorten | 4 file ad alta risoluzione (Orbitrap, DDA): serie temporale di un inquinante degradato su TiO2 (0, 5, 10 e 30 min) | 4 file Orbitrap DDA: inquinante degradato su TiO2 a 0, 5, 10 e 30 min / 4 Orbitrap DDA files: a pollutant degraded on TiO2 at 0, 5, 10 and 30 min |
| `load.demo.mrm.title` | index.html:53 | 66 | keep | 4 standard MRM a concentrazione crescente (0.6; 2.4; 7.2 e 18 ppm) |  |
| `load.demo.ms2.title` | index.html:53 | 42 | keep | 2 file MS2 (ioni prodotto): 15 e 60 minuti |  |
| `load.demo.title` | index.html:53 | 67 | shorten | Scegli quali file di esempio aprire: si scarica solo ciò che scegli | Si scarica solo ciò che scegli / Only what you choose is downloaded |
| `load.guess.title` | ruoli-celle.js:6, shell.js:103 | 34 | keep | Letto dal nome del file: controlla |  |
| `load.ph.index` | explore.js:791 | 14 | keep | Indicizzazione |  |
| `load.table.concStd.title` | shell.js:109 | 29 | keep | Concentrazione dello standard |  |
| `load.table.num.bad` | shell.js:128 | 39 | keep | Scrivi un numero (anche con la virgola) |  |
| `load.table.remove` | shell.js:90, shell.js:110 | 24 | keep | Rimuovi (va in _cestino) |  |
| `load.table.tunit.title` | shell.js:101 | 63 | keep | Unità del tempo mostrata (i tempi sono letti dai nomi dei file) |  |
| `load.table.unit.title` | shell.js:101 | 52 | keep | Unità delle concentrazioni (1 ppm = 1 mg/L in acqua) |  |
| `load.time.dark` | ruoli-celle.js:13 | 62 | keep | Minuti di buio prima del tempo zero: valore negativo (es. −30) |  |
| `lock.off.title` | pannelli.js:267, scroll.js:101 | 83 | shorten | Blocca la scala y: così vedi crescere e calare i picchi fra una scansione e l'altra | Blocca la scala y per vedere i picchi crescere e calare / Lock the y scale to see peaks grow and shrink |
| `lock.on.title` | scroll.js:101 | 53 | keep | Sblocca la scala y: l'asse delle intensità è bloccato |  |
| `loss.radShort` | tables.js:68 | 32 | keep | perdita di radicale, rara in ESI |  |
| `lst.contSigns.title` | liste.js:584 | 200 | shorten | Sugli spettri ad alta risoluzione, un rombo segna i picchi la cui m/z è compatibile con un contaminante noto e il riquadro al mouse dice quale. Spegnendo l'interruttore spariscono segni ed esclusioni. | Un rombo segna i picchi compatibili con un contaminante noto (alta risoluzione) / A diamond marks peaks matching a known contaminant (high resolution) |
| `lst.lab.removeRow` | liste.js:465 | 18 | keep | Toglie questa riga |  |
| `lst.member` | liste.js:775 | 68 | n/a (visible text) | Numero di unità ripetitive (monomeri) nella catena del polimero: {n} |  |
| `lst.polFilter` | liste.js:575 | 15 | keep | Filtro polarità |  |
| `lst.tgt.row.tip` | liste-target.js:145 | 43 | keep | Clic per vedere XIC e spettro a RT {rt} min |  |
| `lst.tgt.upload.tip` | liste-target.js:75 | 41 | keep | Carica un elenco di target da un file CSV |  |
| `lst.title` | built at runtime | 35 | keep | Liste di riferimento e contaminanti |  |
| `lst.userDelete` | liste.js:714 | 20 | keep | Elimina questa lista |  |
| `lv.thr.title` | built at runtime | 18 | keep | Soglie dei livelli |  |
| `lw.opt.title` | built at runtime | 41 | keep | Tolleranze, soglie dei livelli e librerie |  |
| `lw.sort` | libreria-finestra.js:122 | 16 | keep | Ordina per {col} |  |
| `m3d.err.title` | built at runtime | 25 | keep | Visore 3D non disponibile |  |
| `m3d.hist.title` | mappa3d.js:181 | 72 | keep | Istogramma dei valori (scala log): clic per fissare la soglia del rumore |  |
| `m3d.play` | mappa3d.js:161 | 16 | keep | Scorri nel tempo |  |
| `m3d.side.toggle` | mappa3d.js:162 | 35 | keep | Mostra o nasconde la barra laterale |  |
| `m3d.sync.title` | mappa3d.js:170 | 60 | keep | Riprende l'intervallo RT e m/z che la mappa 2D mostra adesso |  |
| `m3d.title` | built at runtime | 20 | keep | Mappa RT × m/z in 3D |  |
| `m3d.v.mz.title` | mappa3d.js:184 | 49 | keep | Visto di lato lungo l'RT: il profilo è lo spettro |  |
| `m3d.v.rt.title` | mappa3d.js:184 | 54 | keep | Visto di lato lungo m/z: il profilo è il cromatogramma |  |
| `m3d.v.top.title` | mappa3d.js:184 | 30 | keep | Vista dall'alto: è la mappa 2D |  |
| `map.leg2d.down.tip` | grafici.js:607 | 32 | keep | Blu: più intenso nel riferimento |  |
| `map.leg2d.up.tip` | grafici.js:607 | 33 | keep | Arancio: più intenso nel campione |  |
| `mapex.spec.title` | built at runtime | 37 | keep | Spettro del riquadro (finestra di RT) |  |
| `mapex.xic.title` | built at runtime | 52 | keep | XIC del riquadro (finestra m/z, in funzione dell'RT) |  |
| `mapfeat.title` | built at runtime | 35 | keep | Come è stata trovata · feature {id} |  |
| `mappa.bot.title` | built at runtime | 28 | keep | XIC dell'm/z sotto il mirino |  |
| `mappa.btn.mark.title` | mappa.js:973 | 185 | shorten | Segna (tasto S): ogni clic aggiunge un punto numerato sul massimo locale vicino (±0.2 min, ±1 bin); Canc toglie il punto scelto; i punti si trascinano. Il programma misura, non giudica. | Segna (S): un punto numerato sul massimo vicino; Canc lo toglie / Mark (S): a numbered point on the nearby maximum; Delete removes it |
| `mappa.btn.mir.title` | mappa.js:973 | 116 | shorten | Mirino (tasto M): croce, lettura e, accanto alla mappa, lo spettro della scansione e l'XIC dell'm/z sotto il cursore | Mirino (M): spettro e XIC sotto il cursore accanto alla mappa / Sight (M): spectrum and XIC under the cursor next to the map |
| `mappa.btn.snap.title` | mappa.js:973 | 177 | shorten | Aggancio al massimo: un punto segnato scivola sul punto più intenso del picco vicino (±0.2 min, ±1 bin), così nel 3D si vede la cima. Spegnilo per mettere il punto dove clicchi. | Il punto segnato scivola sulla cima del picco vicino / A marked point slides onto the top of the nearby peak |
| `mappa.col.d.title` | built at runtime | 26 | keep | Differenza delle intensità |  |
| `mappa.col.dm.title` | built at runtime | 151 | shorten | Differenza di m/z dal punto più intenso del gruppo, o dal punto di riferimento scelto (colonna Rif.). Clic: apre Perdite neutre (o Addotti) nella barra | Δm/z dal punto più intenso del gruppo o dal riferimento; clic: perdite neutre / Δm/z from the group's top point or the reference; click: neutral losses |
| `mappa.col.g.title` | built at runtime | 59 | keep | Punti con gli apici vicini nel tempo (tolleranza qui sopra) |  |
| `mappa.col.ia.title` | built at runtime | 43 | keep | Intensità nel file mostrato (XIC all'apice) |  |
| `mappa.col.ib.title` | built at runtime | 59 | keep | Intensità nel riferimento (massimo dell'XIC entro ±0.1 min) |  |
| `mappa.col.mz.title` | built at runtime | 76 | keep | m/z del picco all'apice, come lo scrive il file (in alta risoluzione esatta) |  |
| `mappa.col.n.title` | built at runtime | 28 | keep | Numero del punto sulla mappa |  |
| `mappa.col.note.title` | built at runtime | 12 | keep | Una nota tua |  |
| `mappa.col.ratio.title` | built at runtime | 24 | keep | Rapporto delle intensità |  |
| `mappa.col.rt.title` | built at runtime | 55 | keep | RT dell'apice (la scansione più intensa vicino al clic) |  |
| `mappa.col.sn.title` | built at runtime | 60 | keep | Segnale/rumore in A: (apice − mediana dell'XIC) / rumore MAD |  |
| `mappa.dens.title` | mappa.js:970 | 112 | shorten | Etichette m/z dello spettro laterale: poche, medie o molte (scelte per S/N locale, almeno una per fascia di m/z) | Quante etichette m/z nello spettro laterale / How many m/z labels in the side spectrum |
| `mappa.fix.title` | mappa.js:971 | 132 | shorten | Stessa scala dei colori su tutti i file aperti: il massimo è il più alto fra i file, così i campioni si confrontano a colpo d'occhio | Stessa scala dei colori per tutti i file, per confrontarli / Same colour scale for all files, to compare them |
| `mappa.grp.fold` | mappa-tabella.js:139 | 22 | keep | Comprimi il gruppo {g} |  |
| `mappa.grp.unfold` | mappa-tabella.js:139 | 21 | keep | Espandi il gruppo {g} |  |
| `mappa.lbl.tip` | mappa.js:53 | 60 | keep | m/z {mz} · S/N {snr} (intensità sul rumore attorno al picco) |  |
| `mappa.pal.title` | mappa.js:967 | 98 | shorten | Palette dei colori: viridis (predefinita) o cividis, leggibili anche da chi non distingue i colori | Viridis o cividis: leggibili anche con il daltonismo / Viridis or cividis: both readable with colour-vision deficiency |
| `mappa.rend.title` | mappa.js:969 | 125 | shorten | Resa della mappa quando si ingrandisce: Liscia (le macchie restano continue) o Dettaglio (una cella per ogni valore dei dati) | Liscia (macchie continue) o Dettaglio (una cella per valore) / Smooth (continuous blobs) or Detail (one cell per value) |
| `mappa.rz.title` | mappa.js:1032 | 66 | keep | Trascina per cambiare l'altezza; doppio clic = altezza predefinita |  |
| `mappa.show.title` | mappa.js:971 | 161 | shorten | La mappa mostra gli aumenti (A più alto di B, arancio) e le diminuzioni (blu); togliendo la spunta restano solo gli aumenti. Il 3D mostra sempre entrambi i segni | Spenta, la mappa mostra solo gli aumenti (arancio) / Off, the map shows only the increases (orange) |
| `mappa.side.title` | built at runtime | 23 | keep | spettro della scansione |  |
| `mappa.tab.align.title` | mappa-tabella.js:104 | 93 | shorten | Porta tutti i punti di un gruppo sul RT del primo punto segnato, sulla stessa linea verticale | Porta i punti di un gruppo sul RT del primo punto / Moves a group's points to the RT of its first point |
| `mappa.tab.cp.title` | mappa-tabella.js:105 | 57 | keep | Copia la tabella con le tabulazioni (si incolla in Excel) |  |
| `mappa.tab.csv.title` | mappa-tabella.js:105 | 91 | shorten | Scarica la stessa tabella in CSV (UTF-8, separata da virgole, senza il foglio dei metadati) | CSV UTF-8 separato da virgole, senza i metadati / UTF-8 comma-separated CSV, without the metadata |
| `mappa.tab.grp.title` | mappa-tabella.js:104 | 125 | shorten | Due punti stanno nello stesso gruppo se i loro apici distano al massimo questo tempo (di base circa la larghezza di un picco) | Distanza massima tra gli apici di due punti dello stesso gruppo / Maximum distance between the apexes of two points in one group |
| `mappa.tab.ref.pick` | mappa-tabella.js:103 | 101 | shorten | Usa il punto {n} come riferimento per Δm (premi di nuovo per tornare al punto più intenso del gruppo) | Usa il punto {n} come riferimento per Δm (di nuovo: torna al più intenso) / Use point {n} as the reference for Δm (again: back to the most intense) |
| `mappa.tab.ref.title` | mappa-tabella.js:99 | 102 | shorten | Punto di riferimento per Δm: se non ne scegli uno, Δm è calcolato dal punto più intenso di ogni gruppo | Senza scelta, Δm parte dal punto più intenso di ogni gruppo / With no choice, Δm starts from the most intense point of each group |
| `mappa.tab.title` | built at runtime | 18 | n/a (visible text) | Punti segnati: {n} |  |
| `mappa.tab.xic.title` | mappa-tabella.js:103 | 55 | keep | XIC di questo m/z nei file visibili (A e B sovrapposti) |  |
| `mappa.tab.xl.title` | mappa-tabella.js:105 | 79 | keep | Scarica la tabella in Excel, con un foglio di metadati (file, tolleranze, data) |  |
| `mappa.thr.title` | mappa.js:968 | 105 | shorten | Soglia del rumore: i valori più piccoli di questa percentuale (del 99.5° percentile) restano senza colore | Sotto questa % (del 99.5° percentile) i valori restano senza colore / Below this % (of the 99.5th percentile) values stay uncoloured |
| `masscal.close` | masscal.js:63 | 6 | keep | Chiudi |  |
| `masscal.title` | built at runtime | 46 | keep | Controllo della calibrazione di massa · {file} |  |
| `mem.more.title` | built at runtime | 17 | keep | Serve più memoria |  |
| `meta.title` | built at runtime | 21 | keep | Metadati dei campioni |  |
| `method.check.title` | built at runtime | 30 | keep | Il metodo corrisponde ai dati? |  |
| `method.dialog.title` | built at runtime | 22 | keep | Metodo di acquisizione |  |
| `method.doc.btn.ms2.tip` | built at runtime | 70 | keep | Apri la scansione MS² del precursore m/z {mz} più vicina a RT {rt} min |  |
| `method.doc.btn.xic.tip` | metodo-incl.js:30 | 79 | keep | Estrai l'XIC di m/z {mz} (finestra di una unità o in ppm, come negli altri XIC) |  |
| `method.doc.incl.title` | built at runtime | 19 | keep | Lista di inclusione |  |
| `method.doc.title` | built at runtime | 26 | keep | Metodo registrato nel file |  |
| `method.lc.title` | built at runtime | 26 | keep | Metodo cromatografico (LC) |  |
| `method.m.title` | built at runtime | 28 | keep | Metodo Agilent «{name}» (.m) |  |
| `method.pda.title` | built at runtime | 19 | keep | Rivelatore PDA (UV) |  |
| `method.reg.go` | metodo-registro.js:17 | 16 | keep | Vai a RT {t} min |  |
| `method.source.title` | built at runtime | 26 | keep | Sorgente e composto (.dam) |  |
| `methwarn.hide` | explore.js:425 | 17 | keep | Nascondi l'avviso |  |
| `methwarn.title` | built at runtime | 41 | keep | <b>Il metodo non corrisponde ai dati.</b> |  |
| `mol.title` | built at runtime | 8 | keep | Molecole |  |
| `msngrafo.brk.title` | built at runtime | 33 | keep | Rotture di legami nella struttura |  |
| `msngrafo.copy.tip` | infusione-grafo-esporta.js:68 | 31 | keep | Copia gli spettri negli appunti |  |
| `msngrafo.data.title` | built at runtime | 32 | keep | Dati del grafo di frammentazione |  |
| `msngrafo.draw.go.tip` | infusione-grafo-nodo.js:87 | 41 | keep | Apre Disegno con la struttura di partenza |  |
| `msngrafo.draw.save.tip` | infusione-grafo-nodo.js:87 | 59 | keep | Salva accanto a questo ione la struttura che hai in Disegno |  |
| `msngrafo.draw.title` | built at runtime | 19 | keep | Frammento disegnato |  |
| `msngrafo.export.tip` | infusione-grafo-esporta.js:68 | 38 | keep | Scarica gli spettri nel formato scelto |  |
| `msngrafo.file.close` | infusione-grafo-apri.js:51 | 15 | keep | Chiudi il grafo |  |
| `msngrafo.file.off` | infusione-grafo-nodo.js:35 | 101 | shorten | Richiede il file dei dati grezzi, che non è caricato: questo grafo è stato aperto da un file salvato. | Serve il file grezzo, che non è caricato. / Needs the raw data file, which is not loaded. |
| `msngrafo.file.open.tip` | infusione-grafo-apri.js:119 | 137 | shorten | Apre un grafo di frammentazione salvato (un file .json della riga di comando o del Flow); puoi anche trascinarlo qui o nell'area dei dati | Un grafo salvato (.json della riga di comando o del Flow); anche trascinandolo / A saved graph (.json from the command line or the Flow); dropping it works too |
| `msngrafo.file.title` | built at runtime | 32 | keep | Grafo di frammentazione · {name} |  |
| `msngrafo.find.delta.tip` | infusione-grafo-cerca.js:37 | 90 | shorten | 0 = solo voci con lo stesso precursore; oltre, cerca anche analoghi con frammenti spostati | 0 = stesso precursore; di più cerca anche analoghi con frammenti spostati / 0 = same precursor; more also finds analogs with shifted fragments |
| `msngrafo.find.tip` | infusione-grafo-cerca.js:37 | 64 | keep | Confronta questo albero con la tua libreria locale di alberi MSn |  |
| `msngrafo.flow.tip` | infusione-grafo-campo.js:26 | 90 | shorten | Costruisce il grafo con la molecola scelta e confronta subito l'albero con la tua libreria | Costruisce il grafo della molecola e lo confronta con la tua libreria / Builds the molecule's graph and compares it with your library |
| `msngrafo.int.title` | built at runtime | 32 | keep | Perdita neutra verso questo ione |  |
| `msngrafo.ion.tip` | infusione-grafo-campo.js:25 | 100 | shorten | Scrivi la formula dello ione (per esempio con carica o addotto) oppure scegli una molecola qui sopra | Formula dello ione (con carica o addotto) o una molecola qui sopra / Ion formula (with charge or adduct) or a molecule above |
| `msngrafo.lib.tip` | infusione-grafo-esporta.js:69 | 61 | keep | Aggiunge questo albero alla tua libreria locale di alberi MSn |  |
| `msngrafo.mzlow.tip` | built at runtime | 43 | keep | Sotto questo m/z la finestra può allargarsi |  |
| `msngrafo.node.title` | built at runtime | 20 | keep | MS{level} · m/z {mz} |  |
| `msngrafo.ppm.tip` | built at runtime | 49 | keep | Finestra in ppm per le sottoformule dei frammenti |  |
| `msngrafo.ppmlow.tip` | built at runtime | 63 | keep | Finestra più larga per i candidati con prove, sotto l'm/z basso |  |
| `msngrafo.proofs.tip` | built at runtime | 51 | keep | Prove richieste a un candidato nella finestra larga |  |
| `msngrafo.rule.filter.tip` | infusione-grafo.js:60 | 63 | keep | Mostra solo le perdite spiegate da una regola di frammentazione |  |
| `msngrafo.rule.thr.title` | built at runtime | 19 | keep | Soglie delle regole |  |
| `msngrafo.rule.title` | built at runtime | 24 | keep | Regole di frammentazione |  |
| `msngrafo.str.show.tip` | infusione-grafo-nodo.js:97 | 53 | keep | Mostra nel disegno il legame {bond}, taglio candidato |  |
| `msngrafo.str.title` | built at runtime | 31 | keep | Tagli candidati della struttura |  |
| `msngrafo.use.tip` | infusione-grafo-nodo.js:35 | 59 | keep | Usa {f} per questo ione (candidata, non un'identificazione) |  |
| `msngrafo.v.best.tip` | infusione-grafo.js:73 | 43 | keep | Disegna solo le frecce del cammino migliore |  |
| `msngrafo.v.close.tip` | infusione-grafo.js:74 | 75 | keep | Nasconde gli ioni che dipendono dallo ione scelto lungo il cammino migliore |  |
| `msngrafo.v.hypo.title` | built at runtime | 17 | keep | Ipotesi di taglio |  |
| `msngrafo.v.level.tip` | infusione-grafo.js:72 | 51 | keep | Mostra solo gli ioni visti fino a questo livello MS |  |
| `msngrafo.v.minrel.tip` | infusione-grafo.js:71 | 81 | shorten | Nasconde gli ioni sotto questa % del picco base nello spettro del loro precursore | Nasconde gli ioni sotto questa % del picco base del loro precursore / Hides ions below this % of their precursor's base peak |
| `msngrafo.v.structs.tip` | infusione-grafo.js:75 | 114 | shorten | Disegna la molecola e, per gli ioni con tagli candidati, il genitore con gli atomi del pezzo evidenziati (ipotesi) | Disegna la molecola e i tagli candidati evidenziati (ipotesi) / Draws the molecule with the candidate cuts highlighted (hypothesis) |
| `msngrafo.v.view.radial.tip` | infusione-grafo.js:70 | 70 | keep | Il precursore al centro, un anello per ogni passo del cammino migliore |  |
| `msngrafo.v.view.tree.tip` | infusione-grafo.js:70 | 22 | keep | Colonne per livello MS |  |
| `msnl.hint` | built at runtime | 204 | n/a (visible text) | Un clic su una scansione (o Invio) apre il suo spettro; una riga di precursore apre e chiude le sue scansioni. TIC: le scansioni MS2 del precursore contro il RT; XIC: l'm/z del precursore nella Full Scan. |  |
| `normative.lib.title` | built at runtime | 15 | keep | Liste normative |  |
| `og.s1.title` | built at runtime | 37 | n/a (visible text) | 1. Profili cromatografici confrontati |  |
| `og.s2.title` | built at runtime | 51 | n/a (visible text) | 2. Ioni che escono insieme al candidato progenitore |  |
| `og.s3.title` | built at runtime | 57 | n/a (visible text) | 3. Area dello ione contro area del candidato (F contro P) |  |
| `og.s4.title` | built at runtime | 40 | n/a (visible text) | 4. Comportamento nei campioni (cinetica) |  |
| `og.s5.title` | built at runtime | 17 | n/a (visible text) | 5. La tua ipotesi |  |
| `og.title` | built at runtime | 26 | n/a (visible text) | Da dove viene questo ione? |  |
| `open.title` | built at runtime | 19 | keep | Apertura di {file}… |  |
| `pal.hint` | built at runtime | 40 | keep | ↑ ↓ scorrono · Invio esegue · Esc chiude |  |
| `pal.title` | built at runtime | 5 | keep | Cerca |  |
| `panel.close` | pannelli.js:274 | 18 | keep | Chiudi il pannello |  |
| `panel.dl.title` | pannelli.js:272 | 39 | keep | Scarica l'immagine o i dati del grafico |  |
| `panel.fit.title` | pannelli.js:264 | 134 | shorten | Torna a vedere tutto il grafico (attivo solo quando è ingrandito). Da tastiera: Backspace = vista intera, Ctrl/Cmd+Z = zoom precedente | Torna alla vista intera (Backspace; Ctrl/Cmd+Z: zoom precedente) / Back to the whole view (Backspace; Ctrl/Cmd+Z: previous zoom) |
| `panel.iauto.title` | pannelli.js:269 | 205 | shorten | Integrazione automatica: clicca su un picco e il programma trova i bordi e ne mostra l'area (poi puoi trascinare le barre); su un file ad alta risoluzione un clic integra tutti i picchi del tratto mostrato | Clic su un picco: bordi e area in automatico / Click a peak: edges and area found automatically |
| `panel.iclr.title` | pannelli.js:269 | 48 | keep | Cancella tutte le integrazioni di questo grafico |  |
| `panel.idel.title` | pannelli.js:269 | 81 | shorten | Cancella integrazione: clicca su un picco integrato per eliminarne l'integrazione | Clic su un picco integrato per eliminarlo / Click an integrated peak to delete it |
| `panel.iman.title` | pannelli.js:269 | 68 | shorten | Integrazione manuale: trascina sul grafico l'intervallo da integrare | Trascina sul grafico l'intervallo da integrare / Drag on the chart the interval to integrate |
| `panel.intf.title` | pannelli.js:269 | 88 | shorten | Quale traccia integrare: quella su cui clicchi, tutte le visibili oppure un file preciso | La traccia cliccata, tutte le visibili o un file preciso / The trace you click, all visible ones or one file |
| `panel.itab.title` | pannelli.js:269 | 39 | keep | Tabella delle aree integrate e cinetica |  |
| `panel.izoom.title` | pannelli.js:265 | 174 | shorten | Zoom: attivalo e trascina sul grafico l'intervallo di tempo da ingrandire; il pulsante accanto torna alla vista intera. Ctrl o Cmd + rotella ingrandisce anche senza attivarlo | Trascina un intervallo di tempo per ingrandirlo (anche Ctrl/Cmd + rotella) / Drag a time interval to enlarge it (also Ctrl/Cmd + wheel) |
| `panel.max.exit` | pannelli.js:311 | 31 | keep | Esci dallo schermo intero (Esc) |  |
| `panel.max.title` | pannelli.js:274, pannelli.js:311 | 60 | keep | Schermo intero: ingrandisce questo pannello (Esc per uscire) |  |
| `panel.more.title` | pannelli.js:273 | 27 | keep | Altri strumenti del grafico |  |
| `panel.num.title` | pannelli.js:49 | 46 | keep | Pannello {n}: premi il tasto {n} per attivarlo |  |
| `panel.pairArrow.title` | pannelli.js:88 | 87 | shorten | Lo spettro sotto mostra gli ioni prodotto del precursore scelto nel cromatogramma sopra | Lo spettro sotto è del precursore scelto sopra / The spectrum below belongs to the precursor chosen above |
| `panel.par.title` | pannelli.js:267 | 75 | shorten | Parametri dello spettro: asse in % o in cps, quante etichette m/z, decimali | Asse in % o cps, etichette m/z e decimali / Axis in % or cps, m/z labels and decimals |
| `panel.rel.title` | pannelli.js:267 | 62 | shorten | Asse dell'intensità: in cps (acceso = in % del picco più alto) | Acceso: intensità in % del picco più alto / On: intensity as % of the highest peak |
| `panel.rename.hint` | pannelli.js:261 | 27 | keep | Doppio clic per rinominare. |  |
| `panel.rul.title` | pannelli.js:267 | 182 | shorten | Righello: clic su un picco (riferimento) e poi su un altro: mostra la differenza di m/z. Si può anche usare il clic destro su un picco, «Misura da questo picco». Esc toglie le misure | Clic su un picco e poi su un altro: differenza di m/z (Esc toglie) / Click a peak, then another: m/z difference (Esc clears) |
| `panel.title.xicOne` | pannelli.js:651 | 13 | keep | XIC · {label} |  |
| `panel.tlink.title` | pannelli.js:268 | 124 | shorten | Collega zoom e cursore agli altri cromatogrammi: ingrandendo uno, gli altri collegati mostrano lo stesso intervallo di tempo | Zoom e cursore uguali negli altri cromatogrammi collegati / Same zoom and cursor in the other linked chromatograms |
| `panel.wl.title` | eventi.js:258 | 71 | shorten | Estrai la traccia a una lunghezza d'onda (λ): scegli l'intervallo di nm | Traccia a una lunghezza d'onda (λ), in nm / Trace at one wavelength (λ), in nm |
| `panel.xic.off` | pannelli.js:271 | 23 | keep | Non disponibile per MS2 |  |
| `panel.xic.title` | pannelli.js:271 | 75 | shorten | Estrai uno ione (XIC): scegli la finestra di m/z. Si integra solo dagli XIC | Finestra di m/z di uno ione; si integra solo dagli XIC / m/z window of one ion; integration works only on XICs |
| `pca.ell` | pca.js:79 | 36 | keep | Ellisse al 95 % delle repliche «{g}» |  |
| `pca.help` | pca.js:128 | 12 | keep | Effetto arco |  |
| `pca.sample.tip` | pca.js:87 | 9 | keep | {f} · {t} |  |
| `pca.title` | built at runtime | 15 | keep | PCA esplorativa |  |
| `perf.clear.title` | perf.js:91 | 15 | keep | Azzera le righe |  |
| `perf.copy.title` | perf.js:91 | 22 | keep | Copia tutto come testo |  |
| `perf.min.title` | perf.js:91 | 16 | keep | Riduci o allarga |  |
| `perf.title` | built at runtime | 14 | keep | Misure (?perf) |  |
| `pfas.ev.title` | built at runtime | 29 | keep | Frammenti e perdite nelle MS2 |  |
| `pfas.formulas.title` | built at runtime | 17 | keep | Formule candidate |  |
| `pfas.point.tip` | pfas.js:102 | 43 | keep | m/z {mz} · {f} · MD/C {mdc} · prove MS2 {n} |  |
| `pfas.title` | built at runtime | 34 | n/a (visible text) | PFAS: MD/C e frammenti diagnostici |  |
| `phone.about.title` | built at runtime | 15 | keep | Che cos'è {app} |  |
| `phone.title` | built at runtime | 23 | keep | Siete su uno smartphone |  |
| `pk.title` | built at runtime | 22 | keep | Rilevamento dei picchi |  |
| `predef.guide.title` | built at runtime | 9 | keep | Come fare |  |
| `predef.settings.title` | predefinito.js:60 | 82 | follow-up (SETTINGS-PANEL) | Torna a proporre di rendere mzLab l'app predefinita per i file .mzML e .raw Thermo |  |
| `predef.title` | built at runtime | 32 | keep | Rendere mzLab l'app predefinita? |  |
| `prof.chart.aria` | profilo.js:76 | 26 | keep | Profilo nel tempo di {ion} |  |
| `prof.istd.hint` | built at runtime | 66 | keep | Per normalizzare aggiungi uno standard interno nel pannello QA/QC. |  |
| `prof.title` | built at runtime | 29 | keep | Profilo nel tempo di uno ione |  |
| `qaqc.remove` | qaqc.js:163 | 9 | keep | Togli {n} |  |
| `ref.title` | built at runtime | 26 | keep | Differenza dal riferimento |  |
| `reg.pick` | registro-serie.js:78 | 19 | keep | Canale del registro |  |
| `rep.title` | rapporto.js:76 | 18 | keep | Rapporto di studio |  |
| `reset.title` | built at runtime | 28 | keep | Iniziare una nuova sessione? |  |
| `scorc.apri.tip` | azioni-tasti.js:39 | 25 | keep | Sceglie i file da aprire. |  |
| `scorc.elimina.tip` | azioni-tasti.js:33 | 39 | keep | Elimina il picco integrato selezionato. |  |
| `scorc.esc.tip` | azioni-tasti.js:36 | 88 | shorten | Un solo Esc alla volta: prima menu e finestre, poi misura, riproduzione, schermo intero. | Chiude una cosa alla volta: menu, misura, riproduzione, schermo intero / Closes one thing at a time: menus, measure, playback, full screen |
| `scorc.esporta.tip` | azioni-tasti.js:41 | 35 | keep | Esporta il rapporto della sessione. |  |
| `scorc.file.tip` | azioni-tasti.js:51 | 80 | keep | Passa al file prima o dopo nell'elenco, come le frecce accanto al menu dei file. |  |
| `scorc.integra.tip` | azioni-tasti.js:17 | 89 | shorten | Integra in automatico il picco sotto il cursore (come la voce del menu del tasto destro). | Integra il picco sotto il cursore / Integrates the peak under the cursor |
| `scorc.intera.tip` | azioni-tasti.js:30 | 84 | shorten | Torna all'intervallo intero del grafico attivo; con un picco selezionato lo elimina. | Vista intera del grafico attivo; con un picco scelto lo elimina / Whole view of the active chart; with a peak selected it deletes it |
| `scorc.misura.tip` | azioni-tasti.js:21 | 64 | keep | Prende il picco più alto visibile come riferimento della misura. |  |
| `scorc.pannelli.tip` | azioni-tasti.js:49 | 65 | shorten | Come Ctrl/Cmd+B: nasconde la barra laterale e la riporta com'era. | Nasconde la barra laterale e la riporta com'era / Hides the side bar and brings it back as it was |
| `scorc.schermo.tip` | azioni-tasti.js:24 | 70 | keep | Porta il pannello a schermo intero e di nuovo alla posizione di prima. |  |
| `scorc.titolo.tip` | azioni-tasti.js:54 | 74 | keep | Tutti i tasti del programma, generati dal registro delle azioni (tasto ?). |  |
| `scorc.vista.tip` | azioni-tasti.js:44 | 40 | keep | Cambia vista: Alt+1 Dati, Alt+2 Disegno. |  |
| `settings.beside.title` | built at runtime | 90 | follow-up (SETTINGS-PANEL) | Lo spettro aperto con il doppio clic va nella stessa riga del cromatogramma, se c'è spazio |  |
| `settings.cen.title` | settings.js:142 | 151 | follow-up (SETTINGS-PANEL) | Centroidazione: ogni picco del profilo diventa un punto con la sua massa e la sua area. Crea una copia accanto all'originale, che non viene modificato. |  |
| `settings.ext.title` | built at runtime | 179 | follow-up (SETTINGS-PANEL) | Solo app per il computer. Spento: nessuna richiesta a siti diversi da quello di mzLab. Acceso: le librerie pubbliche si scaricano dai loro siti ufficiali, senza inviare dati tuoi. |  |
| `settings.gear.title` | settings.js:224, index.html:17 | 45 | follow-up (SETTINGS-PANEL) | Impostazioni: testo, tema, colori dei grafici |  |
| `settings.hrDec.title` | built at runtime | 47 | keep | Decimali delle m/z nei file ad alta risoluzione |  |
| `settings.hrPpm.title` | built at runtime | 87 | follow-up (SETTINGS-PANEL) | Finestra di un XIC e confronto fra m/z nei file ad alta risoluzione (predefinito 5 ppm) |  |
| `settings.language.title` | built at runtime | 44 | keep | Lingua dell'interfaccia / Interface language |  |
| `settings.libs.title` | built at runtime | 112 | follow-up (SETTINGS-PANEL) | Carica le librerie (MSP, MGF) con cui confrontare le MS2: clic destro su uno spettro MS2, «Cerca nelle librerie» |  |
| `settings.motion.title` | built at runtime | 55 | follow-up (SETTINGS-PANEL) | Sempre ridotte; altrimenti segue la scelta del sistema. |  |
| `settings.pal.title` | settings.js:136 | 228 | follow-up (SETTINGS-PANEL) | Per file: un colore per file. Per tempo: dal viola al verde in ordine di tempo. Accessibili: distinguibili con il daltonismo, più linee tratteggiate. Alto contrasto: aggiunge lo stile della linea. Arcobaleno: tinte ben separate. |  |
| `settings.reset.title` | settings.js:158 | 94 | follow-up (SETTINGS-PANEL) | Riporta lingua, tema, colori, lettura e opzioni ai valori iniziali; i file aperti non cambiano |  |
| `settings.title` | built at runtime | 12 | keep | Impostazioni |  |
| `settings.xiclens.title` | built at runtime | 123 | follow-up (SETTINGS-PANEL) | Passando sul picco di uno spettro compare una piccola anteprima dello XIC di quello ione; un clic la apre come pannello XIC |  |
| `side.addf` | index.html:28 | 17 | keep | Carica altri file |  |
| `side.close` | index.html:28 | 18 | keep | Chiudi il pannello |  |
| `side.resize` | index.html:28 | 65 | keep | Trascina per regolare la larghezza (doppio clic per ripristinare) |  |
| `spec.dm.tip.ad` | built at runtime | 29 | keep | Clic destro: apre gli addotti |  |
| `spec.dm.tip.ls` | built at runtime | 39 | keep | Clic destro: cerca nelle perdite neutre |  |
| `spec.follow.none.title` | pannelli.js:734 | 62 | keep | Spettro fisso al suo tempo. Scegli un cromatogramma da seguire |  |
| `spec.follow.title` | pannelli.js:734 | 127 | shorten | Questo spettro segue il cromatogramma: un clic, il cursore o le frecce lo aggiornano. Scegli un altro cromatogramma o «nessuno» | Segue il cromatogramma scelto: clic, cursore o frecce lo aggiornano / Follows the chosen chromatogram: click, cursor or arrows update it |
| `spec.measure.toLosses.tip` | spettro.js:140 | 91 | shorten | Apre la tabella delle perdite neutre con questa differenza già scritta nel campo di ricerca | Perdite neutre con questa differenza già cercata / Neutral losses with this difference already searched |
| `spec.par.nlab.title` | spettro.js:154 | 31 | keep | Numero massimo di etichette m/z |  |
| `spec.par.reset.title` | spettro.js:158 | 29 | keep | Riporta ai valori predefiniti |  |
| `spec.par.sticks.title` | spettro.js:156 | 189 | shorten | File in profilo: la linea è lo spettro com'è registrato; i bastoncini sono le cime, una per massa nominale (grafico pulito per la relazione). Le etichette e la tabella usano sempre le cime. | Bastoncini: una cima per massa nominale; etichette e tabella usano le cime / Sticks: one top per nominal mass; labels and table use the tops |
| `spec.par.thr.title` | spettro.js:153 | 101 | shorten | Si etichettano solo i picchi sopra questa percentuale del picco più alto; gli altri restano disegnati | Etichette solo sui picchi sopra questa % del più alto / Labels only on peaks above this % of the highest |
| `storia.btn.redo` | index.html:33 | 26 | keep | Ripeti (Maiusc+Ctrl/Cmd+Z) |  |
| `storia.btn.undo` | index.html:33 | 20 | keep | Annulla (Ctrl/Cmd+Z) |  |
| `storia.pannello` | pannelli.js:161 | 8 | keep | pannello |  |
| `sub.chip.title` | sottrazione.js:131 | 73 | shorten | Riferimento sottratto: cambia file, fattore, normalizzazione e tolleranza | Cambia file, fattore, normalizzazione e tolleranza / Change file, factor, normalisation and tolerance |
| `sub.next` | sottrazione.js:131 | 37 | keep | File successivo (riferimento: {file}) |  |
| `sub.off.aria` | sottrazione.js:131 | 43 | keep | Togli la sottrazione e torna ai dati grezzi |  |
| `sub.pop.title` | built at runtime | 27 | keep | Sottrazione del riferimento |  |
| `sub.prev` | sottrazione.js:131 | 37 | keep | File precedente (riferimento: {file}) |  |
| `tbl.diff.title` | built at runtime | 31 | keep | Differenze di massa tra addotti |  |
| `tbl.loss.exactTitle` | tables.js:66 | 12 | keep | Massa esatta |  |
| `tbl.loss.polTitle` | tables.js:80 | 29 | keep | Polarità tipica della perdita |  |
| `tbl.loss.searchTitle` | tables.js:79 | 186 | follow-up (ADD-LOSS) | Scrivi la differenza di massa osservata: evidenzia le perdite con quel valore (secondo la tolleranza scelta) e mostra coppie e ripetizioni che la compongono, i contaminanti e gli addotti |  |
| `tdati.btn.title` | infusione-grafo.js:60, tdati.js:76 | 57 | keep | Dati del grafico in tabella (tasto T sul grafico a fuoco) |  |
| `tdati.close` | tdati.js:35 | 6 | keep | Chiudi |  |
| `tdati.title` | built at runtime | 16 | keep | Dati del grafico |  |
| `tip.area` | grafici.js:444 | 76 | keep | <b>Area</b> {area} <span class="sm">conteggi·s</span> · intensità <b>{y}</b> |  |
| `tip.more` | built at runtime | 15 | n/a (visible text) | ... e altre {n} |  |
| `tip.peak` | built at runtime | 51 | keep | <b>m/z {mz}</b><div>intensità <b>{y}</b>{abs}</div> |  |
| `tip.raw` | built at runtime | 10 | keep | grezzo {v} |  |
| `touch.grip.title` | touch.js:169 | 43 | keep | Trascina per cambiare l'altezza del grafico |  |
| `tree.title` | built at runtime | 10 | keep | Albero MSn |  |
| `update.settings.title` | aggiorna.js:78 | 88 | follow-up (SETTINGS-PANEL) | Cerca una versione più recente del programma sul sito; si scaricano solo i file cambiati |  |
| `valida.title` | built at runtime | 19 | n/a (visible text) | Valida mzML: {name} |  |
| `xic.adduct.title` | pannelli.js:938 | 183 | shorten | L'addotto trasforma la formula neutra nell'm/z dello ione: [M+H]+ aggiunge un protone; Nessun addotto: il testo è già lo ione (se contiene un addotto, come +H o [M+H]+, si usa quello) | Rende la formula neutra m/z dello ione; Nessun addotto: il testo è già lo ione / Turns the neutral formula into the ion m/z; No adduct: text is already the ion |
| `xic.remove.title` | grafici.js:268, pannelli.js:938 | 17 | keep | Togli questo ione |  |
| `xicbtn.ion.title` | pannelli.js:266 | 55 | shorten | Ione e tolleranza dello XIC: clic o Invio per scriverli | Clic o Invio per cambiare ione e tolleranza / Click or Enter to change ion and tolerance |
| `xicbtn.title` | pannelli.js:266 | 184 | shorten | Estrae subito lo XIC dell'ione scelto nello spettro e lo mostra nel pannello sotto. Senza un ione scelto: clic su un picco (o tasti , e .). Maiusc: aggiunge lo ione come traccia in più | XIC dell'ione scelto nello spettro, nel pannello sotto (Maiusc: traccia in più) / XIC of the ion chosen in the spectrum, in the panel below (Shift: extra trace) |
| `xicdlg.add.title` | index.html:109 | 72 | shorten | Aggiunge un altro ione (fino a 10): tutti finiscono nello stesso grafico | Fino a 10 ioni nello stesso grafico / Up to 10 ions in the same chart |
| `xicdlg.desc` | index.html:107 | 236 | n/a (visible text) | XIC (Extracted Ion Chromatogram): il cromatogramma dei soli segnali con m/z dentro un intervallo largo 1 Da attorno alla massa nominale dello ione. Si integra da qui. In ogni riga scrivi un m/z oppure una formula neutra (con l'addotto). |  |
| `xicdlg.file.title` | index.html:110 | 91 | shorten | Da quale file estrarre. Vale solo all'inizio: dopo si cambia dal selettore File del grafico | Vale solo all'inizio; poi dal selettore File del grafico / Applies only at the start; later use the chart's File selector |
| `xicdlg.title` | built at runtime | 17 | keep | Estrai ioni (XIC) |  |
| `xiclens.rt` | xic-lente.js:114 | 19 | keep | Tempo di ritenzione |  |
| `xiclens.toggle` | xic-spettro.js:256 | 30 | keep | Anteprima XIC attiva/disattiva |  |

## Literal tooltips (Tabui, Flow, runtime)
Text as found (the inventory cuts it at 90 characters); runtime rows are built from values and have no fixed text.

| Where | Len | Verdict | Current | Proposed |
|---|---|---|---|---|
| TP_Mine/js/19_tpmine_base.js:7 | 211 | shorten | Signal in the same file and time window at the m/z of the ion, at ±1 and ±2 Da (neighbours) and at ±0.5 Da (decoy windows). A neighbour or a decoy as strong as the ion says the signal is not specific to its m/z. | Neighbours at ±1, ±2 Da and decoys at ±0.5 Da: one as strong as the ion means the signal is not specific |
| TP_Mine/js/20_tpmine.js:123 | 3 | keep | why |  |
| TP_Mine/js/20_tpmine.js:16 | 33 | keep | .mzML, .raw, .wiff (+ .wiff.scan) |  |
| TP_Mine/js/20_tpmine.js:16 | 62 | keep | Reload the files already open in the Data tab (browser memory) |  |
| TP_Mine/js/20_tpmine.js:19 | 115 | shorten | Compare several conditions of the same degradation (gases, photocatalysts...): label the files with their condition | Label each file with its condition (gas, photocatalyst...) to compare them |
| TP_Mine/js/20_tpmine.js:199 | 57 | keep | From the chromatographic method (0.5 % of the run length) |  |
| TP_Mine/js/20_tpmine.js:23 | 64 | keep | Draw the molecule in the Draw tab and bring the SMILES back here |  |
| TP_Mine/js/20_tpmine.js:28 | 134 | shorten | Low resolution: propose the XIC window, the RT tolerance and the start of the analysis from the loaded files (yellow values, editable) | Low resolution: proposes XIC window, RT tolerance and start from the loaded files |
| TP_Mine/js/20_tpmine.js:57 | 32 | keep (runtime) | Remove ${esc(fileLabel(f.name))} |  |
| TP_Mine/js/21_tpmine_hr.js:34 | 89 | keep (runtime) | cosine: equal ${sim.cos[0]} · shifted ${sim.cos[1]} · common neutral losses ${sim.cos[2]} |  |
| TP_Mine/js/21_tpmine_hr.js:38 | 31 | keep (runtime) | cosine ${sim.cos[k].toFixed(3)} |  |
| TP_Mine/js/21_tpmine_hr.js:39 | 32 | keep (runtime) | entropy ${sim.ent[k].toFixed(3)} |  |
| TP_Mine/js/21_tpmine_hr.js:40 | 13 | keep | matched peaks |  |
| TP_Mine/js/21_tpmine_hr.js:98 | 79 | keep | Inclusion list for a second targeted injection: the best candidates without MS2 |  |
| TP_Mine/js/22_tpmine_ms2.js:265 | 37 | keep | Go to the menu of this MS2 experiment |  |
| TP_Mine/js/22_tpmine_ms2.js:92 | 34 | keep | Choose the row this MS2 belongs to |  |
| TP_Mine/js/23_tpmine_albero.js:179 | 11 | keep | Zoom in (+) |  |
| TP_Mine/js/23_tpmine_albero.js:179 | 12 | keep | Zoom out (-) |  |
| TP_Mine/js/23_tpmine_albero.js:179 | 7 | keep | Fit (0) |  |
| TP_Mine/js/27_untarget_ui.js:35 | 5 | keep | Close |  |
| TP_Mine/js/28_untarget_export.js:75 | 33 | keep | Text with ';' as separator, UTF-8 |  |
| TP_Mine/js/28_untarget_export.js:75 | 76 | keep | MS2 of the features with their level and the licences of the library records |  |
| TP_Mine/js/30_untarget_fair.js:50 | 106 | shorten | Quantification table + MGF (+ IIN edges) in a .zip, for FBMN / IIMN on GNPS2. Files only: nothing is sent. | Quantification table + MGF (+ IIN edges) in a .zip for GNPS2 FBMN/IIMN; nothing is sent |
| TP_Mine/js/30_untarget_fair.js:50 | 78 | keep | SIRIUS 6 .ms file: MS2 (and MS1) of the features. Files only: nothing is sent. |  |
| TP_Mine/js/32_pipeline.js:194 | 5 | keep | Close |  |
| TP_Mine/js/32_pipeline.js:195 | 25 | keep | Refresh the list of files |  |
| TP_Mine/js/33_pipeline_view.js:85 | 46 | keep | 1/N: N candidates the proofs cannot tell apart |  |
| TP_Mine/js/34_pipeline_run.js:126 | 38 | keep | Proposed from the name: Enter confirms |  |
| TP_Mine/js/34_pipeline_run.js:139 | 35 | keep | block ? "Run is off: " + block : "" |  |
| TP_Mine/js/34_pipeline_run.js:21 | 49 | keep | Accept the times proposed from the names (yellow) |  |
| TP_Mine/js/34_pipeline_run.js:23 | 83 | keep | Deletes the detections kept for the next run; the next run detects every file again |  |
| TP_Mine/js/34_pipeline_run.js:23 | 89 | keep | Runs the last recipe of the session or of the opened .mzflow again and compares the table |  |
| TP_Mine/js/43_trasformazioni.js:43 | 54 | keep | Show the list as plain text: name;change;note, # = off |  |
| TP_Mine/js/43_trasformazioni.js:67 | 19 | keep | Literature: level 2 |  |
| TP_Mine/js/52_attributio_banco.js:142 | 7 | keep | Zoom in |  |
| TP_Mine/js/52_attributio_banco.js:142 | 8 | keep | Zoom out |  |
| TP_Mine/js/52_attributio_banco.js:143 | 10 | keep | Move right |  |
| TP_Mine/js/52_attributio_banco.js:143 | 10 | keep | Reset zoom |  |
| TP_Mine/js/52_attributio_banco.js:143 | 9 | keep | Move left |  |
| TP_Mine/js/52_attributio_banco.js:275 | 12 | keep | Next feature |  |
| TP_Mine/js/52_attributio_banco.js:275 | 14 | keep | Close Bench 2b |  |
| TP_Mine/js/52_attributio_banco.js:275 | 16 | keep | Previous feature |  |
| TP_Mine/lazy/gruppi-campioni.js:43 | 51 | keep | Proposed from the name: Confirm conditions keeps it |  |
| TP_Mine/lazy/gruppi-campioni.js:44 | 38 | keep | Proposed from the name: Enter confirms |  |
| TP_Mine/lazy/gruppi-campioni.js:96 | 58 | keep | Keeps the conditions and the times proposed from the names |  |
| TP_Mine/lazy/gruppi-vista.js:155 | 23 | keep | area of the filled cell |  |
| TP_Mine/lazy/gruppi-vista.js:155 | 4 | keep | area |  |
| TP_Mine/lazy/gruppi-vista.js:156 | 52 | keep (runtime) | ratio to the reference${ref ? ` (${esc(ref)})` : ""} |  |
| TP_Mine/lazy/gruppi-vista.js:160 | 21 | keep (runtime) | Sort by ${esc(label)} |  |
| TP_Mine/lazy/gruppi-vista.js:166 | 17 | keep | negative polarity |  |
| TP_Mine/lazy/gruppi-vista.js:200 | 22 | keep | Zoom in the UpSet plot |  |
| TP_Mine/lazy/gruppi-vista.js:200 | 23 | keep | Zoom out the UpSet plot |  |
| TP_Mine/lazy/gruppi-vista.js:300 | 17 | keep | Zoom in the chart |  |
| TP_Mine/lazy/gruppi-vista.js:300 | 18 | keep | Zoom out the chart |  |
| TP_Mine/lazy/gruppi-vista.js:417 | 31 | keep | One row per TP (rows now shown) |  |
| TP_Mine/lazy/gruppi-vista.js:417 | 50 | keep | One row per TP × condition × time (rows now shown) |  |
| TP_Mine/lazy/gruppi-vista.js:418 | 28 | keep | The complete gruppi/1 output |  |
| TP_Mine/lazy/gruppi-vista.js:418 | 37 | keep | The sets of conditions and their rows |  |
| TP_Mine/lazy/reatt-auto.js:25 | 79 | keep | st.tpsBusy ? "The TPs are being calculated: the reactivity waits for them" : "" |  |
| appname.js:6 | 44 | keep (runtime) | document.title.split("{APP}").join(APP_NAME) |  |
| appname.js:8 | 44 | keep (runtime) | document.title.split("{APP}").join(APP_NAME) |  |
| avvio.js:4 | 6 | keep (runtime) | titolo |  |
| banco.js:63 | 67 | keep | q.type === "chrom" ? PT.chrom : nt === "ms2" ? PT.product : PT.spec |  |
| banco.js:67 | 8 | keep | PT.chrom |  |
| barra.js:580 | 4 | keep | full |  |
| barra.js:712 | 7 | keep | r.title |  |
| dda.js:118 | 60 | keep | MS2 di  + (D.prec[i] != null ? D.prec[i].toFixed(dec) : "?") |  |
| dda.js:200 | 14 | keep | T:  + j.filter |  |
| dda.js:309 | 3 | keep | MS2 |  |
| disegno-barra.js:63 | 8 | keep | t(g.tip) |  |
| disegno-barra.js:68 | 9 | keep | t(it.tip) |  |
| draw.js:208 | 8 | keep | it.title |  |
| draw.js:88 | 10 | keep | I18N.t(key |  |
| elementare.js:62 | 26 | keep (runtime) | ${EH(I18N.t(`el.${e.s}`))} |  |
| eventi.js:278 | 43 | keep | s.level === 2 ? ptProduct(s.prec) : PT.spec |  |
| explore.js:884 | 24 | keep | one ? why : b.dataset.t0 |  |
| explore.js:885 | 9 | keep | cur.label |  |
| explore.js:952 | 18 | keep | it.tip \|\| it.label |  |
| flow/lazy/flow-live-card.js:36 | 22 | keep (runtime) | `${note} · ${v.title}` |  |
| flow/lazy/flow-live-card.js:37 | 22 | keep (runtime) | `${v.title} (${note})` |  |
| flow/lazy/flow-live-card.js:40 | 7 | keep | v.title |  |
| flow/lazy/flow-meta-grid.js:285 | 21 | keep | mg.input.sha256 \|\| "" |  |
| flow/lazy/flow-meta-grid.js:464 | 31 | keep | No file is open in the Data tab |  |
| flow/lazy/flow-palette.js:49 | 90 | keep (runtime) | reg.byId.get(id).doc + (miss.length ? `\nNothing in the plan can feed yet: ${miss.join(",  |  |
| flow/lazy/flow-shell.js:81 | 90 | keep | bad ? "The plan has things to fix first. The runner refuses it until they are solved." : " |  |
| flow/lazy/flow-views-attributio.js:54 | 1 | keep | v |  |
| fondo-chip.js:15 | 5 | keep (runtime) | title |  |
| index.html:11 | 5 | keep | {APP} |  |
| index.html:12 | 44 | keep | Automatic search for transformation products |  |
| index.html:12 | 82 | keep | Build a chain of steps from the step descriptors and export it as an mzflow/2 plan |  |
| index.html:8 | 1 | keep | T |  |
| infusione-grafo-regole.js:38 | 12 | keep (runtime) | DOI ${f.doi} |  |
| infusione-grafo-schermo.js:17 | 9 | keep | I18N.t(k) |  |
| infusione-grafo.js:60 | 14 | keep | ' + EH(I18N.t( |  |
| libreria-finestra.js:213 | 13 | keep | c.textContent |  |
| llm/llm-clienti.js:56 | 5 | keep (runtime) | title |  |
| mappa.js:830 | 25 | keep (runtime) | ${EH(T(`cmd.${id}.tip`))} |  |
| mappa.js:839 | 2 | keep | nm |  |
| metodo-incl.js:80 | 3 | keep | tip |  |
| pannelli-annulla.js:25 | 16 | keep | JSON.parse(snap) |  |
| pannelli.js:360 | 35 | keep | v === ptitle(p.title) ? p.title : v |  |
| pannelli.js:62 | 25 | keep | I18N.t(key, { n: b.num }) |  |
| pannelli.js:625 | 3 | keep | ttl |  |
| pannelli.js:653 | 25 | keep | == PT.chrom ? "none" : "" |  |
| pannelli.js:799 | 3 | keep | tip |  |
| picchi.js:238 | 27 | keep (runtime) | ${I18N.t(`int.tbl.${k}.h`)} |  |
| picchi.js:476 | 39 | keep | (p._a.hov(px) \|\| { html: "" }).html }); |  |
| picchi.js:499 | 17 | keep | p.leg.textContent |  |
| scroll.js:133 | 1 | keep | t |  |
| shell.js:169 | 9 | keep | I18N.t(k) |  |
| shell.js:40 | 1 | keep | T |  |
| tabs.js:234 | 11 | keep | ptMs2(prec) |  |
| tabs.js:236 | 15 | keep | ptProduct(prec) |  |
| telefono-scheda.js:10 | 11 | keep | ' + I18N.t( |  |
| tpmine-loader.js:100 | 16 | keep | t.desc \|\| t.name |  |
| tpmine-loader.js:106 | 16 | keep | t.desc \|\| t.name |  |
| upload-rows.js:59 | 1 | keep | l |  |
| xic-lente.js:94 | 11 | keep | withFile(s) |  |
| xic-spettro.js:76 | 3 | keep | tip |  |
