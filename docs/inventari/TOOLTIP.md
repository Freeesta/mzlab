# Inventario dei tooltip

Elenco di ogni tooltip del sito (`mzlab/web`, Tabui `TP_Mine`, Flow `flow/lazy`), classificato. Le tabelle per zona (`TOOLTIP-pannelli-1.md`, `-pannelli-2`, `-spettri`, `-liste`, `-disegno`, `-tabui`) sono generate: `python3 tools/inventario_tooltip.py --md docs/inventari` (le icone senza tooltip: `--gaps`). Qui: meccanismo, regola, hover dei grafici, mancanti, decisioni.

## Come funzionano
- **Tooltip dei controlli**: l'attributo `title` (o `.title =`, `data-i18n-title`). Il programma lo sostituisce con il suo tooltip discreto (`eventi.js`, blocco «discreet tooltip»): al primo passaggio il `title` diventa `data-tip`, il riquadro `#qtip` compare dopo 1,25 s (1 s nelle righe `.fl`) e non compare dentro una `dialog` aperta. `data-tiph` = tooltip con HTML. Al tocco non c'è hover: il `title` non si vede (la penna sì, `touch.js`).
- **Hover dei grafici** (canvas): `p._a.hov` restituisce l'HTML del riquadro `p.tip` (`grafici.js` `showHover`); le etichette dei picchi/annotazioni hanno il proprio `tip` (`_a.lbls`, `eventi.js`).
- **Grafici SVG**: elementi `<title>` (`pca.js`, `kmd-vista.js`, `pfas.js`, `profilo.js`, `qaqc.js`).
- **Testi**: chiavi in `lang/it` e `lang/en` (nessuna chiave senza l'altra lingua); i punti senza chiave sono costruiti a runtime o sono Tabui/Flow (solo inglese).

## Numeri (508 punti, 318 chiavi distinte)
| zona | punti | A | C | D | non tooltip |
|---|---|---|---|---|---|
| pannelli, barre, legende | 161 | 152 | 0 | 0 | 9 |
| spettri, mappa, DDA, XIC, integrazione | 86 | 82 | 0 | 1 | 3 |
| file, liste, librerie, tabelle, calcolatrice | 132 | 119 | 2 | 0 | 11 |
| disegno, grafo, infusione | 57 | 56 | 0 | 0 | 1 |
| Tabui e Flow | 72 | 70 | 0 | 0 | 2 |
Per tipo: 330 `title="…"`, 39 `data-i18n-title`, 85 `.title =` (26 righe tra queste e le altre sono proprietà dei pannelli o titoli di pagina, non tooltip), 30 `tip:` nei menu, 19 `<title>` SVG. I 4 tooltip sopra 200 caratteri: `grafici.js:219`, `pannelli.js:713`, `settings.js:149`, `index.html:81`; 25 sopra 150.

**Risultato**: tra i `title` dei controlli la ridondanza vera è quasi assente (uno ripeteva la scritta del pulsante, due erano rumore: vedi «Fatto in questa PR»). Quasi tutti spiegano un'icona, un'unità o un effetto. I tooltip inutili stanno nei **grafici** (hover ed etichette dei picchi) e nel formato non unico dell'hover.

## Classi
- **A utile**: aggiunge ciò che non si vede (unità, scorciatoia, effetto, spiegazione di un'icona). Quasi tutti i punti delle tabelle.
- **B ridondante**: ripete l'etichetta o il valore già scritti accanto.
- **C da rifare**: incompleto o incoerente (formato diverso dagli altri, testo in lingua sbagliata o che nomina file interni).
- **D da togliere**: rumore (per esempio solo «Right-click: actions»).
- **-**: la riga contiene `.title` ma è una proprietà del pannello o della pagina, non un tooltip.

## Regola proposta (da approvare)
1. Un tooltip esiste solo se aggiunge informazione rispetto a ciò che è visibile; mai la copia dell'etichetta o del valore.
2. I pulsanti con sola icona hanno SEMPRE tooltip con nome e, se c'è, scorciatoia (oggi sono ~20 senza: tabella «Mancanti»).
3. Ogni grafico mostra tempo, m/z e intensità nello stesso formato, prodotto da una sola funzione.
4. I suggerimenti sui gesti («Tasto destro: azioni») non stanno nel tooltip: stanno nell'aiuto e nella Teoria.
Guardia: `tests/test_tooltip_inventario.py` (nessun tooltip uguale al testo del proprio elemento, nessuno fatto solo di «Right-click: actions»; le due eccezioni sono quelle di CROMA-CLICK-PICCO e solo calano).

## Hover dei grafici (da tenere insieme)
| dove | cosa mostra oggi | classe | nota |
|---|---|---|---|
| `grafici.js:446` cromatogramma, XIC, MRM | **RT min**, poi per traccia: nome e intensità (+ «grezzo»); fino a 8 righe | A/C | niente unità (cps) e niente m/z della traccia |
| `picchi.js:410` spettro (`tip.peak`) | **m/z**, «intensità» e valore, cps assoluti se in %, Δm/z dal riferimento, file | C | formato diverso dal cromatogramma (CROMA-CLICK-PICCO) |
| `grafici.js:596` mappa 2D (`map.hov`) | **RT · m/z**, «intensità» o «differenza», bin di m/z | A | è il formato più completo |
| `grafici.js:596` mappa PDA | RT min, nm, mAU scritti nel codice, non da `lang/` | C | unità fuori dai cataloghi |
| `uv-spettro.js:101` spettro UV | nm e mAU scritti nel codice | C | come sopra |
| `grafici.js:437` area del picco (`tip.area`) | Area, conteggi·s, «Right-click: actions» | D (riga) | solo la riga del tasto destro; CROMA-CLICK-PICCO |
| `picchi.js:458` etichetta m/z (`tip.peakLabel`) | m/z e «Right-click: actions» | D | l'm/z è già sull'etichetta; CROMA-CLICK-PICCO |
| `etichette-picchi.js:54` etichetta RT (`etpk.tip.rt`) | «Peak at RT 14.34 min» | B | ripete l'etichetta; CROMA-CLICK-PICCO |
| `spettro.js:100` etichette Δ del righello | addotto, Δ, «Clic destro: …» | C | un solo suggerimento di gesto |
| `grafici.js:156` annotazione (`draw.ann.tip`) | il testo della nota + «Doppio clic: modifica · tasto destro: azioni» | C | il testo si ripete; restano i gesti |
| `dda.js:247,257` marcatori DDA | titolo, n. MS2, precursore | A | |
| `mappa.js:53` etichette dello spettro laterale | m/z, S/N | A | |
| SVG: `pca.js:87`, `kmd-vista.js:37`, `pfas.js:102`, `profilo.js:84`, `qaqc.js` | campione, m/z, RT, valore | A | formati propri, ognuno con la sua unità |

## Mancanti: pulsanti solo icona con `aria-label` e senza tooltip
Mostrano il nome ai lettori di schermo ma non al mouse (da completare con tooltip + scorciatoia).
| file:line | segno | aria-label o testo |
|---|---|---|
| banco.js:449 | × | ${I18N.t( |
| elementare.js:45 | × | ${t( |
| elementare.js:48 | × | ${t( |
| explore.js:808 | × | ${EH(I18N.t( |
| fair.js:138 | &times; | ${T( |
| pca.js:128 | ? | ${eh(t( |
| picchi.js:231 | ${IC_DL}Excel |  |
| picchi.js:231 | ${IC_DL}CSV |  |
| qaqc.js:163 | × | ${eh(t( |
| shell.js:122 | ? | ${I18N.t( |
| index.html:74 | ( |  |
| index.html:74 | ) |  |
| index.html:76 | &times; |  |
| index.html:78 | , |  |
| index.html:78 | + |  |
| TP_Mine/js/20_tpmine.js:57 | × | Remove ${esc(fileLabel(f.name))} |
| TP_Mine/js/27_untarget_ui.js:35 | &times; | Close |
| TP_Mine/js/32_pipeline.js:194 | &times; | Close |
| TP_Mine/js/32_pipeline.js:195 | &#8635; | Refresh the list of files |
| TP_Mine/js/52_attributio_banco.js:142 | + | Zoom in |
| TP_Mine/js/52_attributio_banco.js:142 | − | Zoom out |
| TP_Mine/js/52_attributio_banco.js:143 | ← | Move left |
| TP_Mine/js/52_attributio_banco.js:143 | → | Move right |
| TP_Mine/js/52_attributio_banco.js:275 | ← | Previous feature |
| TP_Mine/js/52_attributio_banco.js:275 | → | Next feature |
| TP_Mine/lazy/gruppi-vista.js:200 | − | Zoom out the UpSet plot |
| TP_Mine/lazy/gruppi-vista.js:200 | + | Zoom in the UpSet plot |
| TP_Mine/lazy/gruppi-vista.js:300 | − | Zoom out the chart |
| TP_Mine/lazy/gruppi-vista.js:300 | + | Zoom in the chart |

## Fatto in questa PR (prima → dopo)
| dove | prima | dopo |
|---|---|---|
| `fondo-piatto.js:67` pulsante «↺ ripristina automatico» | tooltip «ripristina automatico» | nessuno (ripeteva la scritta) |
| `mappa3d.js:184` pulsante «Ripristina (R)» | tooltip «R» | nessuno (la scritta ha già la scorciatoia) |
| `tpmine-loader.js:125` pulsante «Close» di Tabui | tooltip «Close» | nessuno |
| `tbl.loss.exactTitle` (m/z esatto nella tabella Perdite neutre) | «Massa esatta, da elements.py» / «Exact mass, from elements.py» | «Massa esatta» / «Exact mass» |

## Lasciato come proposta
- Tutto ciò che riguarda le etichette dei picchi, l'hover dello spettro e «Right-click: actions» (`tip.area`, `tip.peakLabel`, `etpk.tip.rt`, `tip.peak`): lo sta cambiando CROMA-CLICK-PICCO.
- Funzione unica dell'hover dei grafici (tempo, m/z, intensità con unità) e unità da `lang/` per mappa PDA e spettro UV.
- Tooltip con i gesti ripetuti su ogni elemento («Clic: mostra o nascondi» nelle legende, «Clic: mostra solo questo file»): da spostare nell'aiuto.
- `liste-target.js:75,145`: tooltip in inglese scritti nel codice, da portare in `lang/it` e `lang/en`.
- I 4 tooltip sopra 200 caratteri da accorciare o spostare nell'aiuto; `draw.ann.tip` e `spec.dm.tip.*` (un tooltip che è solo un gesto).

## Decisioni per Federico
1. Approvi la regola «un tooltip solo se aggiunge informazione» e il «sempre tooltip con nome e scorciatoia sulle icone»?
2. Formato unico dell'hover dei grafici: «RT 14.34 min · m/z 301.1 · intensità 2.3e5 cps». Va bene, con una sola funzione?
3. I suggerimenti sui gesti (tasto destro, doppio clic) escono dai tooltip e vanno nell'aiuto: sì o no?
4. Aggiungo il tooltip ai ~20 pulsanti solo icona che ne sono privi?
5. I 4 tooltip lunghi (oltre 200 caratteri): accorciarli o spostarli nell'aiuto?
6. Porto in `lang/` i due tooltip inglesi di `liste-target.js`?
7. Tooltip dell'etichetta del picco (RT ripetuto): tenere solo l'area e l'intensità, come proposto da CROMA-CLICK-PICCO?
8. Tabui e Flow: stessa regola (restano in inglese)?
9. Al tocco il tooltip non si vede: va bene così o serve un modo (pressione lunga)?
10. La guardia dei test (nessun tooltip uguale alla propria scritta) resta obbligatoria?
