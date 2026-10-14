// @ts-strict
// Two helpers of lr.js about the list of the files of a session.

const BASE = { sample: "sample", t0: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", calibration: "standard", qc: "sample" };

/** The notebook lists the items of the session: the parts of a mixed file ("name.mzML#MS1") come back as the file, once. An entry of campioni/2 (notebook v2,
 *  `tipo`, `tempo_min`, ...) gives the fields the engine knows (type, time, concentration, replicate); the labels are the ones of `labels` (`ui.files`). */
export function sessionFiles(items, ok, labels = []) {
  const seen = new Set(), lab = new Map((Array.isArray(labels) ? labels : []).filter(x => x && x.file).map(x => [x.file, x.label]));
  return (Array.isArray(items) ? items : []).filter(x => x && x.file)
    .map(x => (x.tipo ? { file: x.file, label: lab.get(x.file) ?? null, type: BASE[x.tipo] || "sample", time: x.tempo_min, conc: x.concentrazione, cunit: x.unita, rep: x.rep } : x))
    .map(x => (String(x.file).includes("#") ? { ...x, file: String(x.file).split("#")[0], label: null } : x))
    .filter(x => ok(x.file) && !seen.has(x.file) && seen.add(x.file));
}

/** True when the names are of both resolutions (a list opened file by file is judged before its first file replaces the session). */
export async function mixesResolutions(rpc, names, ok) {
  const kinds = new Set();
  for (const n of names) if (ok(n)) kinds.add(JSON.parse((await rpc({ op: "lr_items", name: n, sample: {} })).info).some(it => it.hr));
  return kinds.size > 1;
}
