// @ts-check
// Files open as soon as they are read: upload0 (shell.js) calls OPENNOW.add(name) when a file is in the loading table. The first file of a session starts it (api/explore),
// the others are appended (api/explore append) in order of arrival, with one bootSession at most every 200 ms; a new file starts hidden (bootSession0), so the work area shows
// the sample table until the first click. The world of the session (low or high resolution) is the world of the first file read: the other world stays unticked and #hrmix
// offers the switch. Also the write-through of the sample table to the open files (roles, times, concentrations: CAMPIONI.apply). Used by shell.js and ruoli-carica.js.
"use strict";

// nothing waits in E.pending any more (the progress rows are UPROWS): the waits of the page and of the tests read an empty list
E.pending = [];
const OPENNOW = /** @type {any} */ ((() => {
  /** @type {string[]} */ const q = [];
  const held = new Set();      // «Choose» of a large folder: these files stay unticked until the student ticks them
  const sent = new Set();      // files whose roles and times went to the open files once
  const tally = { n: 0, m: 0 };      // files opened and set aside since the last note of the side list
  let busy = 0, last = 0, late = false, cur = "";      // cur = the file being opened now
  /** @type {(() => void)[]} */ let calm = [];
  const src = (/** @type {string} */ n) => String(n).split("#")[0];
  const openOf = (/** @type {string} */ name) => E.files.filter((/** @type {FileAperto} */ x) => !x.gone && src(x.file) === name);
  const isOpen = (/** @type {string} */ name) => openOf(name).length > 0;
  const rowOf = (/** @type {string} */ name) => ST.files.find((/** @type {FileRiga} */ f) => f.name === name);
  /** the world of the open files: true = high resolution, null = no file open */
  const world = () => { const r = ST.files.find((/** @type {FileRiga} */ f) => f.kind in GORD && isOpen(f.name)); return r ? !!r.hr : null; };
  // the time of a dark is negative whatever way it got here (name, table, old project)
  const sample = (/** @type {FileRiga} */ f) => { if (RUOLI.roleOf(f) === "dark") f.time = darkTime(f.time); return { file: f.name, time: f.time, type: f.type, conc: isCal(f) ? f.conc : null, cunit: CU[ST.unit][0], label: null }; };
  /** the row of the table as an entry of CAMPIONI.apply */
  function entry(/** @type {FileRiga} */ f) {
    const r = RUOLI.roleOf(f), e = /** @type {any} */ ({ file: f.name, role: r });
    if (RUOLI.TIMED.has(r)) { e.time = f.time ?? null; e.tconf = true; }
    if (isCal(f)) { e.conc = f.conc ?? null; e.cunit = CU[ST.unit][0]; }
    return e;
  }
  async function boot() {
    last = Date.now(); late = false;
    await bootSession();
    const fresh = ST.files.filter((/** @type {FileRiga} */ f) => !sent.has(f.name) && isOpen(f.name));
    fresh.forEach((/** @type {FileRiga} */ f) => sent.add(f.name));
    if (fresh.length) RUOLI.commit(fresh);
    applyView(); FILETAB.soon(); uiSave();
  }
  // one file at a time, in the order the files were read
  async function drain() {
    if (busy) return;
    busy = 1;
    try {
      while (q.length) {
        const name = /** @type {string} */ (q.shift()), f = rowOf(name);
        if (!f || !(f.kind in GORD) || isOpen(name)) continue;
        if (held.has(name)) { f.use = false; continue; }
        const w = world();
        if (w !== null && !!f.hr !== w) { f.use = false; tally.m++; continue; }
        f.use = true; cur = name;
        try {
          if (w === null) { await post("api/explore", { samples: [sample(f)] }); await boot(); }
          else { await post("api/explore", { samples: [sample(f)], append: true }); if (Date.now() - last >= 200 || !q.length) await boot(); else late = true; }
          tally.n++;
        } catch (e_) { f.use = false; fail(e_); } finally { cur = ""; }
      }
      if (late) await boot();
    } finally { busy = 0; FILETAB.soon(); calm.splice(0).forEach(ok => ok()); }
  }
  /** the file `name` (a row of ST.files) is read: open it, unless the student set it aside */
  function add(/** @type {string} */ name) { q.push(name); drain(); }
  /** a large folder answered «Choose», or × on a progress row: these files come in (or stay) unticked; a file already being opened still opens */
  function hold(/** @type {string[]} */ names) { names.forEach(n => { held.add(n); const f = rowOf(n); if (f && n !== cur && !isOpen(n)) f.use = false; }); }
  /** resolves when nothing is waiting to be opened */
  const idle = () => busy || q.length ? new Promise(ok => calm.push(() => ok(undefined))) : Promise.resolve();
  /** the tick of the sample table: unticked = removed from the session (undo as usual), ticked = opened */
  function tick(/** @type {FileRiga} */ f, /** @type {boolean} */ on) {
    held.delete(f.name);
    if (!on) { f.use = false; openOf(f.name).forEach((/** @type {FileAperto} */ g) => removeFile(g)); return; }
    const w = world();
    if (w !== null && !!f.hr !== w) { f.use = false; const b = /** @type {HTMLElement | null} */ (document.querySelector("#hrmix button:not([hidden])")); if (b) b.focus(); return; }
    f.use = true;
    const gone = E.files.filter((/** @type {FileAperto} */ g) => g.gone && src(g.file) === f.name);
    if (gone.length) { gone.forEach((/** @type {FileAperto} */ g) => fileRemoval(g).back()); return; }
    add(f.name);
  }
  /** a student edit of the sample table on an open file: one undoable step (the writer of Sample metadata) */
  function edit(/** @type {FileRiga} */ f) { if (isOpen(f.name)) campioniEdit([entry(f)]); }
  /** proposals that landed after the files opened: quietly, for the cells the student did not touch */
  function sync() {
    const fs = ST.files.filter((/** @type {FileRiga} */ f) => f.use && !f.ed && (f.gR || f.gT) && isOpen(f.name));
    if (fs.length) CAMPIONI.apply(fs.map(entry), true);
  }
  /** «Switch to …»: closes the files of the open world and opens the other one; one «Removed · Undo» entry gives the old world back */
  async function toWorld(/** @type {boolean} */ hr) {
    const go = async (/** @type {boolean} */ to) => {
      loading(true);
      try {
        await post("api/new", {});      // closes the open files, the uploaded ones stay
        sent.clear();
        const fs = ST.files.filter((/** @type {FileRiga} */ f) => f.kind in GORD);
        fs.forEach((/** @type {FileRiga} */ f) => { f.use = !!f.hr === to; });
        await bootSession();
        fs.filter((/** @type {FileRiga} */ f) => f.use).forEach((/** @type {FileRiga} */ f) => q.push(f.name));
        await drain();
      } catch (e_) { fail(e_); } finally { loading(false); renderFiles(); }
    };
    const n = E.files.filter((/** @type {FileAperto} */ g) => !g.gone).length;
    await go(hr);
    if (typeof STORIA !== "undefined" && STORIA) STORIA.tolto({ what: I18N.t("files.removeAll.what", { n }), dati: String(n), undo: () => go(!hr), redo: () => go(hr) });
  }
  /** a batch has ended: the side list says what was added and what was set aside (only with files already shown: else the sample table says it) */
  async function note() {
    await idle();
    const { n, m } = tally; tally.n = tally.m = 0;
    if (!E.files.some((/** @type {FileAperto} */ f) => f.vis && !f.gone)) return;
    if (n || m) directNote(m ? "files.drop.mixed" : "files.drop.done", { n, m });
  }
  /** a folder of more than 50 data files asks once: true = open all, false = choose (all unticked) */
  function many(/** @type {number} */ n) {
    return new Promise(res => {
      const d = /** @type {HTMLDialogElement} */ (document.querySelector("#askdlg")), ok = /** @type {HTMLElement} */ (document.querySelector("#askok")), no = /** @type {HTMLElement} */ (document.querySelector("#askno")), inp = /** @type {HTMLElement} */ (document.querySelector("#askin"));
      const was = [ok.textContent, no.textContent];
      /** @type {HTMLElement} */ (document.querySelector("#asktxt")).textContent = I18N.t("load.many.text", { n });
      inp.hidden = true; no.hidden = false; ok.textContent = I18N.t("load.many.all"); no.textContent = I18N.t("load.many.choose");
      const done = (/** @type {boolean} */ v) => { d.close(); inp.hidden = false; [ok.textContent, no.textContent] = was; ok.onclick = no.onclick = null; d.oncancel = null; res(v); };
      ok.onclick = () => done(true); no.onclick = () => done(false); d.oncancel = () => done(false);
      d.showModal(); ok.focus();
    });
  }
  return { add, hold, idle, tick, edit, sync, toWorld, note, many, isOpen, world, busy: () => busy + q.length };
})());
