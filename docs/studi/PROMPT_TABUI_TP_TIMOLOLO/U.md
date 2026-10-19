# U — Schede «MS2» e «Structure» della riga, disegno del TP: TT-U1

Prompt pronto per una chat DEV (Sonnet): copia tutto ciò che segue la riga «---», poi le regole di [COMUNI.md](COMUNI.md). Due PR in fila: la scheda MS2 quando TT-R1 è in `main`, la scheda Structure quando TT-R2 è in `main`.

---

Pacchetto TT-U1 dello studio `docs/studi/TABUI_TP_TIMOLOLO.md` (leggi §0, §5, §7-§9, §13). File d'area: `docs/agenti/mzfeat-tabella-vista.md`, `docs/agenti/mzfinder-tabella.md`, `docs/agenti/interp.md` (scritto da TT-R1), `docs/agenti/posizioni.md` (solo «Pannello»), `docs/agenti/disegno.md` (solo le righe di `TPDraw`).

## Obiettivo
Nella riga espansa della Pipeline (`TPRIGA`, `TP_Mine/js/44_tp_riga.js`) due schede nuove dopo «Identity»: «MS2» e «Structure», con i dati di `table_interp(id)`; il TP disegnato sul genitore; «Hidden: N» col registro dei filtri.

## PR 1: scheda MS2 e registro
1. `44_tp_riga.js`: scheda «MS2» (`role=tab`): tabella delle scansioni collegate (file col colore del tempo, RT, CE, finestra, purezza in parola e numero, «chimeric»), riga «MS2 in k of n files», spettro di consenso a specchio col genitore con tre segni diversi per uguali, spostati, perdite (forma + colore), coseno modificato ed entropia, `TDATI`, zoom con pulsanti e tasti. Senza MS2: «No MS2 in any file: needs targeted MS2» e il pulsante con icona «Add to inclusion list» (aggiunge m/z e RT a una lista esportabile in CSV dalla barra della tabella).
2. `42_tp_table.js`: sopra la tabella «Hidden: N» (pulsante): apre l'elenco dei filtri di `filtri/1` (nome, soglia, quante righe) e un clic su un filtro mostra le sue righe nella tabella con il motivo; «Export view» aggiunge il foglio «Filters».
3. `40_tabui_testi.js`: aiuti brevi in inglese di ogni colonna, simbolo e filtro nuovo.
4. e2e `tests_e2e/e2e_tabui_ms2_riga.py` (nuovo): serie sintetica, scheda MS2, tastiera (←/→ fra le schede, Esc), axe senza violazioni, «Hidden: N» coerente col registro.

## PR 2: scheda Structure e disegno
1. `TP_Mine/js/46_tp_mol.js` (nuovo, `// @ts-check`, globale `TPMOL` assente altrove: `grep -rn`): `TPMOL.svg(molfile, {highlight:[{atoms, color, label}], grey:[atoms]})` → SVG (legami 1/2/3/aromatici, eteroatomi con etichetta, numero della classe accanto a ogni atomo candidato oltre al colore, `role="img"` + `aria-label` con formula e atomi candidati). Nessun calcolo chimico: il molfile arriva da Rust.
2. Coordinate del genitore: al primo disegno della serie `TPDraw.load(smiles)` e lettura del molfile (via esistente del Disegno, nessuna modifica a `draw.js`); ricordato in IndexedDB `mzlab_lab` con l'impronta dello SMILES; se il chimico ha già usato «Use this structure», quel molfile. Il molfile entra in `interp_build` (TT-R2) per le varianti.
3. Scheda «Structure»: frase di testa («+O · Position uncertain · 3 classes · relative support 0.6 / 0.3 / 0.1», oppure «Site: …», «Region: ring A», «Cannot say: <reason in words>»), disegno del genitore con le classi colorate, miniature delle varianti in fila (prima la più sostenuta; clic = ingrandisce; Invio uguale), pulsante con icona «Open in Draw» (`TPDraw.load(molfile)` + `#/disegno`), tabella delle classi (atomi, tipo, supporto, stato ✓ ✗ ≈ in parole, frammenti a favore e contro), albero del genitore con i nodi uguali/spostati (`23_tpmine_albero.js`, solo l'ingresso dei dati), limiti in una riga, suggerimenti con DOI. Nessun pulsante «Compute»: la scheda si apre già piena.
4. Colonna «Identity»: «+O (ring A?)» con `region`, «+O (?)» con `uncertain`.
5. Tour: se cambia ciò che il tour guarda, `tour.js` e `e2e_tour.py` restano validi.
6. e2e `tests_e2e/e2e_tabui_struttura.py` (nuovo): sito unico sintetico disegnato, isomero con «Position uncertain» e tutte le varianti, «Open in Draw» apre il Disegno con la struttura, tema chiaro e scuro, 390 px senza scorrimento orizzontale della pagina, `prefers-reduced-motion`, axe.

## Atteso
- Le due schede si aprono in un clic dalla riga; nessun testo italiano in Tabui; nessuna struttura scritta come identificazione; `docs/agenti/mzfeat-tabella-vista.md` e `mzfinder-tabella.md` descrivono le schede.
