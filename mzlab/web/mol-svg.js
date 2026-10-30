// @ts-check
// 2D drawing of a structure from its SMILES: OpenChemLib (vendor, loaded on first use, once for every caller) and the theme-aware SVG of MOLUI.
// Used by the «What is it?» window (libreria-finestra.js) and the «Fragmentation graph» (infusione-grafo-mol.js).

/** @type {any} */ let OCL = null;
/** @type {Promise<any> | null} */ let loading = null;

/** OpenChemLib, once; null when the file cannot be loaded (no drawings then). */
export function ocl() {
  if (OCL) return Promise.resolve(OCL);
  if (!loading) loading = import("./vendor/openchemlib.js").then(m => (OCL = m.default || m), () => { loading = null; return null; });
  return loading;
}

/** The SVG text of the SMILES `smi` (`w`×`h`, element ids prefixed by `id`), themed by MOLUI; null when it cannot be read or OpenChemLib is missing. */
export async function smilesSvg(/** @type {string} */ smi, /** @type {number} */ w, /** @type {number} */ h, /** @type {string} */ id) {
  const o = smi ? await ocl() : null;
  if (!o) return null;
  try {
    const m = o.Molecule.fromSmiles(smi); m.inventCoordinates();
    const w_ = /** @type {any} */ (window), raw = m.toSVG(w, h, id, w_.MOLUI ? w_.MOLUI.SVG_OPTS : { suppressChiralText: true, suppressCIPParity: true, suppressESR: true });
    return w_.MOLUI ? w_.MOLUI.svg(raw) : raw;
  } catch (_) { return null; }
}
