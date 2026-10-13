// @ts-strict
"use strict";
// Agilent MassHunter .D folders (docs/agenti/agilent.md): a folder loaded with «Load a folder» (cartella-carica.js) holds a run as many files; the ones the Rust engine reads
// (AcqData/MSScan.bin, MSPeak.bin, ...) are grouped by their .D folder here, converted to one mzML each in the engine (rust-agilent.js) and handed to upload() like any mzML.
// The pure parts (locate, group, mzmlName) have no DOM and are tested in node (tests/test_agilent_d.py).

const AgilentD = (() => {
  // the files of a run that the engine takes, by the name of its argument; scan and peak are required unless there is a data.ms (an older run)
  const NEED = { scan: "AcqData/MSScan.bin", peak: "AcqData/MSPeak.bin", xsd: "AcqData/MSScan.xsd", ts: "AcqData/MSTS.xml", devices: "AcqData/Devices.xml", contents: "AcqData/Contents.xml", tree: "AcqData/MSTree2.bin", acqmeth: "acqmeth.txt", datams: "data.ms" };
  const dirOf = f => (typeof CartCarica !== "undefined" ? CartCarica.dir(f) : (f.webkitRelativePath || "").replace(/[^/]*$/, ""));

  // where a file lies in a .D folder: { root: "a/b/x.D/", name: "x.D", rel: "AcqData/MSScan.bin" }, or null when no folder above it is called *.D
  function locate(f) {
    const parts = (dirOf(f) + f.name).split("/");
    const i = parts.findIndex((p, k) => k < parts.length - 1 && /\.d$/i.test(p) && p.length > 2);
    if (i < 0) return null;
    return { root: parts.slice(0, i + 1).join("/") + "/", name: parts[i], rel: parts.slice(i + 1).join("/") };
  }

  // the runs among the files, and the files that are not part of any .D folder. A run: { name, root, files: { scan, peak, ... }, missing: [names of required files not found; none when there is a data.ms] }
  function group(files) {
    const runs = new Map(), rest = [];
    for (const f of files) {
      const loc = locate(f);
      if (!loc) { rest.push(f); continue; }
      const r = runs.get(loc.root) || { name: loc.name, root: loc.root, files: {}, seen: new Set() };
      runs.set(loc.root, r);
      r.seen.add(loc.rel.toLowerCase());
      for (const [k, rel] of Object.entries(NEED)) if (rel.toLowerCase() === loc.rel.toLowerCase()) r.files[k] = f;
    }
    const list = [...runs.values()].map(r => ({ name: r.name, root: r.root, files: r.files, missing: r.files.datams ? [] : ["scan", "peak"].filter(k => !r.files[k]).map(k => NEED[k]) }));
    return { runs: list, rest };
  }

  // name of the mzML of a run: «x.D» → «x.mzML»; two runs of the same name in different folders take the name of the folder above (as cartella-carica.js does for equal names)
  function mzmlName(run, taken) {
    const base = run.name.replace(/\.d$/i, "");
    const above = run.root.split("/").filter(Boolean).slice(0, -1).pop() || "";
    const seen = new Set(taken.map(n => n.toLowerCase()));
    let name = base + ".mzML";
    for (let i = 1; seen.has(name.toLowerCase()); i++) name = `${base} (${above || i}${above && i > 1 ? " " + i : ""}).mzML`;
    return name;
  }

  // ---- page part: converts the runs with the engine into the box of the loading (#up); resolves to the Files of mzML (a run that fails shows its notice there)
  async function toMzml(runs) {
    const T = (k, p) => I18N.t(k, p), out = /** @type {{ files: File[], errs: any[] }} */ ({ files: [], errs: [] }), box = /** @type {HTMLElement} */ (document.getElementById("up"));
    if (!runs.length) return [];
    const rb = window.MZLAB_RUST_READY ? await window.MZLAB_RUST_READY : null;
    const taken = (ST.files || []).map(x => x.name);
    const ok = [];
    for (const r of runs) {
      if (r.missing.length) out.errs.push([T("err.file.notAgilentD", { name: r.name }), r.name]);
      else if (!rb || rb.disabled || !(rb.convertLettori || rb.convertAgilent)) out.errs.push([T("err.file.agilentNoEngine", { name: r.name }), r.name]);
      else ok.push(r);
    }
    const fails = () => out.errs.forEach(([msg, nm]) => { const d = document.createElement("div"); d.className = "fail"; d.textContent = nm + ": " + msg; box.appendChild(d); });
    if (!ok.length) { fails(); return []; }
    const g = ConvUI.group(box, ok.map(r => (r.files.peak || r.files.datams).size));
    let i = 0;
    for (const r of ok) {
      if (g.signal.aborted) break;
      const ui = g.start(++i, r.name);
      try {
        const name = mzmlName(r, [...taken, ...out.files.map(f => f.name)]);
        const blob = rb.convertLettori
          ? await rb.convertLettori(r.name, Object.entries(r.files).map(([k, f]) => ({ name: NEED[k], blob: f })), p => ui.update(p), g.signal)      // the registry of readers (rust-lettori.js)
          : await rb.convertAgilent(r.name, r.files, p => ui.update(p), g.signal);
        CONV.add(name, blob);
        out.files.push(new File([blob], name));
        ui.done();
        if (typeof renderFiles === "function") renderFiles();
      } catch (e) {
        if (e.cancelled) { ui.done(); continue; }
        ui.fail(e.json ? I18N.err(e.json).message : e.message);
      }
    }
    g.end();
    fails();
    return out.files;
  }

  return { NEED, locate, group, mzmlName, toMzml };
})();
if (typeof module !== "undefined") module.exports = AgilentD;
