// Two helpers of lr.js about the list of the files of a session.

/** The notebook lists the items of the session: the parts of a mixed file ("name.mzML#MS1") come back as the file, once. */
export function sessionFiles(items, ok) {
  const seen = new Set();
  return (Array.isArray(items) ? items : []).filter(x => x && x.file)
    .map(x => (String(x.file).includes("#") ? { ...x, file: String(x.file).split("#")[0], label: null } : x))
    .filter(x => ok(x.file) && !seen.has(x.file) && seen.add(x.file));
}

/** True when the names are of both resolutions (a list opened file by file is judged before its first file replaces the session). */
export async function mixesResolutions(rpc, names, ok) {
  const kinds = new Set();
  for (const n of names) if (ok(n)) kinds.add(JSON.parse((await rpc({ op: "lr_items", name: n, sample: {} })).info).some(it => it.hr));
  return kinds.size > 1;
}
