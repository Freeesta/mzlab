// @ts-check
// Memory budget of the Rust engine as the page shows and keeps it: the device facts the engine's rule (budget_choice in Rust) needs, the choice made by
// hand (localStorage), the audit log of every choice (what, why, with which numbers, who) and the row of the settings. The rule itself runs in Rust.

const KEY_MANUAL = "mzlab.memBudget";
const KEY_LOG = "mzlab.memBudget.log";
const LOG_MAX = 40;
/** The budgets offered by hand; "" = automatic. The engine keeps any number inside the limits of its build. */
const MANUAL_MB = [96, 128, 192];

/** @param {string} k @returns {string | null} */
function read(k) { try { return localStorage.getItem(k); } catch (_) { return null; } }
/** @param {string} k @param {string | null} v */
function write(k, v) { try { if (v === null) localStorage.removeItem(k); else localStorage.setItem(k, v); } catch (_) { /* no storage: the choice lasts until the page closes */ } }

/** What the browser says of the device; a number it does not give is 0. */
export function device() {
  const n = /** @type {any} */ (navigator);
  const small = !!(/** @type {any} */ (window).MZLAB_PHONE) || /iPad|iPhone|Android/i.test(n.userAgent || "") || (n.platform === "MacIntel" && n.maxTouchPoints > 1);
  return { device_gb: Number(n.deviceMemory) || 0, cores: Number(n.hardwareConcurrency) || 0, small };
}

/** The budget chosen by hand in MB, 0 = automatic. */
export function manualMb() { const v = Number(read(KEY_MANUAL)); return Number.isFinite(v) && v > 0 ? v : 0; }

/** @returns {Array<{ t: number, by: string, mode: string, mb: number, reason: string, confidence: string, inputs: any }>} */
export function readLog() { try { const a = JSON.parse(read(KEY_LOG) || "[]"); return Array.isArray(a) ? a : []; } catch (_) { return []; } }

/** Writes a choice in the audit log unless it is the same as the last one. `by`: auto (the program), user (the settings), supervisor (the engine ran out of memory). */
export function record(/** @type {any} */ choice, /** @type {string} */ by) {
  if (!choice) return;
  const log = readLog(), last = log[log.length - 1];
  if (last && last.mb === choice.mb && last.mode === choice.mode && last.reason === choice.reason) return;
  log.push({ t: Date.now(), by, mode: choice.mode, mb: choice.mb, reason: choice.reason, confidence: choice.confidence, inputs: choice.inputs || {} });
  write(KEY_LOG, JSON.stringify(log.slice(-LOG_MAX)));
}

/** The message that tells a worker what to decide from: the device and the budget chosen by hand. */
export function policy() { return { ...device(), manual: manualMb() }; }

const esc = (/** @type {any} */ s) => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));

/**
 * The settings row. `bridge` is the Rust bridge (window.MZLAB_RUST): `choice` is what the engine reported, `setBudget(mb)` applies a new manual choice (0 = automatic).
 * @returns {{ html: string, bind: (d: ParentNode) => void } | null} null when the page has no engine to speak of (no bridge or no answer yet)
 */
export function row(/** @type {any} */ bridge) {
  const I = /** @type {any} */ (window).I18N, c = bridge && bridge.choice;
  if (!I || !c) return null;
  const t = (/** @type {string} */ k, /** @type {any} */ p = undefined) => I.t(k, p);
  const opt = (/** @type {string} */ v, /** @type {string} */ label, /** @type {boolean} */ on) => `<option value="${v}"${on ? " selected" : ""}>${label}</option>`;
  const opts = opt("", t("settings.mem.auto"), c.mode !== "manual") + MANUAL_MB.map(mb => opt(String(mb), `${mb} MB`, c.mode === "manual" && c.mb === mb)).join("");
  const i = c.inputs || {};
  const statusOf = (/** @type {any} */ x) => t("settings.mem.status", { mb: x.mb, scans: Math.round(x.max_scans / 1000) * 1000, reason: t(`settings.mem.reason.${x.reason}`), conf: t(`settings.mem.conf.${x.confidence}`) });
  const facts = t("settings.mem.inputs", { gb: i.device_gb || "?", cores: i.cores || "?", hard: c.hard_mb });
  const log = readLog().slice().reverse().map(e => `<li>${esc(new Date(e.t).toLocaleString(I.lang))} · ${t(`settings.mem.by.${e.by}`)} · ${e.mb} MB · ${t(`settings.mem.reason.${e.reason}`)}</li>`).join("");
  const html = `<div class="uip-mem" style="margin:8px 0;padding-top:8px;border-top:1px solid var(--line)">
    <div class="row" title="${esc(t("settings.mem.title"))}"><label for="uip-mem-sel">${t("settings.mem.label")}</label> <select id="uip-mem-sel" aria-describedby="uip-mem-st">${opts}</select></div>
    <div class="sm" id="uip-mem-st" role="status" aria-live="polite">${esc(statusOf(c))}</div>
    <details class="uip-sub"><summary>${t("settings.mem.log")}</summary><div class="sm">${esc(facts)}</div><ul class="sm" style="margin:4px 0 0 16px;padding:0">${log}</ul></details>
  </div>`;
  return {
    html,
    bind(d) {
      const sel = /** @type {HTMLSelectElement | null} */ (d.querySelector("#uip-mem-sel"));
      const st = d.querySelector("#uip-mem-st");
      if (sel) sel.onchange = () => { bridge.setBudget(Number(sel.value) || 0); };
      bridge.onChoice = (/** @type {any} */ x) => { if (st && st.isConnected) st.textContent = statusOf(x); };
    },
  };
}

/** Keeps the manual choice (0 = automatic) and writes it in the log; the bridge then tells the engine. */
export function setManual(/** @type {number} */ mb) { write(KEY_MANUAL, mb > 0 ? String(mb) : null); }
