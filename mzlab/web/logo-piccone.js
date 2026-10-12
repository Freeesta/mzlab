// Easter egg of the header logo: five hits with a small pickaxe switch mzFinder on (or off, see tpmine-loader.js). Each hit shakes the logo and adds a
// stage of cracks (1-4); the fifth shatters it into pieces, flips the switch and leaves the logo GOLDEN for good (localStorage, as a trophy).
// No hit for a few seconds: the cracks close again. Reduced motion: no shake, no pickaxe, no shards, only the stages of cracks. Enter / Space on the focused logo = a hit.
// Pickaxe, cracks and shards are inline SVG/CSS (no asset). The label of the logo (alt/title) never changes.
(() => {
  "use strict";
  const HITS = 5, HEAL_MS = 3000, GOLD_KEY = "qqq.logo.gold";
  const logo = document.querySelector("header img.logo");
  if (!logo) return;
  const t = (k, d) => (window.I18N && I18N.t ? I18N.t(k) : d);
  const still = () => document.documentElement.hasAttribute("data-a11y-reduce") || !logo.animate || matchMedia("(prefers-reduced-motion: reduce)").matches;
  const root = document.documentElement;
  const store = { get() { try { return localStorage.getItem(GOLD_KEY) === "1"; } catch (_) { return false; } }, set() { try { localStorage.setItem(GOLD_KEY, "1"); } catch (_) { /* private mode: golden for this page only */ } } };

  const st = document.createElement("style");
  st.textContent = `:root{--gold:#c9a227}
html[data-logo-gold] header img.logo{filter:sepia(1) saturate(3.5) hue-rotate(-12deg) brightness(1.12) drop-shadow(0 0 3px var(--gold))}
/* dark theme (also the high-contrast one): the same gold, brighter, so the glyph keeps >= 4.5:1 on the header (measured 7:1 or more) */
@media (prefers-color-scheme:dark){html[data-logo-gold]:not([data-theme=light]) header img.logo{filter:sepia(1) saturate(3) hue-rotate(-12deg) brightness(2.3) drop-shadow(0 0 3px var(--gold))}}
html[data-theme=dark][data-logo-gold] header img.logo{filter:sepia(1) saturate(3) hue-rotate(-12deg) brightness(2.3) drop-shadow(0 0 3px var(--gold))}
header img.logo{cursor:pointer;border-radius:4px}header img.logo:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
#lg-fx{position:fixed;z-index:6100;pointer-events:none;overflow:visible}#lg-fx svg{position:absolute;left:0;top:0;overflow:visible}
#lg-fx .lg-cr{fill:none;stroke-linecap:round;stroke-linejoin:round;opacity:0;transition:opacity .15s}#lg-fx .lg-cr.on{opacity:1}
#lg-fx .lg-ck1{stroke:var(--panel);stroke-width:2.6}#lg-fx .lg-ck2{stroke:var(--ink);stroke-width:1.1}
#lg-fx .lg-pk{position:absolute;transform-origin:100% 100%;opacity:0}#lg-fx .lg-pk svg{position:static}
@media (prefers-reduced-motion:reduce){#lg-fx .lg-cr{transition:none}}`;
  document.head.appendChild(st);

  logo.setAttribute("tabindex", "0"); logo.setAttribute("role", "button");
  if (store.get()) root.setAttribute("data-logo-gold", "");
  const live = document.createElement("div"); live.className = "sr-only"; live.setAttribute("aria-live", "polite"); document.body.appendChild(live);

  // cracks per stage, in logo units (the logo is 26 x 26); the shards tile the whole square
  const CRACKS = ["M13 1 L11 8 L14 12", "M11 8 L5 10 L3 16", "M14 12 L19 14 L22 21", "M14 12 L12 18 L15 25 M12 18 L7 22"];
  const SHARDS = ["0,0 13,0 11,9 0,12", "13,0 26,0 26,10 17,15 11,9", "0,12 11,9 8,18 0,26", "11,9 17,15 8,18", "17,15 26,10 26,26 14,26 8,18", "8,18 14,26 0,26"];
  const PICK = `<svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true" focusable="false"><path d="M18 18 L7 7" stroke="var(--ink)" stroke-width="1.8" stroke-linecap="round"/><path d="M2 8 Q6 1 14 3" fill="none" stroke="var(--muted)" stroke-width="2.4" stroke-linecap="round"/></svg>`;

  let stage = 0, fx = null, healT = 0;
  function overlay() {
    const r = logo.getBoundingClientRect(), s = r.width / 26, pad = 14 * s;
    if (!fx) {
      fx = document.createElement("div"); fx.id = "lg-fx"; fx.setAttribute("aria-hidden", "true");
      fx.innerHTML = `<svg viewBox="0 0 26 26" width="${r.width}" height="${r.height}">${CRACKS.map(d => `<g class="lg-cr"><path class="lg-ck1" d="${d}"/><path class="lg-ck2" d="${d}"/></g>`).join("")}</svg><span class="lg-pk">${PICK}</span>`;
      document.body.appendChild(fx);
    }
    fx.style.left = r.left + "px"; fx.style.top = r.top + "px"; fx.style.width = r.width + "px"; fx.style.height = r.height + "px";
    const pk = fx.querySelector(".lg-pk"); pk.style.left = (r.width - 4 * s) + "px"; pk.style.top = (-pad * 0.55) + "px";
    return fx;
  }
  function showStage(n) {
    stage = n;
    if (!n) { clearTimeout(healT); if (fx) { fx.querySelectorAll(".lg-cr").forEach(g => g.classList.remove("on")); const f = fx; setTimeout(() => { if (f === fx && !stage) { f.remove(); fx = null; } }, 250); } return; }
    overlay().querySelectorAll(".lg-cr").forEach((g, i) => g.classList.toggle("on", i < n));
    clearTimeout(healT); healT = setTimeout(() => showStage(0), HEAL_MS);
  }
  function swing() {
    if (still()) return;
    const f = overlay(), pk = f.querySelector(".lg-pk");
    pk.getAnimations().forEach(a => a.cancel());
    pk.animate([{ transform: "rotate(-55deg)", opacity: 1 }, { transform: "rotate(8deg)", opacity: 1, offset: 0.4 }, { transform: "rotate(-10deg)", opacity: 1, offset: 0.7 }, { transform: "rotate(-55deg)", opacity: 0 }], { duration: 320, easing: "ease-out" });
    logo.getAnimations().forEach(a => a.cancel());
    const k = 1 + stage * 0.35;                                     // the more damaged, the harder it shakes
    logo.animate([{ transform: "translate(0,0)" }, { transform: `translate(${-1.6 * k}px,${0.6 * k}px)`, offset: 0.2 }, { transform: `translate(${1.6 * k}px,${-0.6 * k}px)`, offset: 0.45 }, { transform: `translate(${-1 * k}px,0)`, offset: 0.7 }, { transform: "translate(0,0)" }], { duration: 260 });
  }
  function shatter() {
    const gold = () => { store.set(); root.setAttribute("data-logo-gold", ""); live.textContent = t("logo.gold.live", "Logo turned gold"); };
    if (still()) { showStage(0); gold(); return; }
    const f = overlay(), pk = f.querySelector(".lg-pk"); pk.remove();
    f.querySelectorAll(".lg-cr").forEach(g => g.classList.remove("on"));
    const svg = f.querySelector("svg"), ns = "http://www.w3.org/2000/svg", pieces = [];
    SHARDS.forEach((pts, i) => {
      const id = "lg-cp" + i, cp = document.createElementNS(ns, "clipPath"); cp.id = id;
      const pg = document.createElementNS(ns, "polygon"); pg.setAttribute("points", pts); cp.appendChild(pg); svg.appendChild(cp);
      const g = document.createElementNS(ns, "g"); g.setAttribute("clip-path", `url(#${id})`);
      const im = document.createElementNS(ns, "image"); im.setAttribute("href", logo.currentSrc || logo.src); im.setAttribute("width", "26"); im.setAttribute("height", "26");
      g.appendChild(im); svg.appendChild(g); pieces.push([g, pts]);
    });
    logo.style.opacity = "0"; clearTimeout(healT); stage = 0;
    let done = 0;
    pieces.forEach(([g, pts], i) => {
      const c = pts.split(" ").map(p => p.split(",").map(Number)), cx = c.reduce((a, p) => a + p[0], 0) / c.length - 13, cy = c.reduce((a, p) => a + p[1], 0) / c.length - 13, d = Math.hypot(cx, cy) || 1;
      const a = g.animate([{ transform: "translate(0,0)", opacity: 1 }, { transform: `translate(${cx / d * 16}px,${cy / d * 16 + 6}px)`, opacity: 0 }], { duration: 650 + i * 40, easing: "cubic-bezier(.2,.7,.4,1)", fill: "forwards" });
      a.onfinish = () => { if (++done === pieces.length) { f.remove(); fx = null; gold(); logo.style.opacity = ""; logo.animate([{ opacity: 0, transform: "scale(.7)" }, { opacity: 1, transform: "scale(1.15)", offset: 0.6 }, { opacity: 1, transform: "scale(1)" }], { duration: 400 }); } };
    });
  }
  function hit() {
    if (logo.style.opacity === "0") return;                         // the shards are flying: wait for the logo to be whole again
    const n = stage + 1;
    if (n >= HITS) { if (window.QTOOLS && QTOOLS.flip) QTOOLS.flip(); shatter(); return; }
    showStage(n); swing();
  }
  document.addEventListener("click", ev => { if (ev.target && ev.target.closest && ev.target.closest("header img.logo")) hit(); }, true);
  logo.addEventListener("keydown", ev => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); hit(); } });
})();
