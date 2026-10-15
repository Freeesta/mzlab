// @ts-strict
"use strict";
// Agilent MassHunter .D folders (docs/agenti/agilent.md): a folder loaded with «Load a folder» (cartella-carica.js) holds a run as many files; the ones the Rust engine reads
// (AcqData/MSScan.bin, MSPeak.bin, ...) are grouped by their .D folder here, read as they are by the engine (rust-nativo.js) and handed to upload() as one sample each.
// The other files directly under AcqData (module traces, DAD data...) go along under the key «x:» + path: the readers module of the LC/MS variants keeps them as size and SHA-1 (docs/agenti/agilent-lcms.md).
// The pure parts (locate, group, mzmlName) have no DOM and are tested in node (tests/test_agilent_d.py).

// A Bruker timsTOF `.d` folder shares the extension with Agilent's: its members tell it (analysis.tdf/.tsf with their bin, or only a BAF). The readers module `bruker` reads it; the engine does not.
const BrukerD = {
  BRUKER: { tdf: "analysis.tdf", tdfbin: "analysis.tdf_bin", tdfwal: "analysis.tdf-wal", tsf: "analysis.tsf", tsfbin: "analysis.tsf_bin", tsfwal: "analysis.tsf-wal", baf: "analysis.baf" },
  is: (/** @type {any} */ r) => !!(r.files.tdf || r.files.tsf || r.files.baf),
  /** the bin file of a database that has none */
  missing: (/** @type {any} */ r) => [["tdf", "tdfbin"], ["tsf", "tsfbin"]].filter(([a, b]) => r.files[a] && !r.files[b]).map(([, b]) => BrukerD.BRUKER[b]),
  /** a Bruker run: an error with a key, or a sample the module writes as mzML (shell.js moduloToMzml) */
  toModulo(/** @type {any} */ r, /** @type {any} */ out, /** @type {any} */ T, /** @type {string} */ nm) {
    const M = window.LETTORI_MOD, mem = Object.entries(r.files).filter(([n]) => n !== "baf" && BrukerD.BRUKER[n]).map(([n, f]) => ({ name: r.name + "/" + BrukerD.BRUKER[n], blob: f }));
    if (r.missing.length) out.errs.push([T("err.file.brukerNoBin", { name: r.name, file: r.missing[0] }), r.name]);
    else if (!mem.length) out.errs.push([T("err.file.brukerBaf"), r.name]);
    else if (!M) out.errs.push([T("err.file.notAccepted", { name: r.name }), r.name]);
    else out.files.push(M.stand(nm, mem, 0, "bruker"));
  },
};

const AgilentD = (() => {
  // the files of a run that the engine takes, by the name of its argument; scan and peak are required unless there is a data.ms (an older run)
  const NEED = { scan: "AcqData/MSScan.bin", peak: "AcqData/MSPeak.bin", xsd: "AcqData/MSScan.xsd", ts: "AcqData/MSTS.xml", devices: "AcqData/Devices.xml", contents: "AcqData/Contents.xml", tree: "AcqData/MSTree2.bin", acqmeth: "acqmeth.txt", datams: "data.ms", profile: "AcqData/MSProfile.bin", masscal: "AcqData/MSMassCal.bin", defcal: "AcqData/DefaultMassCal.xml", acqxml: "AcqData/AcqMethod.xml", sampleinfo: "AcqData/sample_info.xml", sequence: "AcqData/sequence.xml" };
  const OTHER = /^AcqData\/[^/]+$/i, relOf = /** @type {(k: string) => string} */ k => /** @type {any} */ (NEED)[k] || k.slice(2), dirOf = f => (typeof CartCarica !== "undefined" ? CartCarica.dir(f) : (f.webkitRelativePath || "").replace(/[^/]*$/, ""));

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
      const hit = Object.entries({ ...NEED, ...BrukerD.BRUKER }).find(([, rel]) => rel.toLowerCase() === loc.rel.toLowerCase()); if (hit) r.files[hit[0]] = f; else if (OTHER.test(loc.rel) && f.name[0] !== ".") r.files["x:" + loc.rel] = f;
    }
    const list = [...runs.values()].map(r => ({ name: r.name, root: r.root, files: r.files, bruker: BrukerD.is(r), missing: BrukerD.is(r) ? BrukerD.missing(r) : r.files.datams ? [] : ["scan", "peak"].filter(k => !r.files[k] && !(k === "peak" && r.files.profile)).map(k => NEED[k]) }));
    return { runs: list, rest };
  }

  // method folders *.m outside any .D run (the acquisition method of a GC/MS, e.g. «Geosmin and MIB method for scan.m»): their files are not data and not «ignored»; they are held
  // here with their folder name, so the Method section can read them. { methods: [{ name, root, files: [File] }], rest }. Hidden files inside (.DS_Store) are dropped.
  const held = /** @type {{ name: string, root: string, files: File[] }[]} */ ([]);
  function groupMethods(files) {
    const found = new Map(), rest = [];
    for (const f of files) {
      const parts = (dirOf(f) + f.name).split("/"), i = parts.findIndex((p, k) => k < parts.length - 1 && /\.m$/i.test(p) && p.length > 2);
      if (i < 0) { rest.push(f); continue; }
      const root = parts.slice(0, i + 1).join("/") + "/", m = found.get(root) || { name: parts[i], root, files: [] };
      found.set(root, m);
      if (f.name[0] !== ".") m.files.push(f);
    }
    return { methods: [...found.values()], rest };
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
    for (const r of runs) {
      if (r.bruker) BrukerD.toModulo(r, out, T, mzmlName(r, [...taken, ...out.files.map(f => f.name)]));
      else if (r.missing.length) out.errs.push([T("err.file.notAgilentD", { name: r.name }), r.name]);
      else if (!rb || rb.disabled || !rb.nativeSamples) out.errs.push([T("err.file.agilentNoEngine", { name: r.name }), r.name]);
      else {
        // the folder is read as it is: its files are the members, a run is one sample
        const mem = Object.entries(r.files).map(([k, f]) => ({ name: r.name + "/" + relOf(k), blob: f }));
        try {
          const nm = mzmlName(r, [...taken, ...out.files.map(f => f.name)]);
          const s = (await rb.nativeSamples(mem)).samples;
          out.files.push(NATIVO.sample(nm, NATIVO.key(r.name), mem, 0, s.length ? s[0].id : 1, false));
        } catch (e) { const M = window.LETTORI_MOD, fam = M && e.json ? M.retriable(e.json.error || e.json.error_key) : ""; if (fam) out.files.push(M.stand(mzmlName(r, [...taken, ...out.files.map(f => f.name)]), mem, 0, fam)); else out.errs.push([e.json ? I18N.err(e.json).message : e.message, r.name]); }   // a variant the engine refuses: the readers module tries it (shell.js moduloToMzml)
      }
    }
    out.errs.forEach(([msg, nm]) => { const d = document.createElement("div"); d.className = "fail"; d.textContent = nm + ": " + msg; box.appendChild(d); });
    return out.files;
  }

  return { NEED, locate, group, groupMethods, held, mzmlName, toMzml };
})();
if (typeof module !== "undefined") module.exports = AgilentD;
