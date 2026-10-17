// @ts-check
// One progress panel for the heavy jobs of the page that are not the loading of data files (those have CONVROWS and PROGRESSO): the example files, the library packages.
// It says the job, the item («2 of 5: name»), a bar with the true percentage when it is known (no aria-valuenow when it is not), the time elapsed and, once it is reliable,
// the time left; it is never still for more than a second (the clock), it adds «Still working…» after 2 s without news and advice after 30 s. At the end a short summary
// («4 of 5 done, 1 failed · 0:12») with the errors and «Details» / «Copy diagnosis»; it goes away by itself only when nothing failed.
// ES module, imported when a job starts (nothing at the start of the page).

const BEAT = 2000, STALL = 30000, KEEP = 8000;

/** @param {number} ms */
export function tempo(ms) { const s = Math.max(0, Math.round(ms / 1000)); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`; }

/** the time left from the fraction done, or "" while it is not reliable (under 5 % or 3 s, or the fraction is unknown)
 * @param {number | null} frac @param {number} ms elapsed */
export function rimanente(frac, ms) { return frac == null || frac < 0.05 || frac >= 1 || ms < 3000 ? "" : tempo(ms * (1 - frac) / frac); }

/**
 * @param {HTMLElement} box where the panel goes
 * @param {{ title: string, n: number, cancel?: boolean, signal?: AbortController }} o `cancel: false` when the caller has its own «Cancel»; `signal` is aborted by «Cancel»
 */
export function avanza(box, o) {
  const w = /** @type {any} */ (globalThis), T = (/** @type {string} */ k, /** @type {any} */ p = undefined) => w.I18N.t(k, p), esc = (/** @type {any} */ s) => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] || c);
  const ctl = o.signal || new AbortController(), t0 = Date.now(), el = document.createElement("div");
  el.className = "convprog avanza"; el.setAttribute("role", "group"); el.setAttribute("aria-label", o.title);
  el.innerHTML = `<div class="cp-head"><b data-r="title"></b> <span data-r="item"></span> <button type="button" class="sec sm" data-r="cancel"></button></div>
    <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-r="bar"><i></i></div>
    <div class="muted sm cp-num"><span data-r="detail"></span> <span data-r="time"></span> <span class="cp-beat" data-r="beat" role="status" aria-live="polite"></span></div>`;
  const q = (/** @type {string} */ s) => /** @type {HTMLElement} */ (el.querySelector(`[data-r=${s}]`)), bar = q("bar"), fill = /** @type {HTMLElement} */ (bar.querySelector("i"));
  q("title").textContent = o.title; bar.setAttribute("aria-label", o.title);
  const cn = /** @type {HTMLButtonElement} */ (q("cancel"));
  if (o.cancel === false) cn.hidden = true; else { cn.textContent = T("av.cancel"); cn.title = cn.ariaLabel = T("av.cancel.title"); cn.onclick = () => { cn.disabled = true; q("beat").textContent = T("conv.cancelling"); ctl.abort(); }; }
  box.appendChild(el);
  /** @type {{ name: string, msg: string, diag?: any }[]} */ const fails = [];
  let frac = /** @type {number | null} */ (null), max = 0, seen = t0, done = 0, cancelled = 0, beat = "", timer = /** @type {any} */ (0);
  const show = () => {
    const now = Date.now(), left = rimanente(frac, now - t0);
    q("time").textContent = T("av.elapsed", { t: tempo(now - t0) }) + (left ? " · " + T("av.left", { t: left }) : "");
    const b = cn.disabled ? "" : now - seen >= STALL ? "stall" : now - seen >= BEAT ? "beat" : "";
    if (b !== beat) { beat = b; q("beat").textContent = b ? T(b === "stall" ? "av.stall" : "av.beat") : ""; }
  };
  timer = setInterval(show, 1000); show();
  return {
    signal: ctl.signal,
    /** item `k` (1..n) starts @param {number} k @param {string} name */
    item(k, name) { q("item").textContent = T("av.item", { k, n: o.n, name }); seen = Date.now(); show(); },
    /** the fraction 0..1 of the whole job (null: not known) and the numbers of the item @param {number | null} f @param {string} [detail] */
    set(f, detail) {
      if (f == null) { frac = null; bar.removeAttribute("aria-valuenow"); fill.style.width = "0%"; }
      else { max = Math.max(max, Math.min(99, Math.floor(100 * f))); frac = max / 100; bar.setAttribute("aria-valuenow", String(max)); fill.style.width = max + "%"; }      // forward only, 100 % only at the end
      if (detail != null && detail !== q("detail").textContent) q("detail").textContent = detail;
      seen = Date.now(); show();
    },
    ok() { done++; },
    /** @param {string} name @param {string} msg @param {any} [diag] a diagnosis of diagnosi.js */
    fail(name, msg, diag) { fails.push({ name, msg, diag }); },
    stop() { cancelled++; },
    /** the summary in place of the bar, with the time taken; `head` replaces «k of n done» @param {{ head?: string }} [x] */
    end(x) {
      clearInterval(timer);
      const parts = [x && x.head ? x.head : T("av.sum", { done, n: o.n })];
      if (fails.length) parts.push(T("conv.sum.fail", { n: fails.length }));
      if (cancelled) parts.push(T("conv.sum.cancel", { n: cancelled }));
      const d = fails.find(f => f.diag), D = w.DIAG;
      const li = fails.map(f => h`<li class="fail">${f.name ? f.name + ": " + f.msg : f.msg}</li>`);
      el.innerHTML = h`<p class="sm" data-r="sum"><b>${o.title}</b>: ${parts.join(", ")} · ${tempo(Date.now() - t0)} <button type="button" class="sec sm" data-r="close">${T("common.close")}</button></p>${raw(fails.length ? h`<ul class="sm">${li.map(raw)}</ul>` : "")}${raw(d && D ? D.html(d.diag, esc, T) : "")}`;
      if (d && D) D.cabla(el, d.diag, w.MZLAB_VERSION || "", T);
      q("close").onclick = () => el.remove();
      if (!fails.length) setTimeout(() => el.remove(), KEEP);
    },
    remove() { clearInterval(timer); el.remove(); },
  };
}
