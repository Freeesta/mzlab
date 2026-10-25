// @ts-strict
// State and requests of the «Fragmentation graph» tab of an MSn infusion: the ion typed or chosen by name, the settings, the user's formula choices
// and the graph (msngrafo/1) that /api/msn_grafo answers. No chemistry here: the engine reads the ion text and does every calculation.
// Used by infusione-grafo*.js.

/** Texts of the evidences, of the parity of the electrons and of the warnings, spelled out so the catalogue check sees every key used. */
export const TXT = {
  conf: { high: "msngrafo.int.high", medium: "msngrafo.int.medium", low: "msngrafo.int.low" },
  prove: { perdita: "msngrafo.prove.perdita", isotopi: "msngrafo.prove.isotopi", msn: "msngrafo.prove.msn" },
  prog: { load: "msngrafo.prog.load", convert: "msngrafo.prog.convert", index: "msngrafo.prog.index", graph: "msngrafo.prog.graph" },
  stalled: { load: "msngrafo.stalled.load", convert: "msngrafo.stalled.convert", index: "msngrafo.stalled.index", graph: "msngrafo.stalled.graph" },
  elettroni: { pari: "msngrafo.e.pari", dispari: "msngrafo.e.dispari" },
  // Errors of the library route, named by the engine through error_key.
  err: { noIon: "err.msnlib.noIon", bad: "err.msnlib.bad" },
  warn: {
    "warn.msngrafo.isolation": "warn.msngrafo.isolation", "warn.msngrafo.precNotFound": "warn.msngrafo.precNotFound", "warn.msngrafo.polarity": "warn.msngrafo.polarity",
    "warn.msngrafo.badFormula": "warn.msngrafo.badFormula", "warn.msngrafo.rootMissing": "warn.msngrafo.rootMissing", "warn.msngrafo.fixedNotFound": "warn.msngrafo.fixedNotFound",
    "warn.msngrafo.userConflict": "warn.msngrafo.userConflict", "warn.msngrafo.isolatedUnexplained": "warn.msngrafo.isolatedUnexplained", "warn.msngrafo.mostUnexplained": "warn.msngrafo.mostUnexplained",
    "warn.msngrafo.structMixture": "warn.msngrafo.structMixture", "warn.msngrafo.radicalLoss": "warn.msngrafo.radicalLoss", "warn.msngrafo.belowCutoff": "warn.msngrafo.belowCutoff",
  },
};

export const ADDUCTS = { 1: ["[M+H]+", "[M+Na]+", "[M+NH4]+", "[M+K]+"], [-1]: ["[M-H]-", "[M+HCOO]-", "[M+Cl]-", "[M+CH3COO]-"] };

/** @typedef {{ id: string, nome: string, formula: string, massa?: number, smiles: string | null, addotti?: string[] }} Molecola */

/** The state of the tab; it lives as long as the page. */
export const G = {
  k: -1,
  /** @type {string} */ text: "",
  /** @type {Molecola | null} */ mol: null,
  adduct: "",
  smiles: "",
  /** Defaults are the `opzioni` of the first answer. */
  o: { ppm_formula: 10, mz_basso: 150, ppm_basso: 20, prove_min: 1, attivazione: "", isotopi: false },
  /** @type {{ mz: number, formula: string }[]} */ scelte: [],
  /** @type {any} */ j: null,
  /** @type {string | null} */ sel: null,
  /** @type {string | null} */ error: null,
  /** @type {any} */ errorObj: null,
  /** «Run MSn workflow» asked for the library search right after the next graph is drawn (read once). */
  auto: false,
  /** Show only the losses a fragmentation rule explains (the filter of the toolbar). */
  rf: false,
  /** The answer is for the request with this key. */
  key: "",
  /** The view: «tree» or «radial»; the filters of the toolbar (min % of the base peak, highest MS level shown, only the best path, ions whose subtree is closed, structures in the boxes). */
  /** @type {"tree" | "radial"} */ view: "tree",
  minRel: 0,
  maxLevel: 0,
  bestOnly: false,
  /** @type {Set<string>} */ collapsed: new Set(),
  structs: true,
  /** @type {string | null} The name of the graph file shown read-only (no raw data, no engine requests), else null. */
  ro: null,
  /** The 2D molfile of the SMILES of the tab (OpenChemLib, infusione-grafo-mol.js), sent back to the graph for the drawings; "" = none. */
  molfile: "",
};

/** Polarity of the file: 1 positive, -1 negative (the engine warns when the ion disagrees). */
export const polarityOf = (/** @type {any} */ f) => (f && f.polarity === "negative" ? -1 : 1);

/** The adducts offered for a polarity and the one a name starts with (the first adduct of the molecule that fits, else the first of the list). */
export function startAdduct(/** @type {1 | -1} */ pol, /** @type {Molecola | null} */ mol) {
  const list = ADDUCTS[pol];
  return (mol && mol.addotti && mol.addotti.find(a => list.includes(a))) || list[0];
}

/** The text of the ion the engine reads: what was typed when it has a charge or an adduct, else the formula and the chosen adduct. */
export function ionText() {
  const t = (G.mol ? G.mol.formula : G.text).trim();
  if (!t) return "";
  return G.mol || !/[+\-[]/.test(t) ? `${t} ${G.adduct}` : t;
}

/** The parameters of a graph request (the options of msngrafo/1 and those of the precursor tree), as plain values. @returns {Record<string, any>} */
export function params(/** @type {number} */ k) {
  const { o } = G, ion = ionText();
  /** @type {Record<string, any>} */
  const p = { k, tol: INF.O.tol, ce: INF.O.ce ? 1 : 0, ppm_formula: o.ppm_formula, mz_basso: o.mz_basso, ppm_basso: o.ppm_basso, prove_min: o.prove_min };
  if (ion) p.formula = ion;
  if (ion && G.smiles) p.smiles = G.smiles;
  if (o.attivazione) p.attivazione = o.attivazione;
  if (o.isotopi) p.isotopi = 1;
  if (G.scelte.length) p.scelte = G.scelte;
  return p;
}

/** The query string of the parameters (the choices as JSON). */
export function query(/** @type {number} */ k) {
  const q = new URLSearchParams();
  for (const [n, v] of Object.entries(params(k))) q.set(n, typeof v === "object" ? JSON.stringify(v) : String(v));
  return q.toString();
}

/** The URL of the graph for the state and the options of the precursor tree. */
export const graphUrl = (/** @type {number} */ k) => "api/msn_grafo?" + query(k);

/** The URL the tab draws from: the graph plus the interpretation of the losses and the fragmentation rules and, with a SMILES, the bond breaks of the structure (evidence for the arcs and the panel of a node; exports and the library do not carry them). */
const viewUrl = (/** @type {number} */ k, /** @type {boolean} */ withCuts = true) => graphUrl(k) + "&interpreta=1&regole=1" + (withCuts && G.smiles && ionText() ? "&rotture=1" : "") + (G.smiles && G.molfile ? "&molfile=" + encodeURIComponent(G.molfile) + (withCuts ? "&strutture=tutti" : "") : "");

/** Asks for the graph of the file; keeps the answer in G.j or the error in G.error. @returns {Promise<boolean>} false when a newer request took over. */
export async function fetchGraph(/** @type {number} */ k) {
  let url = viewUrl(k);
  G.key = url;
  try {
    /** @type {any} */ let j;
    try { j = await J(url); } catch (e) {
      // a SMILES the engine cannot read must not hide the graph: ask again without the cuts of every ion
      if (!/** @type {any} */ (e).key || !String(/** @type {any} */ (e).key).startsWith("err.msngrafo.badSmiles")) throw e;
      url = viewUrl(k, false); G.key = url; j = await J(url);
    }
    if (G.key !== url) return false;
    if (G.j == null) { const o = j.opzioni || {}; for (const n of /** @type {const} */ (["ppm_formula", "mz_basso", "ppm_basso", "prove_min"])) if (typeof o[n] === "number") G.o[n] = o[n]; }
    G.j = j; G.error = null; G.errorObj = null; G.k = k;
    if (G.sel && !j.nodi.some((/** @type {any} */ n) => n.id === G.sel)) G.sel = null;
  } catch (e) {
    if (G.key !== url) return false;
    G.error = /** @type {any} */ (e).message || String(e); G.errorObj = e;
  }
  return true;
}

/** @type {Map<string, any>} */
const cuts = new Map();

/**
 * The cleavages of the SMILES of the tab that explain a node: the value of `strutture_tutti` of the graph when it has it (no request), else the key `strutture` of the graph request plus `strutture=<id>`.
 * Null when there is no SMILES, no ion, the node is a root, the engine answers without the key or the request fails.
 */
export async function fetchStrutture(/** @type {any} */ n) {
  if (G.ro || !G.smiles || !G.j || !ionText() || n.livello < 2) return null;
  if (G.j.strutture_tutti) return G.j.strutture_tutti.find((/** @type {any} */ s) => s.nodo === n.id) || null;
  const url = graphUrl(G.k) + "&strutture=" + n.id;
  if (!cuts.has(url)) cuts.set(url, J(url).then((/** @type {any} */ j) => j.strutture || null).catch(() => null));
  return cuts.get(url);
}

/** The node of the answer by id. */
export const nodeOf = (/** @type {string | null} */ id) => (G.j && id ? G.j.nodi.find((/** @type {any} */ n) => n.id === id) || null : null);

/** The formula the engine chose or proposes for a node: the chosen one, else the only candidate (null when there are several). */
export function chosen(/** @type {any} */ n) {
  const f = n && n.formula;
  return f ? f.scelta || (f.candidati.length === 1 ? f.candidati[0].formula : null) : null;
}

/** Fixes a candidate of a node («Use»): a new request with the choice, found again by the engine in the node by its m/z. */
export function useCandidate(/** @type {any} */ n, /** @type {string} */ formula) {
  G.scelte = [...G.scelte.filter(s => Math.abs(s.mz - n.mz) > 1e-4), { mz: n.mz, formula }];
}
