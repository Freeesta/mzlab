"use strict";
// Why a file could not be read: ONE shape {codice, fase, causa, dettagli} for every loading error of every format. `fase` = where it failed (open / copy / index / decode / write),
// `causa` = a short key of the catalog (diag.cause.*), `dettagli` = what helps to find the reason without showing data: format, size, the original technical error, the first lines of the header
// (never the spectra), `trace` = how the loading went (CONVROWS.trace: phases, times, the last message of the worker). Paths and user names are cut out. The pure parts are run by node in tests/test_diagnosi.py. Classic script, loaded after conversione.js.

const DIAG = {
  FASI: ["open", "copy", "index", "decode", "write"],
  // personal parts out of a text: full paths of the disk (Windows, Mac, Linux) and the user name inside them
  pulisci(t, max = 400) {
    let s = String(t == null ? "" : t);
    s = s.replace(/[A-Za-z]:\\[^\s"'<>|]*/g, "<path>").replace(/\\\\[^\s"'<>|]+/g, "<path>").replace(/file:\/\/[^\s"'<>]*/g, "<path>")
      .replace(/(^|[\s"'=(])\/(?:Users|home|root|var|tmp|mnt|Volumes|private|opt|srv)\/[^\s"'<>)]*/g, "$1<path>");
    return s.length > max ? s.slice(0, max) + "…" : s;
  },
  // the cause (key diag.cause.*) from the technical text of the error; the phase can force it
  causa(raw, fase) {
    const r = String(raw || "");
    if (/quota|NS_ERROR_FILE_NO_DEVICE_SPACE|no space left|disk is full/i.test(r)) return "quota";
    if (/NotReadable|NotFoundError|NotAllowed|could not be read|permission|SecurityError|file.*(changed|moved|deleted)/i.test(r)) return "read";
    if (/out of memory|memory access|allocation|RangeError|Array buffer allocation|unreachable|memory budget|oom|indexCrash|memoryBudget/i.test(r)) return "memory";
    if (/xml|parse|parsing|unexpected|malformed|invalid|truncated|end of file|eof|not an? (mzml|mzxml)|zlib|inflate|base64|numpress|offset|checksum|index list/i.test(r)) return fase === "decode" ? "decode" : "format";
    return "unknown";
  },
  // the first lines of a text file up to the list of the spectra: the header says who wrote the file; nothing of the data
  testa(text) {
    const out = [];
    for (const line of String(text || "").split(/\r?\n/)) {
      if (/[\x00-\x08\x0e-\x1f]/.test(line)) break;                     // binary: not a header
      const i = line.search(/<(?:spectrumList|chromatogramList|binary|scan |Scan)/);
      out.push(this.pulisci(i >= 0 ? line.slice(0, i + 14) : line, 240));
      if (i >= 0 || out.length >= 25) break;
    }
    return out.join("\n");
  },
  // the diagnosis of an error `e` (Error from I18N.err: e.params = {fase, raw, ...}) for the file `info` = {name, size, format}
  da(e, info, head = "") {
    const p = (e && e.params) || {}, fase = this.FASI.includes(p.fase) ? p.fase : (e && e.fase) || "open";
    let raw = String(p.raw != null ? p.raw : (e && e.message) || "");
    const nm = String((info && info.name) || ""), stem = nm.replace(/\.[^.]*(\.scan)?$/i, "");
    for (const x of [nm, stem]) if (x.length > 2) raw = raw.split(x).join("<file>");      // the name of the file never goes in the text to copy (the messages of the engine carry it)
    const ext = (String((info && info.name) || "").match(/\.([A-Za-z0-9]+)(?:\.scan)?$/) || [, ""])[1].toLowerCase();
    return {
      codice: (e && e.key) || "err.engine.index", fase, causa: this.causa(raw, fase),
      dettagli: { format: (info && info.format) || ext || "?", size: (info && info.size) || 0, raw: this.pulisci(raw), head: head || "" },
    };
  },
  // The long tasks (> 50 ms) of the page, each with the phase the engine was in when it started: what blocks the page on a file we do not have. The observer starts with the page
  // (avvia) and costs nothing while the page is responsive; `marca` is called at every change of phase (Tabui: TPLIVE) and at the moments that matter (start, results drawn).
  LONG: {
    max: 200, list: /** @type {{t: number, dur: number, fase: string}[]} */ ([]), marks: /** @type {{t: number, fase: string}[]} */ ([]),
    marca(fase) { this.marks.push({ t: performance.now(), fase }); if (this.marks.length > 200) this.marks.shift(); },
    // the phase that was active at time `t` (ms of performance.now())
    faseA(t) { let f = ""; for (const m of this.marks) { if (m.t <= t) f = m.fase; else break; } return f; },
    registra(start, dur) { this.list.push({ t: Math.round(start), dur: Math.round(dur), fase: this.faseA(start) }); if (this.list.length > this.max) this.list.shift(); },
    avvia() { try { new PerformanceObserver(l => { for (const e of l.getEntries()) this.registra(e.startTime, e.duration); }).observe({ type: "longtask", buffered: true }); } catch (_) { /* no Long Tasks API: nothing is recorded */ } },
    da(since = 0) { return this.list.filter(x => x.t >= since); },
    // lines for the text to copy: how many, the longest, then the ten longest with their phase
    righe(since = 0) {
      const l = this.da(since); if (!l.length) return [];
      const top = l.slice().sort((a, b) => b.dur - a.dur).slice(0, 10);
      return [`long tasks: ${l.length}, longest ${top[0].dur} ms, total ${l.reduce((a, x) => a + x.dur, 0)} ms`, ...top.map(x => `  +${x.t} ms: ${x.dur} ms in ${x.fase || "(no phase)"}`)];
    },
  },
  // the text to copy and send: nothing personal (no file name, no path)
  testo(d, version) {
    const x = d.dettagli, L = [`mzLab ${version || "?"}`, `code: ${d.codice}`, `phase: ${d.fase}`, `cause: ${d.causa}`, `format: ${x.format}`, `size: ${x.size} bytes`, `error: ${x.raw}`];
    if (x.head) L.push("header:", x.head);
    if (x.trace) L.push("loading:", x.trace);
    L.push(...this.LONG.righe());
    return L.join("\n");
  },
  // the same for a page that was slow without any error (the Dig of Tabui): version, cores, long tasks since `since`
  testoLento(version, since = 0) {
    return [`mzLab ${version || "?"}`, `cores: ${typeof navigator !== "undefined" ? navigator.hardwareConcurrency || "?" : "?"}`, ...this.LONG.righe(since)].join("\n");
  },
  // the first bytes of a File read for the header (a binary maker's file gives no lines)
  async leggiTesta(f) { try { return this.testa(await f.slice(0, 8192).text()); } catch (_) { return ""; } },
  // the block «Details» of the notice: a button (aria-expanded, closed) and the hidden box with «Copy diagnosis»
  html(d, esc, t) {
    const x = d.dettagli, row = (k, v) => `<dt>${t(`diag.k.${k}`)}</dt><dd>${esc(v)}</dd>`;
    return `<div class="diag"><button type="button" class="sec" data-a="diagdet" aria-expanded="false">${t("diag.details")}</button>`
      + `<div class="diagbox" hidden><p>${t(`diag.cause.${d.causa}`)}</p><dl>${row("phase", t(`diag.phase.${d.fase}`))}${row("format", x.format)}${row("size", x.size + " B")}${row("error", x.raw)}</dl>`
      + (x.head ? `<pre>${esc(x.head)}</pre>` : "") + (x.trace ? `<pre>${esc(x.trace)}</pre>` : "") + `<button type="button" class="sec" data-a="diagcopy">${t("diag.copy")}</button></div></div>`;
  },
  // one diagnosis shown for a group of files of the notice: the first one; `d` can be a function that gives it when it is copied (a loading still running)
  cabla(root, d, version, t) {
    const b = root.querySelector('[data-a=diagdet]'), box = root.querySelector(".diagbox"), c = root.querySelector('[data-a=diagcopy]');
    if (b) b.onclick = () => { const o = b.getAttribute("aria-expanded") === "true"; b.setAttribute("aria-expanded", String(!o)); box.hidden = o; };
    if (c) c.onclick = async () => { try { await navigator.clipboard.writeText(DIAG.testo(typeof d === "function" ? d() : d, version)); c.textContent = t("diag.copied"); } catch (_) { c.textContent = t("diag.copyFail"); } };
  },
};

if (typeof PerformanceObserver !== "undefined" && typeof document !== "undefined") DIAG.LONG.avvia();
if (typeof module !== "undefined") module.exports = DIAG;
