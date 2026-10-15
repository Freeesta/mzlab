// @ts-check
// TPMINE-PRIVATE  TABUI_T / TT: the help texts of Tabui in Italian and English (labels stay short and in English; the explanation lives here). TT(key, params)
// takes the language of the page (I18N.lang, or ?lang=), falls back to English, and replaces {name}.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  /** @type {{ it: Record<string, string>, en: Record<string, string> }} */
  const T = {
  it: {
    "ft.col.level": "Livello di identificazione (scala di Schymanski): L5 compare col trattamento, L4 formula unica, L3 MS2 legata al progenitore. Il 2b lo scrive solo il chimico.",
    "ft.col.pred": "Previsione (Löffler et al. 2025): quanto la trasformazione è attesa, da 4 regola generica a 1 confermata dai dati. È un'altra scala: «Prev. 3» non è «L3».",
    "ft.col.identity": "Che cosa potrebbe essere: formula e trasformazione dal progenitore, o «m/z … (unexpected)» per uno ione nuovo.",
    "ft.col.mz": "m/z misurato (apice). In bassa risoluzione è un candidato, non un'identificazione.",
    "ft.col.rt": "Tempo di ritenzione dell'apice, in minuti.",
    "ft.col.trend": "Area nei campioni trattati in ordine di tempo; i simboli sotto dicono com'è stata ottenuta ogni area.",
    "ft.col.cells": "Un simbolo per campione: com'è stata ottenuta l'area. Apri la riga, scheda Samples, per i numeri.",
    "ft.col.maxarea": "Area più alta fra i campioni (unità del motore). Il valore intero è nell'aiuto della cella.",
    "ft.col.score": "Punteggio di ordinamento (0-100): quota dei criteri superati. Non è una probabilità.",
    "ft.state.detected": "● rilevata: il picco è stato trovato dal rilevamento in questo campione.",
    "ft.state.filled": "◐ riempita: nessun picco trovato; l'area è stata riletta dove la feature era attesa.",
    "ft.state.targeted": "◌ mirata: cercata apposta (ipotesi o lista), non trovata dal rilevamento libero.",
    "ft.state.absent": "○ assente: nessun segnale.",
    "ft.chip.changed": "Solo le righe che compaiono o crescono col trattamento.",
    "ft.chip.noblank": "Togli le righe trovate nel bianco o nel buio.",
    "ft.chip.not0": "Togli le righe già presenti al t0.",
    "ft.chip.ms2": "Solo le righe con almeno una MS2.",
    "ft.chip.level": "Solo le righe a questo livello o migliore.",
    "ft.chip.diagnostic": "Prove diagnostiche: sito determinato con margine, RT e cinetica coerenti. Solo il chimico può farne un 2b.",
    "ft.chip.unexpected": "Ioni nuovi che non vengono da una regola di trasformazione.",
    "ft.export.view": "Esporta le righe e le colonne che vedi, nell'ordine in cui le vedi, con i filtri scritti in testa.",
    "ft.pin.parent": "Il progenitore resta in cima come riferimento: filtri e ordinamento non lo spostano.",
    "ft.cols": "Aggiungi o togli colonne facoltative; le prime due restano sempre.",
    "ft.row.open": "Clic o Invio: la riga si espande qui sotto con tutto ciò che si sa. Esc la chiude.",
    "ft.tab.overview": "XIC di tutti i campioni con i bordi del picco, e lo spettro (MS1 con gli isotopi, MS2).",
    "ft.tab.samples": "Per ogni campione: tipo e tempo, area, stato, RT dell'apice e bordi del picco.",
    "ft.tab.identity": "Massa neutra, addotto, candidati con le loro prove, livello proposto e livello deciso.",
    "ft.tab.quality": "Forma del picco, segnale/rumore, larghezza, isotopo M+1, presenza nel bianco e nel t0.",
    "ft.peak.borders": "Inizio e fine del picco (mediana dei file che lo hanno rilevato).",
    "ft.q.flags": "Avvisi sul picco: forma irregolare, segnale/rumore basso, pochi punti.",
    "ft.adduct": "Ione che il motore ha scelto per questa riga ([M+H]+, [M+Na]+…) e massa neutra che ne deriva.",
    "ft.showdata": "Apre il file nella vista Dati con l'XIC di questa m/z.",
    "ft.decided": "Il livello che decidi tu. Resta nel browser per questa serie ed entra nelle esportazioni.",
    "ft.more": "Altri risultati della serie, uno aperto alla volta.",
    "ft.f.name": "Nome del composto progenitore: scrivi 2 lettere per cercarlo nell'elenco, oppure lascia vuoto e dai la formula.",
    "ft.f.formula": "Formula molecolare neutra del progenitore, per esempio C10H12N2O3S. Se manca, il motore usa lo SMILES.",
    "ft.f.smiles": "Struttura del progenitore (facoltativa se c'è la formula): serve ai frammenti e ai gruppi delle regole.",
    "ft.f.adduct": "Ione con cui il progenitore compare nello spettro.",
    "ft.f.prt": "Tempo di ritenzione del progenitore, in minuti. Vuoto = automatico.",
    "ft.f.tol": "Finestra dell'XIC attorno a ogni m/z, in Da (o in ppm in alta risoluzione).",
    "ft.f.rtt": "Quanto può spostarsi l'RT di un picco fra i file, in minuti.",
    "ft.f.steps": "Quante trasformazioni si possono combinare in un solo prodotto (1-3).",
    "ft.f.minf": "Bassa risoluzione: il picco deve comparire in almeno questi campioni trattati, o in due consecutivi allo stesso RT. 0 = spento.",
    "ft.f.rtmin": "Ignora ciò che esce prima di questo RT (fronte non trattenuto), in minuti.",
    "ft.f.disc": "Cerca anche ioni che crescono nel tempo senza venire da una regola di trasformazione. Più lento.",
    "ft.f.chg": "Variazione di formula della regola, per esempio +O-H2."
  },
  en: {
    "ft.col.level": "Identification level (Schymanski scale): L5 appears with the treatment, L4 unique formula, L3 MS2 linked to the parent. Only the chemist writes 2b.",
    "ft.col.pred": "Prediction (Löffler et al. 2025): how expected the transformation is, from 4 generic rule to 1 confirmed by the data. A different scale: «Prev. 3» is not «L3».",
    "ft.col.identity": "What it might be: formula and transformation from the parent, or «m/z … (unexpected)» for a new ion.",
    "ft.col.mz": "Measured m/z (apex). At unit resolution it is a candidate, not an identification.",
    "ft.col.rt": "Retention time of the apex, in minutes.",
    "ft.col.trend": "Area in the treated samples in time order; the symbols below tell how each area was obtained.",
    "ft.col.cells": "One symbol per sample: how the area was obtained. Open the row, Samples tab, for the numbers.",
    "ft.col.maxarea": "Highest area among the samples (engine units). The full value is in the cell help.",
    "ft.col.score": "Ranking score (0-100): share of the criteria passed. Not a probability.",
    "ft.state.detected": "● detected: the peak was found by the detection in this sample.",
    "ft.state.filled": "◐ filled: no peak was found; the area was re-read where the feature was expected.",
    "ft.state.targeted": "◌ targeted: searched on purpose (hypothesis or list), not found by the free detection.",
    "ft.state.absent": "○ absent: no signal.",
    "ft.chip.changed": "Only the rows that appear or grow with the treatment.",
    "ft.chip.noblank": "Hide the rows found in the blank or dark control.",
    "ft.chip.not0": "Hide the rows already present at t0.",
    "ft.chip.ms2": "Only the rows with at least one MS2.",
    "ft.chip.level": "Only the rows at this level or better.",
    "ft.chip.diagnostic": "Diagnostic evidence: site determined with margin, consistent RT and kinetics. Only the chemist can turn it into 2b.",
    "ft.chip.unexpected": "New ions that do not come from a transformation rule.",
    "ft.export.view": "Export the rows and columns you see, in the order you see them, with the filters written at the top.",
    "ft.pin.parent": "The parent stays at the top as a reference: filters and sorting do not move it.",
    "ft.cols": "Add or remove optional columns; the first two always stay.",
    "ft.row.open": "Click or Enter: the row expands right below with everything that is known. Esc closes it.",
    "ft.tab.overview": "XIC of all the samples with the peak borders, and the spectrum (MS1 with isotopes, MS2).",
    "ft.tab.samples": "For every sample: type and time, area, state, apex RT and peak borders.",
    "ft.tab.identity": "Neutral mass, adduct, candidates with their evidence, proposed level and decided level.",
    "ft.tab.quality": "Peak shape, signal/noise, width, M+1 isotope, presence in blank and t0.",
    "ft.peak.borders": "Start and end of the peak (median of the files that detected it).",
    "ft.q.flags": "Peak warnings: irregular shape, low signal/noise, few points.",
    "ft.adduct": "Ion the engine chose for this row ([M+H]+, [M+Na]+…) and the neutral mass it gives.",
    "ft.showdata": "Opens the file in the Data view with the XIC of this m/z.",
    "ft.decided": "The level you decide. Kept in the browser for this series and written in the exports.",
    "ft.more": "Other results of the series, one open at a time.",
    "ft.f.name": "Name of the parent compound: type 2 letters to search the list, or leave it empty and give the formula.",
    "ft.f.formula": "Neutral molecular formula of the parent, for example C10H12N2O3S. If missing, the engine uses the SMILES.",
    "ft.f.smiles": "Structure of the parent (optional when there is a formula): used by the fragments and the groups of the rules.",
    "ft.f.adduct": "Ion with which the parent appears in the spectrum.",
    "ft.f.prt": "Retention time of the parent, in minutes. Empty = automatic.",
    "ft.f.tol": "XIC window around every m/z, in Da (or in ppm at high resolution).",
    "ft.f.rtt": "How far the RT of a peak may move between files, in minutes.",
    "ft.f.steps": "How many transformations can be combined in one product (1-3).",
    "ft.f.minf": "Low resolution: a peak must appear in at least this many treated samples, or in two consecutive ones at the same RT. 0 = off.",
    "ft.f.rtmin": "Ignore what elutes before this RT (unretained front), in minutes.",
    "ft.f.disc": "Also look for ions that grow over time without coming from a transformation rule. Slower.",
    "ft.f.chg": "Formula change of the rule, for example +O-H2."
  }
  };
  /** @param {string} key @param {Record<string, string | number>} [params] */
  const TT = (key, params) => {
    let lang = "en";
    try { const q = new URLSearchParams(location.search).get("lang"); lang = q === "it" || q === "en" ? q : (G.I18N && G.I18N.lang === "it" ? "it" : "en"); } catch (_) { /* English */ }
    let s = (T[lang] && T[lang][key]) || T.en[key] || "";
    if (params) s = s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
    return s;
  };
  G.TABUI_T = T; G.TT = TT;
})();
