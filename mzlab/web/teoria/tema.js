/* Theme of the Teoria (the minimal part, data-theme + colour-scheme, also runs inline at the top of every page: tools/tema_testa.py; rule: theme first, synchronous, before any stylesheet): follows the app («Chiaro», «Scuro», «Come il sistema»). Loaded in the <head> of every page, before anything is drawn.
   Inside the app the page asks the parent (same origin) and listens for its messages; opened alone it follows the system. */
(function () {
  var R = document.documentElement, mq = window.matchMedia ? matchMedia("(prefers-color-scheme: dark)") : null, pref = "auto";
  function parentPref() { try { if (parent !== window) return parent.document.documentElement.getAttribute("data-theme") || "auto"; } catch (e) { /* not reachable: own preference */ } try { var t = JSON.parse(localStorage.getItem("qqq.prefs") || "{}").theme; if (t === "dark" || t === "light") return t; } catch (e) { /* no storage: system */ } return "auto"; }
  function eff(x) { return x === "dark" || (x !== "light" && mq && mq.matches) ? "dark" : "light"; }
  function apply(x) { pref = x; R.setAttribute("data-theme", eff(x)); R.style.colorScheme = eff(x); }
  apply(parentPref());
  // «Lettura facilitata»: the choices (localStorage «qqq.a11y», JSON of codes) become data-a11y-* attributes of <html> before the page is drawn; the rules are in teoria.css, the panel in a11y.js
  var A11Y_KEYS = { font: ["atkinson", "dyslexic"], bg: ["cream", "blue", "hc"], motion: ["auto", "always"] };
  function a11yRead() { var o = {}; try { o = JSON.parse(localStorage.getItem("qqq.a11y") || "{}") || {}; } catch (e) { /* no storage: defaults */ } return o; }
  function a11yApply(o) {
    function set(n, v) { if (v) R.setAttribute("data-a11y-" + n, v === true ? "" : v); else R.removeAttribute("data-a11y-" + n); }
    set("font", A11Y_KEYS.font.indexOf(o.font) >= 0 ? o.font : ""); set("bg", A11Y_KEYS.bg.indexOf(o.bg) >= 0 ? o.bg : "");
    set("space", !!o.space); set("nums", !!o.nums); set("ruler", !!o.ruler); set("focus", !!o.focus);
    var rm = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    set("reduce", o.motion === "always" || (rm && o.motion !== "never"));
  }
  window.QA11Y = { read: a11yRead, apply: a11yApply, write: function (o) { try { localStorage.setItem("qqq.a11y", JSON.stringify(o)); } catch (e) { /* not stored */ } a11yApply(o); } };
  a11yApply(a11yRead());
  // the drawings on canvas use fixed colours of the light theme: in the dark one they are swapped for their dark equivalents (the SVG ones are in teoria.css)
  var MAP = { "#24231f": "#e4e7eb", "#fff": "#1d2127", "#ffffff": "#1d2127", "#f4f3ef": "#252a31", "#c9c5bb": "#3a424e", "#e9e7e1": "#2c323b", "#6b675c": "#98a2b0",
    "#57534e": "#b6bfcb", "#2b5c8a": "#7db4e6", "#c2410c": "#fb923c", "#b42318": "#f87171", "#0e7490": "#22d3ee", "#b45309": "#fbbf24", "#efe3dc": "#3a2f29", "#e4cfc3": "#5a463b",
    "rgba(255,255,255,.85)": "rgba(29,33,39,.85)", "rgba(36,35,31,.55)": "rgba(228,231,235,.55)" };
  // «Alto contrasto»: axes, text and lines of the figures reach >= 7:1 against the page (the light-theme originals are the keys); grids and light fills stay faint
  var HC = {
    light: { "#24231f": "#000000", "#9b978c": "#595959", "#6b675c": "#1a1a1a", "#57534e": "#1a1a1a", "#c9c5bb": "#595959", "#999": "#595959", "#2b5c8a": "#0030a0", "#c2410c": "#7a2800",
      "#b42318": "#8c0000", "#0e7490": "#004f5e", "#b45309": "#5c2a00", "#047857": "#00522f", "#7c3aed": "#4b1fa0", "rgba(36,35,31,.55)": "rgba(0,0,0,.75)" },
    dark: { "#24231f": "#ffffff", "#9b978c": "#bdbdbd", "#6b675c": "#ececec", "#57534e": "#ececec", "#c9c5bb": "#bdbdbd", "#999": "#bdbdbd", "#2b5c8a": "#8fd0ff", "#c2410c": "#ffb36b",
      "#b42318": "#ff9a9a", "#0e7490": "#67e8f9", "#b45309": "#ffd36b", "#047857": "#6ee7a8", "#7c3aed": "#c4b0ff", "#fff": "#000000", "#ffffff": "#000000", "#f4f3ef": "#1a1a1a", "#e9e7e1": "#4d4d4d", "rgba(255,255,255,.85)": "rgba(0,0,0,.85)", "rgba(36,35,31,.55)": "rgba(255,255,255,.75)" }
  };
  if (window.CanvasRenderingContext2D) ["fillStyle", "strokeStyle"].forEach(function (k) {
    var d = Object.getOwnPropertyDescriptor(CanvasRenderingContext2D.prototype, k); if (!d || !d.set) return;
    Object.defineProperty(CanvasRenderingContext2D.prototype, k, { configurable: true, enumerable: d.enumerable, get: d.get, set: function (v) {
      if (typeof v === "string") {
        var k = v.replace(/\s/g, "").toLowerCase(), th = R.getAttribute("data-theme") === "dark" ? "dark" : "light", m = R.getAttribute("data-a11y-bg") === "hc" ? HC[th][k] : null;
        if (!m && th === "dark") m = MAP[k];
        if (m) v = m;
      }
      d.set.call(this, v);
    } });
  });
  // colours written in the <style> blocks and style="" attributes of the pages (the figures define classes such as .bx{fill:#fff}) get the same swap
  function swapText(t) {
    var hc = R.getAttribute("data-a11y-bg") === "hc" ? HC[R.getAttribute("data-theme") === "dark" ? "dark" : "light"] : null;
    return t.replace(/#[0-9a-fA-F]{3,6}\b|rgba\([0-9, .]+\)/g, function (c) { var k = c.replace(/\s/g, "").toLowerCase(); return (hc && hc[k]) || (R.getAttribute("data-theme") === "dark" && MAP[k]) || c; });
  }
  document.addEventListener("DOMContentLoaded", function () {
    if (R.getAttribute("data-theme") !== "dark" && R.getAttribute("data-a11y-bg") !== "hc") return;
    document.querySelectorAll("body style").forEach(function (st) { st.textContent = swapText(st.textContent); });
    document.querySelectorAll("[style]").forEach(function (el) { var v = el.getAttribute("style"); if (/#|rgba/.test(v)) el.setAttribute("style", swapText(v)); });
  });
  // the app changed the theme: the simulations read their colours once, so the page is simply loaded again, at the same place
  addEventListener("message", function (e) {
    if (!e.data || e.data.type !== "qqq-theme") return;
    var before = R.getAttribute("data-theme"); apply(e.data.theme || "auto");
    if (before !== R.getAttribute("data-theme")) { try { sessionStorage.setItem("qqq.teoria.y", String(scrollY)); } catch (x) { /* nothing to keep */ } location.reload(); }
  });
  addEventListener("load", function () { try { var y = sessionStorage.getItem("qqq.teoria.y"); if (y != null) { sessionStorage.removeItem("qqq.teoria.y"); scrollTo(0, +y); } } catch (x) { /* ignore */ } });
  if (mq && mq.addEventListener) mq.addEventListener("change", function () { if (pref === "auto" && parentPref() === "auto") { var before = R.getAttribute("data-theme"); apply("auto"); if (before !== R.getAttribute("data-theme")) location.reload(); } });
})();
