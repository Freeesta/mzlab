// @ts-strict
// Warm-up of the pointed folder (docs/agenti/riscaldamento.md): once the folder is on screen, its mzML files are copied and indexed one at a time in the background, in the
// order of the list, so that opening one is a cache hit (lr.js asks the engine: «is it warm?»). Nothing opens, no row appears. It waits for a quiet page (no loading, no
// input for a moment), skips what is big, what the device cannot afford (phone, little memory, data saving, low battery) and stays silent on any error.
// Loaded by cartella-puntata.js on first use (never at start-up). The user's click calls claim(): that file leaves the queue, and work already running on it is awaited, not repeated.

/** @typedef {{ key: string, name: string, size?: number, get: () => Promise<File> }} Item */
const QUIET_MS = 1500, POLL_MS = 400, BETWEEN_MS = 250;
const limit = () => (window.MZLAB_DESKTOP ? 1024 : 128) * 1048576;       // the engine of the site reads the scan table in one go: past this size it would stall the other requests

/** @type {Item[]} */ let queue = [];
/** @type {{ key: string, done: Promise<void> } | null} */ let running = null;
/** @type {Map<string, string>} */ const finished = new Map();             // key -> "warmed" | "skipped" | "failed"
let gen = 0, lastInput = 0, pumping = false, battery = /** @type {any} */ (null), watching = false;

function watchInput() {
  if (watching) return; watching = true;
  const mark = () => { lastInput = performance.now(); };
  for (const t of ["pointerdown", "keydown", "wheel", "touchstart"]) addEventListener(t, mark, { capture: true, passive: true });
  try { const g = /** @type {any} */ (navigator).getBattery; if (g) g.call(navigator).then((/** @type {any} */ b) => { battery = b; }).catch(() => {}); } catch (_) { /* no battery API */ }
}
/** the device can afford background work */
function allowed() {
  const nav = /** @type {any} */ (navigator), c = nav.connection || {};
  if (window.QQQ_PHONE || c.saveData || (nav.deviceMemory && nav.deviceMemory < 4)) return false;
  if (battery && !battery.charging && battery.level < 0.25) return false;
  return true;
}
/** the page is quiet: visible, no loading under way, no input for a moment */
const quiet = () => !document.hidden && !(typeof UPBUSY !== "undefined" && UPBUSY > 0) && performance.now() - lastInput >= QUIET_MS;
const later = (/** @type {number} */ ms) => new Promise(r => setTimeout(r, ms));

async function warmOne(/** @type {Item} */ it) {
  let state = "skipped";
  try {
    const f = await it.get();
    if (f.size > 0 && f.size <= limit() && !(typeof ST !== "undefined" && ST.files.some((/** @type {{ name: string }} */ g) => g.name === f.name))) {
      const r = await fetch("api/warm?name=" + encodeURIComponent(f.name), { method: "POST", body: f });
      const j = await r.json().catch(() => ({}));
      state = j.warmed ? "warmed" : j.skipped === "failed" ? "failed" : "skipped";
      if (j.skipped === "room" || j.skipped === "loaded") state = "retry";
    }
  } catch (_) { state = "failed"; }
  return state;
}

async function pump() {
  if (pumping) return; pumping = true;
  const mine = gen;
  try {
    while (mine === gen && queue.length) {
      if (!allowed()) { queue = []; break; }
      if (!quiet()) { await later(POLL_MS); continue; }
      const it = /** @type {Item} */ (queue.shift());
      let release = () => {};
      running = { key: it.key, done: new Promise(r => { release = () => r(); }) };
      const state = await warmOne(it);
      running = null; release();
      if (state !== "retry") finished.set(it.key, state);
      await later(BETWEEN_MS);
    }
  } finally { pumping = false; running = null; }
}

export const RISCALDA = {
  /** the files to prepare, in the order of the list; replaces the queue (work under way finishes) */
  offer(/** @type {Item[]} */ items) {
    watchInput();
    if (!allowed()) { queue = []; return; }
    const count = new Map(); for (const it of items) count.set(it.name.toLowerCase(), (count.get(it.name.toLowerCase()) || 0) + 1);
    queue = items.filter(it => /\.mzml$/i.test(it.name) && count.get(it.name.toLowerCase()) === 1 && !finished.has(it.key) && !(running && running.key === it.key) && (it.size == null || (it.size > 0 && it.size <= limit())));
    pump();
  },
  /** the student opens `key`: it leaves the queue; resolves when no warm-up work on it is running (the normal open then finds it ready) */
  async claim(/** @type {string} */ key) {
    queue = queue.filter(it => it.key !== key);
    if (running && running.key === key) await running.done;
    finished.delete(key);
  },
  /** the folder is forgotten or changed: nothing more is started */
  stop() { gen++; queue = []; },
  /** for the tests */
  state: () => ({ queued: queue.map(i => i.key), running: running && running.key, finished: Object.fromEntries(finished) }),
};
/** @type {any} */ (window).RISCALDA = RISCALDA;
