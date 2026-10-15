// @ts-strict
"use strict";
// Waters MassLynx .raw folders (docs/agenti/waters.md): a folder loaded with «Load a folder» (cartella-carica.js) that is called *.raw and holds _FUNC001.DAT is one run. No engine reads it, so it goes
// straight to the readers module `waters` (rule R of shell.js, docs/agenti/lettori-modulo.md): a File that stands for it in the list until the module has written its mzML. A Thermo .raw is a FILE and never matches.
// The pure parts (locate, group, mzmlName) have no DOM and are tested in node (tests/test_waters_raw.py).

const WatersRaw = (() => {
  const dirOf = (/** @type {File} */ f) => (typeof CartCarica !== "undefined" ? CartCarica.dir(f) : ((/** @type {any} */ (f)).webkitRelativePath || "").replace(/[^/]*$/, ""));

  // where a file lies in a MassLynx folder: { root: "a/x.raw/", name: "x.raw", rel: "_FUNC001.DAT" } for a file directly inside a folder called *.raw, else null
  function locate(/** @type {File} */ f) {
    const parts = (dirOf(f) + f.name).split("/"), i = parts.length - 2;
    return i >= 0 && /\.raw$/i.test(parts[i]) && parts[i].length > 4 ? { root: parts.slice(0, i + 1).join("/") + "/", name: parts[i], rel: parts[i + 1] } : null;
  }

  // the runs among the files (the *.raw folders with a _FUNC001.DAT: { name, root, files: [{ rel, file }] }, hidden files left out) and the other files; the files of a *.raw folder without _FUNC001.DAT pass
  function group(/** @type {File[]} */ files) {
    const found = new Map(), rest = /** @type {File[]} */ ([]);
    for (const f of files) {
      const loc = locate(f);
      if (!loc) { rest.push(f); continue; }
      const w = found.get(loc.root) || { name: loc.name, root: loc.root, files: [] };
      found.set(loc.root, w);
      w.files.push({ rel: loc.rel, file: f });
    }
    const runs = [];
    for (const w of found.values()) {
      if (w.files.some((/** @type {{ rel: string }} */ x) => x.rel.toLowerCase() === "_func001.dat")) runs.push({ ...w, files: w.files.filter((/** @type {{ rel: string }} */ x) => x.rel[0] !== ".") });
      else rest.push(...w.files.map((/** @type {{ file: File }} */ x) => x.file));
    }
    return { runs, rest };
  }

  // name of the mzML of a run: «x.raw» → «x.mzML»; two runs of the same name in different folders take the name of the folder above (as agilent-d.js does)
  function mzmlName(/** @type {{ name: string, root: string }} */ run, /** @type {string[]} */ taken) {
    const base = run.name.replace(/\.raw$/i, ""), above = run.root.split("/").filter(Boolean).slice(0, -1).pop() || "", seen = new Set(taken.map(n => n.toLowerCase()));
    let name = base + ".mzML";
    for (let i = 1; seen.has(name.toLowerCase()); i++) name = `${base} (${above || i}${above && i > 1 ? " " + i : ""}).mzML`;
    return name;
  }

  // page part: the Files that stand for the runs (members «x.raw/_FUNC001.DAT»…, read lazily by the module); a failure shows its notice in the box of the loading (#up)
  function toMzml(/** @type {ReturnType<typeof group>["runs"]} */ runs) {
    const M = window.LETTORI_MOD, box = /** @type {HTMLElement} */ (document.getElementById("up")), taken = (ST.files || []).map((/** @type {{ name: string }} */ x) => x.name), out = /** @type {File[]} */ ([]);
    for (const r of runs) {
      if (!M) { const d = document.createElement("div"); d.className = "fail"; d.textContent = r.name + ": " + I18N.t("err.file.wiffLoad"); box.appendChild(d); continue; }
      out.push(M.stand(mzmlName(r, [...taken, ...out.map(f => f.name)]), r.files.map((/** @type {{ rel: string, file: File }} */ x) => ({ name: r.name + "/" + x.rel, blob: x.file })), 0, "waters"));
    }
    return out;
  }

  return { locate, group, mzmlName, toMzml };
})();
if (typeof module !== "undefined") module.exports = WatersRaw;
