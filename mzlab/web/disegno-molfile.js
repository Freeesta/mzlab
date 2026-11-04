// @ts-check
// V2000 molfile of some atoms of Ketcher's struct, for Indigo (SMILES of the selection) and the Rust graph (fragments, partial structures), in
// Ketcher's atom order. Isotopes (M  ISO), radicals (M  RAD), wedge/hash bonds and atom parity are kept, so the SMILES of a selection has the same
// [13C] and @/@@ as the one of the whole canvas; chiral flag 1 when every stereo centre of the piece is absolute.
/**
 * @param {any} st Ketcher's struct @param {number[]} ids atoms of the piece @param {Set<number> | undefined} noCharge atoms written without their charge
 * @param {(label: string) => boolean} known false for R groups, abbreviations, atom lists... (no structure: null)
 * @returns {string | null}
 */
export function molfileOf(st, ids, noCharge, known) {
  const set = new Set(ids), idx = new Map(ids.map((id, n) => [id, n + 1])), atoms = ids.map(i => st.atoms.get(i));
  if (atoms.some(a => !known(a.label))) return null;
  const bonds = /** @type {any[]} */ ([]); st.bonds.forEach((/** @type {any} */ b) => { if (set.has(b.begin) && set.has(b.end)) bonds.push(b); });
  const f = (/** @type {number} */ v, /** @type {number} */ w) => v.toFixed(4).padStart(w), n3 = (/** @type {number} */ v) => String(v).padStart(3);
  const chg = /** @type {[number, number][]} */ ([]), iso = /** @type {[number, number][]} */ ([]), rad = /** @type {[number, number][]} */ ([]);
  const stereo = atoms.filter(a => a.stereoLabel), chiral = stereo.length && stereo.every(a => String(a.stereoLabel).startsWith("abs")) ? 1 : 0;
  const L = ["", "  qqq", "", `${n3(atoms.length)}${n3(bonds.length)}  0  0${n3(chiral)}  0  0  0  0  0999 V2000`];
  atoms.forEach((a, /** @type {number} */ k) => {
    const par = a.stereoParity >= 1 && a.stereoParity <= 3 ? a.stereoParity : 0;
    L.push(`${f(a.pp.x, 10)}${f(-a.pp.y, 10)}${f(0, 10)} ${a.label.padEnd(3)} 0  0${n3(par)}  0  0  0  0  0  0  0  0  0`);
    if (a.charge && !(noCharge && noCharge.has(ids[k]))) chg.push([k + 1, a.charge]);
    if (a.isotope > 0) iso.push([k + 1, a.isotope]);
    if (a.radical >= 1 && a.radical <= 3) rad.push([k + 1, a.radical]);
  });
  const bst = (/** @type {any} */ b) => [1, 4, 6].includes(b.stereo) && b.type === 1 ? b.stereo : b.stereo === 3 && b.type === 2 ? 3 : 0;   // wedge, either, hash; crossed double bond
  bonds.forEach(b => L.push(`${n3(/** @type {number} */ (idx.get(b.begin)))}${n3(/** @type {number} */ (idx.get(b.end)))}${n3(b.type >= 1 && b.type <= 4 ? b.type : 1)}${n3(bst(b))}  0  0  0`));
  const prop = (/** @type {string} */ tag, /** @type {[number, number][]} */ v) => { for (let i = 0; i < v.length; i += 8) L.push(`M  ${tag}` + n3(Math.min(8, v.length - i)) + v.slice(i, i + 8).map(([a, c]) => " " + n3(a) + " " + n3(c)).join("")); };
  prop("CHG", chg); prop("ISO", iso); prop("RAD", rad);
  L.push("M  END");
  return L.join("\n");
}
