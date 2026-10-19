// @ts-strict
// Start screen of the page: loading phrases and the three-ion animation (functions used by loading() in explore.js).
// Classic script, run in place by index.html: it shares the global scope with the Dati scripts.
document.title = I18N.t("app.title", { app: window.APP_NAME || "" });
const NPHRASES = 17;   // the phrases are the catalog keys load.phrase.1 ... load.phrase.17 (lang/it.js, en.js)
// loading screen (functions used by loading() in explore.js): one phrase every 5 s, in random order without repeats; the three dots appear one after the other
let ldTimer = null, ldDotTimer = null, ldShown = "", ldSince = 0, ldBag = [], ldLast = -1;
// data loading: three ions drawn always the same way (a dot with a fading trail of its past positions), advancing and weaving together.
// The analyser screens (an-*.js, docs/agenti/analizzatori.md) are switched off: LD_ANALYSERS = true loads them on demand and picks one at random.
const LD_ANALYSERS = false, LD_AN = ["util", "quad", "paul", "tof", "settore", "orbi", "ims", "esi", "maldi", "trappola", "fticr", "qqq"];
let ldIonRaf = 0, ldAnLoad = null;
// plain scripts, in order (an-util first); resolves once they have all run
function ldAnLoaded() {
  return ldAnLoad || (ldAnLoad = LD_AN.reduce((p, n) => p.then(() => new Promise(ok => { const s = document.createElement("script"); s.src = "static/an-" + n + ".js"; s.onload = s.onerror = /** @type {any} */ (ok); document.head.appendChild(s); })), Promise.resolve()));
}
function ldIon(on, mode) {
  if (on && LD_ANALYSERS && !/** @type {any} */ (window).LDAN) { ldAnLoaded().then(() => ldIon(on, mode)); return; }
  // already turning (first paint, then engine start and data loading): keep the same analyser
  if (on && ldIonRaf && !mode) return;
  const cv = /** @type {HTMLCanvasElement | null} */ (document.getElementById("ldion"));
  FRAME.cancel(ldIonRaf); ldIonRaf = 0;
  if (!cv) return;
  cv.style.display = on ? "block" : "none";
  if (!on) return;
  const W = 420, H = 120, R = window.devicePixelRatio || 1, TAU = 2 * Math.PI;
  const css = getComputedStyle(document.documentElement), v = n => css.getPropertyValue(n).trim();
  const col = [v("--accent"), v("--ok"), v("--bad")], line = v("--line") || "#ccc";
  cv.width = W * R; cv.height = H * R; const cx = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")); cx.setTransform(R, 0, 0, R, 0, 0);
  // the plain animation: three ions 120° apart, their trails advancing and weaving together (depth z sets size and brightness)
  const ctx = { cx, W, H, TAU, line, muted: v("--muted") || line }, A = { base: {
    n: 26, dt: 0.045, step: 9,
    at(j, t) { const ph = TAU / 1.6 * t + j * TAU / 3; return { x: 0, y: H / 2 + 30 * Math.sin(ph), z: Math.cos(ph) }; },
    shift(k) { return W / 2 + 25 * 9 / 2 - k * 9; } } };
  // an-*.js: window.LDAN.<name>(ctx) -> { n, dt, T?, at(j, t), draw?(t), mid? }
  if (LD_ANALYSERS) for (const k of Object.keys(/** @type {any} */ (window).LDAN || {})) A[k] = /** @type {any} */ (window).LDAN[k](ctx);
  const keys = Object.keys(A), an = A[mode in A ? mode : LD_ANALYSERS ? keys[1 + Math.floor(Math.random() * (keys.length - 1))] : "base"];
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches || document.documentElement.hasAttribute("data-a11y-reduce"), t0 = performance.now();
  const frame = ms => {
    const t = still ? 1.2 : (ms - t0) / 1000, pts = [];
    cx.clearRect(0, 0, W, H); cx.globalAlpha = 1; if (an.draw) an.draw(t);
    for (let j = 0; j < 3; j++) for (let k = 0; k < an.n; k++) {
      const tk = t - k * an.dt; if (an.T && tk < 0) continue;
      const q = an.at(j, tk); if (!q) continue;
      const z = q.z ?? 0, x = an.shift ? an.shift(k) : q.x;
      pts.push({ x, y: q.y, z, r: (5.5 + 2 * z) * (1 - k / an.n * 0.7) * (q.s ?? 1), a: (1 - k / an.n) * (0.6 + 0.4 * (z + 1) / 2), c: col[j] });
    }
    pts.sort((p, q) => p.z - q.z);
    let midDone = !an.mid;
    for (const p of pts) {
      if (!midDone && p.z >= 0) { cx.globalAlpha = 1; an.mid(); midDone = true; }
      cx.globalAlpha = p.a; cx.fillStyle = p.c; cx.beginPath(); cx.arc(p.x, p.y, p.r, 0, TAU); cx.fill();
    }
    cx.globalAlpha = 1; if (!midDone) an.mid();
    if (!still) ldIonRaf = FRAME.next(frame);
  };
  ldIonRaf = FRAME.next(frame);
}
// the ONE status line of the loading screen (#ldmsg): the text of a load in progress (loading(true, msg)) or the phase of the engine start
// (window.qqPhaseCur, set by browser.js) replaces the funny phrases; the bar under it is determinate only when the percentage is known
let ldMsg = null;
function ldRefresh() {
  const ph = window.qqPhaseCur, cur = ldMsg || (ph && window.I18N ? { text: I18N.t(ph.key, ph.params), pct: ph.pct } : null);
  const el = document.getElementById("ldmsg");
  if (!el || window.qqFailed) return;
  // no status text: a phrase keeps running as it is; after a status text the SAME phrase comes back (never a new one before its time)
  if (!cur) {
    el.setAttribute("aria-live", "off");
    if (ldTimer && ldShown === "msg") { if (ldLast >= 0) ldShow(ldLast); else ldNext(); }
    return;
  }
  ldShown = "msg";
  clearInterval(ldDotTimer); const pct = cur.pct == null ? null : Math.max(0, Math.min(100, Math.round(cur.pct)));
  el.setAttribute("aria-live", "polite"); el.textContent = "";
  const ic = document.createElementNS("http://www.w3.org/2000/svg", "svg"); ic.setAttribute("class", "ldic"); ic.setAttribute("viewBox", "0 0 24 24"); ic.setAttribute("aria-hidden", "true"); ic.setAttribute("focusable", "false");
  ic.innerHTML = '<path d="M12 3v12m0 0l-4-4m4 4l4-4M4 17v3h16v-3"/>'; el.appendChild(ic); el.appendChild(document.createTextNode(cur.text));
  // the percentage is shown, not announced at every tick
  if (pct != null) { const s = document.createElement("span"); s.setAttribute("aria-hidden", "true"); s.textContent = ` ${pct}%`; el.appendChild(s); }
}
// a button under the status line (stuck start: «start from scratch» or full reset)
function ldAction(label, fn) {
  const a = document.getElementById("ldact"); if (!a) return; a.textContent = ""; a.hidden = !label; if (!label) return;
  const b = document.createElement("button"); b.className = "btn imp"; b.id = "ldreset"; b.type = "button"; b.textContent = label; b.onclick = fn; a.appendChild(b); b.focus();
}
let ldBootTimer = null;
function ldFullReset() {
  const b = /** @type {HTMLButtonElement | null} */ (document.querySelector("#ldact button"));
  if (b) b.disabled = true;
  const msg = document.getElementById("ldmsg");
  if (msg && window.I18N) msg.textContent = I18N.t("reset.busy");
  if (window.qqHardReset) window.qqHardReset();
  else if (window.QQRESET && window.QQRESET.run) window.QQRESET.run("all");
  else location.reload();
}
function ldArmBootTimer() {
  if (ldBootTimer || window.qqReady) return;
  ldBootTimer = setTimeout(() => {
    if (window.qqReady) return;
    const L = document.getElementById("loading");
    if (!L || L.hidden) return;
    const label = window.I18N ? I18N.t("reset.all.btn") : "Full reset";
    ldAction(label, ldFullReset);
  }, 10000);
}
function ldClearBootTimer() {
  if (ldBootTimer) { clearTimeout(ldBootTimer); ldBootTimer = null; }
}
function ldNext() {
  // a failed start keeps its error message on screen
  if (window.qqFailed || ldMsg || window.qqPhaseCur) return;
  // refill with a fresh shuffle, never starting with the phrase just shown
  if (!ldBag.length) {
    ldBag = Array.from({ length: NPHRASES }, (_, i) => i).sort(() => Math.random() - 0.5);
    if (ldBag[ldBag.length - 1] === ldLast) ldBag.unshift(/** @type {number} */ (ldBag.pop()));
  }
  ldShow(ldBag.pop());
}
function ldShow(i) {                                              // phrase i (0-based) with its three dots
  ldLast = i; ldShown = "phrase";
  const el = document.getElementById("ldmsg"); if (!el) return; el.innerHTML = I18N.t(`load.phrase.${i + 1}`);
  const dots = document.createElement("span");
  // the phrase appears at once with its three dots (a fast load must still show them)
  for (let k = 0; k < 3; k++) { const d = document.createElement("span"); d.textContent = "."; dots.appendChild(d); }
  el.appendChild(dots);
  let n = 3; clearInterval(ldDotTimer);
  ldDotTimer = setInterval(() => { n = (n + 1) % 4; /** @type {HTMLElement[]} */ ([...dots.children]).forEach((d, i) => { d.style.visibility = i < n ? "visible" : "hidden"; }); }, 450);
}
if (!window.QQQ_PHONE) { ldArmBootTimer(); }
window.ldAction = ldAction;
window.ldFullReset = ldFullReset;
window.ldArmBootTimer = ldArmBootTimer;
window.ldClearBootTimer = ldClearBootTimer;

