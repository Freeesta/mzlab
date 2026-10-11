/* QqQ lab - Teoria: shared layout (header, chapter list, on-page TOC, prev/next) and tiny plotting helpers.
   Classic script, no modules, no network: the pages work from file:// and from the local server. */
"use strict";
// [file, number shown in the menu, title, part]. The order is the study path: each chapter uses only what comes before it.
// The "Pratica" pages (pratica*.html) are the exercises and games; they use the same layout.
const CHAPTERS = [
  ["00-uso.html", "A", "Come si usa {APP}", "Guida"],
  ["index.html", "0", "Introduzione e mappa del percorso", "Guida"],
  ["01-tp.html", "1", "Prodotti di trasformazione", "I. Il problema"],
  ["02-fotocatalisi.html", "2", "Fotocatalisi con TiO₂", "I. Il problema"],
  ["03-cromatografia.html", "3", "Fondamenti di cromatografia", "II. Separare"],
  ["04-lc.html", "4", "Cromatografia liquida (HPLC)", "II. Separare"],
  ["05-gc.html", "5", "Gascromatografia (GC)", "II. Separare"],
  ["06-esi.html", "6", "Elettrospray (ESI) e sorgenti a pressione atmosferica", "III. Ionizzare"],
  ["07-ei-ci.html", "7", "Ionizzazione elettronica (EI) e chimica (CI)", "III. Ionizzare"],
  ["08-vuoto.html", "8", "Dalla sorgente al vuoto", "III. Ionizzare"],
  ["09-quadrupolo.html", "9", "Il quadrupolo: teoria", "IV. Analizzare gli ioni"],
  ["10-qqq.html", "10", "Il triplo quadrupolo e la CID", "IV. Analizzare gli ioni"],
  ["11-risoluzione.html", "11", "Risoluzione e alta risoluzione: TOF, Orbitrap", "IV. Analizzare gli ioni"],
  ["12-dati.html", "12", "Full scan, MS2 e MRM: leggere i dati", "V. Leggere gli spettri"],
  ["13-formula.html", "13", "Dalla massa alla formula", "V. Leggere gli spettri"],
  ["14-frammentazione-esi.html", "14", "Frammentazione in CID (ioni a elettroni pari)", "V. Leggere gli spettri"],
  ["15-ei-metodo.html", "15", "Risolvere uno spettro EI: il metodo", "V. Leggere gli spettri"],
  ["16-ei-famiglie.html", "16", "Le famiglie di composti in EI", "V. Leggere gli spettri"],
  ["17-origine.html", "17", "Da dove viene questo ione?", "V. Leggere gli spettri"],
  ["18-strategia.html", "18", "Strategia per trovare i TP", "VI. Dal dato al risultato"],
  ["19-hr-dda.html", "19", "Alta risoluzione e DDA nel programma", "VI. Dal dato al risultato"],
  ["20-disegno.html", "20", "Disegnare le molecole", "VI. Dal dato al risultato"],
  ["21-glossario.html", "21", "Glossario e bibliografia", "Appendice"],
  ["22-formulario.html", "22", "Formulario per l'orale", "Appendice"],
  ["pratica.html", "P", "Allenarsi per l'orale", "Pratica"],
  ["pratica-ei.html", "P1", "Dallo spettro alla struttura", "Pratica"],
  ["pratica-orale.html", "P2", "Domande dell'orale", "Pratica"],
  ["pratica-perdite.html", "P3", "Che cosa ha perso?", "Pratica"],
  ["pratica-isotopi.html", "P4", "Indovina gli elementi", "Pratica"],
  ["pratica-formula.html", "P5", "Formula esatta", "Pratica"],
  ["pratica-strumento.html", "P6", "Carte dello strumento", "Pratica"],
  ["pratica-quadrupolo.html", "P7", "Pilota il quadrupolo", "Pratica"],
  ["pratica-tp.html", "P8", "Δm ed ESI± dei prodotti di trasformazione", "Pratica"],
];
const APP = typeof APP_NAME === "string" ? APP_NAME : "mzLab";        // appname.js is loaded first by every page
CHAPTERS.forEach(c => { c[2] = c[2].split("{APP}").join(APP); });
// the chapters were written before the name changed: the old name in the text of the page is shown as the current one
function renameApp(root = document.body) {
  const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, { acceptNode: n => /^(SCRIPT|STYLE|TEXTAREA)$/.test(n.parentNode.nodeName) || !n.nodeValue.includes("QqQ lab") ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT }), l = [];
  while (w.nextNode()) l.push(w.currentNode);
  l.forEach(n => { n.nodeValue = n.nodeValue.split("QqQ lab").join(APP); });
  root.querySelectorAll("[title*='QqQ lab'],[alt*='QqQ lab']").forEach(e => ["title", "alt"].forEach(a => { const v = e.getAttribute(a); if (v && v.includes("QqQ lab")) e.setAttribute(a, v.split("QqQ lab").join(APP)); }));
}
addEventListener("load", () => renameApp());

const TP = (() => {
  const $ = (q, el = document) => el.querySelector(q);
  const here = location.pathname.split("/").pop() || "index.html";
  const idx = Math.max(0, CHAPTERS.findIndex(c => c[0] === here));
  // teoria/#5, teoria/#cap-5, teoria/#A, teoria/#09-quadrupolo: the index page forwards to that chapter (one address per chapter without rewriting URLs)
  if (here === "index.html") {
    const k = location.hash.slice(1).toLowerCase().replace(/^cap-?/, "");
    const hit = k && CHAPTERS.find(c => c[0] !== "index.html" && (c[1].toLowerCase() === k || c[0].replace(".html", "") === k));
    if (hit) location.replace(hit[0]);
  }
  const embedded = window.self !== window.top;
  // smartphone: same rule as ../telefono.js (which the program loads before everything; the Teoria pages do not load it)
  const PHONE = typeof QQQ_PHONE === "boolean" ? QQQ_PHONE : (() => {
    let f = null; try { f = sessionStorage.getItem("qqq.telefono"); } catch (_) { /* automatic */ } // kept from the old name: renaming it would lose the users' data
    if (f !== null) return f === "1";
    const coarse = !!(window.matchMedia && matchMedia("(pointer:coarse)").matches);
    return /iPhone|iPod|Android.+Mobile|Windows Phone|Mobile.+Firefox|Opera Mini/i.test(navigator.userAgent || "") || (coarse && Math.min(screen.width, screen.height) < 500);
  })();
  // on a phone the exercises are not offered (they need a large screen): the page says so and sends back to the Teoria.
  // The oral questions are only text: they stay.
  const NOEX = PHONE && /^pratica-/.test(here) && here !== "pratica-orale.html";

  function slug(s) {
    return s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 48);
  }

  function layout() {
    // wide tables scroll horizontally on phones instead of widening the page
    document.querySelectorAll("main table").forEach(t => { const w = document.createElement("div"); w.className = "tw"; t.parentNode.insertBefore(w, t); w.appendChild(t); });
    const top = document.createElement("header"); top.id = "top";
    // inside the program the main bar already has the logo and the tabs: no second header there (only the index button on narrow screens)
    if (embedded) document.body.classList.add("emb");
    // standalone header: logo (to the program), title and the same three tabs as the program (Dati · Disegno · Teoria); always visible.
    // From file:// the relative links to the program do not exist (as before: logo to the index, no tabs); on a phone only the logo (to the notice) and Teoria.
    const prog = location.protocol !== "file:", tabs = !prog ? "" : `<nav id="tabs" aria-label="Sezioni di ${APP}">` +
      (PHONE ? "" : `<a id="tab-dati" href="../../#/dati">Dati</a><a id="tab-disegno" href="../../#/disegno">Disegno</a>`) +
      `<a id="tab-teoria" href="index.html" class="on" aria-current="page">Teoria</a></nav>`;
    top.innerHTML = `<button id="menu" type="button" aria-label="Indice" aria-expanded="false" aria-controls="side">&#9776;</button>` + (embedded ? "" :
      `<a id="logo" href="${!prog ? "index.html" : PHONE ? "../../" : "../../#/dati"}" aria-label="${!prog ? "Teoria: introduzione" : PHONE ? APP : APP + ": Dati"}"><img src="../logo-64.png" alt="" width="28" height="28"></a>
      <span class="t">${APP} <small>· Teoria</small></span><span class="sp"></span>` + tabs);
    document.body.prepend(top);
    const side = $("#side"); side.setAttribute("aria-label", "Indice dei capitoli e della pagina");
    // «Salta al contenuto»: first thing reached with Tab, visible only on focus
    const mn = $("main"); mn.id = mn.id || "contenuto"; mn.tabIndex = -1;
    const sk = document.createElement("a"); sk.className = "skip"; sk.href = "#" + mn.id; sk.textContent = "Salta al contenuto";
    sk.onclick = e => { e.preventDefault(); mn.focus(); mn.scrollIntoView(); };
    document.body.prepend(sk);
    // chapters grouped by part: a small heading each time the part changes
    const list = CHAPTERS.map((c, i) => (c[3] && c[3] !== (CHAPTERS[i - 1] || [])[3] ? `<li class="part">${c[3]}</li>` : "") +
      `<li><a href="${c[0]}" class="${i === idx ? "on" : ""}"${i === idx ? ' aria-current="page"' : ""}><b>${c[1]}</b><span>${c[2]}</span></a></li>`).join("");
    // on-page table of contents from h2/h3
    const hs = [...document.querySelectorAll("main h2, main h3")];
    hs.forEach(h => { if (!h.id) h.id = slug(h.textContent); });
    const toc = hs.map(h => `<a href="#${h.id}" class="${h.tagName === "H3" ? "l3" : ""}">${h.textContent}</a>`).join("");
    side.innerHTML = `<h4>Capitoli</h4><ol>${list}</ol>${toc ? `<h4>In questa pagina</h4><nav class="toc" aria-label="In questa pagina">${toc}</nav>` : ""}`;
    $("#menu").onclick = () => $("#menu").setAttribute("aria-expanded", String(side.classList.toggle("open")));
    side.addEventListener("click", e => { if (e.target.closest("a")) { side.classList.remove("open"); $("#menu").setAttribute("aria-expanded", "false"); } });   // narrow screens: the index is a full-screen panel
    if (PHONE) phone(top);
    // prev / next
    const pn = document.createElement("div"); pn.className = "pn";
    const p = CHAPTERS[idx - 1], n = CHAPTERS[idx + 1];
    pn.innerHTML = (p ? `<a href="${p[0]}"><small>&larr; Precedente</small>${p[1]}. ${p[2]}</a>` : "<span></span>") +
                   (n ? `<a href="${n[0]}" style="text-align:right"><small>Successivo &rarr;</small>${n[1]}. ${n[2]}</a>` : "<span></span>");
    $("main").appendChild(pn);
    // highlight the current section in the TOC
    const links = [...side.querySelectorAll(".toc a")];
    if (links.length && "IntersectionObserver" in window) {
      const io = new IntersectionObserver(es => {
        es.forEach(e => { if (e.isIntersecting) links.forEach(a => a.classList.toggle("on", a.getAttribute("href") === "#" + e.target.id)); });
      }, { rootMargin: "-70px 0px -70% 0px" });
      hs.forEach(h => io.observe(h));
    }
    document.title = `${/^[0-9]+$/.test(CHAPTERS[idx][1]) && CHAPTERS[idx][1] !== "0" ? CHAPTERS[idx][1] + ". " : ""}${CHAPTERS[idx][2]} · ${/^pratica/.test(here) ? "Pratica" : "Teoria"} ${APP}`;
    fixLevels();
    // figures without a name take the caption that already stands just above them (no new text)
    document.querySelectorAll("main svg:not([aria-label]):not([aria-labelledby]):not([aria-hidden])").forEach((s, i) => {
      const c = s.previousElementSibling; if (!c || !c.textContent.trim() || c.tagName === "svg") return;
      c.id = c.id || "fig-cap-" + i; s.setAttribute("role", "img"); s.setAttribute("aria-labelledby", c.id);
    });
    renameApp();
    liveFeedback();
  }

  // the answers of the games (.fb) appear after a click: a screen reader announces them (polite) without moving the focus
  // a figure title (h4) right under an h2 would skip a level: its announced level is the one that keeps the order (also for cards added later by the games)
  function fixLevels() {
    let prev = 1; document.querySelectorAll("main h1,main h2,main h3,main h4").forEach(h => { let l = +h.tagName[1]; if (l > prev + 1) { l = prev + 1; h.setAttribute("aria-level", String(l)); } else h.removeAttribute("aria-level"); prev = l; });
  }
  function liveFeedback() {
    const mark = n => { if (n.nodeType === 1) [n, ...n.querySelectorAll(".fb,.readout")].forEach(e => { if (e.classList && (e.classList.contains("fb") || e.classList.contains("readout")) && !e.hasAttribute("aria-live")) e.setAttribute("aria-live", "polite"); }); };
    mark(document.body);
    let t = 0;
    new MutationObserver(ms => { ms.forEach(m => m.addedNodes.forEach(mark)); cancelAnimationFrame(t); t = requestAnimationFrame(fixLevels); }).observe(document.body, { childList: true, subtree: true });
  }

  // ---------------------------------------------------------------- smartphone
  const PHONE_NOTE = `<b>Il sito non è ottimizzato per gli smartphone.</b> Dal telefono si legge tutta la Teoria; l'analisi dei dati, il disegno delle molecole e gli esercizi della Pratica si usano da un computer o da un tablet.`;
  function phone(top) {
    document.documentElement.classList.add("phone");
    if (!embedded) {
      // header: logo, Teoria, info; it goes away scrolling down and comes back scrolling up (more room for the text, above all in landscape)
      top.insertAdjacentHTML("beforeend", `<button id="info" type="button" aria-label="Informazioni" title="Informazioni">i</button>`);
      const note = document.createElement("div"); note.id = "phnote"; note.className = "box warn"; note.hidden = true;
      note.innerHTML = `<p>${PHONE_NOTE}</p><p><a href="index.html">Indice della Teoria</a> · <a href="22-formulario.html">Formulario</a> · <a href="pratica-orale.html">Domande dell'orale</a></p>`;
      $("main").prepend(note);
      $("#info").onclick = () => { note.hidden = !note.hidden; if (!note.hidden) scrollTo({ top: 0, behavior: "smooth" }); };
      let y0 = scrollY;
      addEventListener("scroll", () => { const y = scrollY; top.classList.toggle("away", y > y0 && y > 80 && !$("#side").classList.contains("open")); y0 = y; }, { passive: true });
    }
    if (NOEX) {
      document.body.classList.add("noex");
      $("h1").insertAdjacentHTML("afterend", `<div class="box warn phex"><b>Gli esercizi si fanno da computer o da tablet</b><p>Questo gioco ha bisogno di uno schermo più grande (grafici da cliccare, editor delle strutture). Dal telefono potete ripassare con la Teoria, il formulario e le domande dell'orale.</p><p><a href="index.html">Indice della Teoria</a> · <a href="22-formulario.html">Formulario</a> · <a href="pratica-orale.html">Domande dell'orale</a></p></div>`);
    } else if (here === "pratica.html") {
      $("h1").insertAdjacentHTML("afterend", `<div class="box warn phex"><b>Dal telefono</b><p>I giochi si fanno da computer o da tablet; dal telefono funzionano le <a href="pratica-orale.html">domande dell'orale</a>.</p></div>`);
    }
  }

  // ---------------------------------------------------------------- controls
  /** Build sliders/selects. spec: [{id,label,min,max,step,value,fmt,unit,options:[[v,label]]}] -> returns getter object */
  function controls(el, spec, onchange) {
    const box = document.createElement("div"); box.className = "ctl"; el.appendChild(box);
    const val = {};
    spec.forEach(s => {
      const lab = document.createElement("label");
      if (s.options) {
        lab.innerHTML = `<span>${s.label}</span><select>${s.options.map(o => `<option value="${o[0]}">${o[1]}</option>`).join("")}</select>`;
        const sel = lab.querySelector("select"); sel.value = String(s.value);
        val[s.id] = s.num === false ? sel.value : +sel.value;
        sel.oninput = () => { val[s.id] = s.num === false ? sel.value : +sel.value; onchange(val, s.id); };
      } else if (s.text) {
        lab.innerHTML = `<span>${s.label}</span><input type="text" value="${s.value}" spellcheck="false">`;
        const inp = lab.querySelector("input"); val[s.id] = s.value;
        inp.oninput = () => { val[s.id] = inp.value; onchange(val, s.id); };
      } else {
        const f = s.fmt || (v => v);
        lab.innerHTML = `<span>${s.label}<output aria-live="polite"></output></span><input type="range" min="${s.min}" max="${s.max}" step="${s.step}" value="${s.value}">`;
        const r = lab.querySelector("input"), o = lab.querySelector("output");
        const upd = () => { val[s.id] = +r.value; o.textContent = f(+r.value) + (s.unit ? " " + s.unit : ""); };
        upd(); r.oninput = () => { upd(); onchange(val, s.id); };
        val["_set_" + s.id] = v => { r.value = v; upd(); };
      }
      box.appendChild(lab);
    });
    return val;
  }

  function buttons(el, items, cur, onclick) {
    const b = document.createElement("div"); b.className = "btns"; b.setAttribute("role", "group"); el.appendChild(b);
    items.forEach(([k, lab]) => {
      const x = document.createElement("button"); x.textContent = lab; x.dataset.k = k;
      x.type = "button"; x.setAttribute("aria-pressed", String(k === cur));
      if (k === cur) x.classList.add("on");
      x.onclick = () => { b.querySelectorAll("button").forEach(y => { y.classList.toggle("on", y === x); y.setAttribute("aria-pressed", String(y === x)); }); onclick(k); };
      b.appendChild(x);
    });
    return b;
  }

  // ---------------------------------------------------------------- canvas plotting
  /** A canvas sized for the device pixel ratio. Returns {cv, ctx, W, H, resize()} */
  function canvas(el, h = 260) {
    const cv = document.createElement("canvas"); el.appendChild(cv);
    // the drawing is an image for a screen reader: it says WHAT it shows (the simulation and its axes), never the result
    const box = el.closest(".sim,.game,.gm,section,main"), hd = box && box.querySelector("h4,h3,h2");
    cv.setAttribute("role", "img"); cv.dataset.title = hd ? hd.textContent.trim() : "";
    cv.setAttribute("aria-label", "Grafico" + (cv.dataset.title ? ": " + cv.dataset.title : ""));
    const o = { cv, ctx: cv.getContext("2d"), W: 0, H: h, onresize: null };
    o.resize = () => {
      const w = Math.max(280, cv.clientWidth || el.clientWidth || 600), d = Math.min(4, (window.devicePixelRatio || 1) * ((window.visualViewport && visualViewport.scale) || 1));
      cv.style.height = h + "px"; cv.width = Math.round(w * d); cv.height = Math.round(h * d);
      o.ctx.setTransform(d, 0, 0, d, 0, 0); o.W = w; o.H = h;
    };
    o.resize();
    let t = null;
    const again = ms => { clearTimeout(t); t = setTimeout(() => { o.resize(); o.onresize && o.onresize(); }, ms); };
    window.addEventListener("resize", () => again(120));
    if (window.visualViewport) visualViewport.addEventListener("resize", () => again(150));
    return o;
  }

  function nice(lo, hi, n = 5) {
    const span = hi - lo || 1, step0 = span / n, mag = Math.pow(10, Math.floor(Math.log10(step0)));
    const st = [1, 2, 2.5, 5, 10].map(k => k * mag).find(k => span / k <= n) || 10 * mag;
    const out = []; for (let v = Math.ceil(lo / st) * st; v <= hi + st * 1e-9; v += st) out.push(+v.toFixed(10));
    return out;
  }

  /** Axes frame. opt: {x0,x1,y0,y1,xl,yl,m:{l,r,t,b},logy,grid,xfmt,yfmt}. Returns mapping functions. */
  function axes(c, opt) {
    const { ctx, W, H } = c, m = Object.assign({ l: 56, r: 14, t: 12, b: 38 }, opt.m || {});
    const X = v => m.l + (v - opt.x0) / (opt.x1 - opt.x0) * (W - m.l - m.r);
    const ly = v => opt.logy ? Math.log10(Math.max(v, 1e-12)) : v;
    const Y = v => H - m.b - (ly(v) - ly(opt.y0)) / (ly(opt.y1) - ly(opt.y0)) * (H - m.t - m.b);
    ctx.clearRect(0, 0, W, H);
    if (c.cv) { const t = c.cv.dataset.title; c.cv.setAttribute("aria-label", "Grafico" + (t ? ": " + t : "") + (opt.xl ? ". Asse orizzontale: " + opt.xl : "") + (opt.yl ? ". Asse verticale: " + opt.yl : "") + (opt.logy ? " (scala logaritmica)" : "")); }
    ctx.font = "12px system-ui,sans-serif"; ctx.lineWidth = 1;
    ctx.strokeStyle = "#e9e7e1"; ctx.fillStyle = "#6b675c";
    const xf = opt.xfmt || (v => +v.toFixed(6)), yf = opt.yfmt || (v => +v.toFixed(6));
    ctx.textAlign = "center"; ctx.textBaseline = "top";
    (opt.xticks || nice(opt.x0, opt.x1, Math.max(3, Math.floor((W - m.l - m.r) / 80)))).forEach(v => {
      const x = X(v); if (x < m.l - 1 || x > W - m.r + 1) return;
      if (opt.grid !== false) { ctx.beginPath(); ctx.moveTo(x, m.t); ctx.lineTo(x, H - m.b); ctx.stroke(); }
      ctx.fillText(xf(v), x, H - m.b + 5);
    });
    ctx.textAlign = "right"; ctx.textBaseline = "middle";
    const yt = opt.logy ? (() => { const a = [], e0 = Math.ceil(Math.log10(opt.y0)), e1 = Math.floor(Math.log10(opt.y1)), st = Math.max(1, Math.ceil((e1 - e0) / 7)); for (let e = e0; e <= e1; e += st) a.push(Math.pow(10, e)); return a; })()
                        : (opt.yticks || nice(opt.y0, opt.y1, Math.max(3, Math.floor((H - m.t - m.b) / 45))));
    yt.forEach(v => {
      const y = Y(v); if (y < m.t - 1 || y > H - m.b + 1) return;
      if (opt.grid !== false) { ctx.beginPath(); ctx.moveTo(m.l, y); ctx.lineTo(W - m.r, y); ctx.stroke(); }
      ctx.fillText(opt.logy ? "10" + sup(Math.log10(v)) : yf(v), m.l - 6, y);
    });
    ctx.strokeStyle = "#9b978c"; ctx.beginPath(); ctx.moveTo(m.l, m.t); ctx.lineTo(m.l, H - m.b); ctx.lineTo(W - m.r, H - m.b); ctx.stroke();
    ctx.fillStyle = "#24231f"; ctx.textAlign = "center"; ctx.textBaseline = "bottom";
    if (opt.xl) ctx.fillText(opt.xl, m.l + (W - m.l - m.r) / 2, H - 2);
    if (opt.yl) { ctx.save(); ctx.translate(13, m.t + (H - m.t - m.b) / 2); ctx.rotate(-Math.PI / 2); ctx.textBaseline = "middle"; ctx.fillText(opt.yl, 0, 0); ctx.restore(); }
    return { X, Y, m, W, H, ctx, clip() { ctx.save(); ctx.beginPath(); ctx.rect(m.l, m.t, W - m.l - m.r, H - m.t - m.b); ctx.clip(); } };
  }
  const SUP = { "-": "⁻", "0": "⁰", "1": "¹", "2": "²", "3": "³", "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹" };
  function sup(n) { return String(Math.round(n)).split("").map(ch => SUP[ch] || ch).join(""); }

  function line(ax, xs, ys, color, w = 2, dash) {
    const { ctx, X, Y } = ax; ctx.save(); ctx.strokeStyle = color; ctx.lineWidth = w; if (dash) ctx.setLineDash(dash);
    ctx.beginPath(); let pen = false;
    for (let i = 0; i < xs.length; i++) { const y = ys[i]; if (!isFinite(y)) { pen = false; continue; } const px = X(xs[i]), py = Y(y); pen ? ctx.lineTo(px, py) : ctx.moveTo(px, py); pen = true; }
    ctx.stroke(); ctx.restore();
  }
  function sticks(ax, xs, ys, color, w = 2) {
    const { ctx, X, Y } = ax; ctx.save(); ctx.strokeStyle = color; ctx.lineWidth = w; ctx.beginPath();
    xs.forEach((x, i) => { ctx.moveTo(X(x), Y(0)); ctx.lineTo(X(x), Y(ys[i])); }); ctx.stroke(); ctx.restore();
  }
  function label(ax, x, y, txt, color = "#24231f", align = "center", base = "bottom", dy = -3) {
    const { ctx, X, Y } = ax; ctx.save(); ctx.fillStyle = color; ctx.font = "12px system-ui,sans-serif"; ctx.textAlign = align; ctx.textBaseline = base;
    ctx.fillText(txt, X(x), Y(y) + dy); ctx.restore();
  }
  function legend(ax, items, x0) {
    const { ctx, m, W } = ax; ctx.save(); ctx.font = "12px system-ui,sans-serif"; ctx.textBaseline = "middle"; ctx.textAlign = "left";
    let x = x0 ?? (m.l + 10), y = m.t + 10;
    items.forEach(([txt, col, dash]) => {
      const w = ctx.measureText(txt).width + 34;
      if (x + w > W - m.r) { x = x0 ?? (m.l + 10); y += 17; }
      ctx.strokeStyle = col; ctx.lineWidth = 2.5; ctx.setLineDash(dash || []); ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x + 20, y); ctx.stroke(); ctx.setLineDash([]);
      ctx.fillStyle = "#24231f"; ctx.fillText(txt, x + 25, y); x += w;
    });
    ctx.restore();
  }
  /** Dash patterns for the 2nd, 3rd... series of a chart: the series differ by more than the colour (WCAG 1.4.1) */
  const DASH = [null, [7, 4], [2, 3], [8, 3, 2, 3]];
  const fmt = (v, d = 2) => (+v).toLocaleString("it-IT", { minimumFractionDigits: d, maximumFractionDigits: d });
  const sci = (v, d = 2) => { if (v === 0) return "0"; const e = Math.floor(Math.log10(Math.abs(v))); const mnt = v / Math.pow(10, e); return fmt(mnt, d) + "·10" + sup(e); };


  // ---------------------------------------------------------------- glossary hints
  // Every glossary term (glossario-dati.js, generated from 21-glossario.html by tools/genera_glossario.py) found in the text gets a dotted
  // underline; the mouse over it (or focus / tap) shows the short definition and a link to its chapter.
  function glossary() {
    if (here === "21-glossario.html") return;
    const s = document.createElement("script"); s.src = "glossario-dati.js"; s.onload = () => { try { markTerms(GLOSSARIO); } catch (_) { /* hints are optional */ } };
    document.head.appendChild(s);
  }

  function markTerms(G) {
    const cs = new Map(), ci = new Map();
    G.forEach((e, i) => e.k.forEach(k => {
      const strict = k.length <= 3 || (k.match(/[A-ZÀ-Ý]/g) || []).length >= 2;
      (strict ? cs : ci).set(strict ? k : k.toLowerCase(), i);
    }));
    const esc = x => x.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const all = [...cs.keys(), ...ci.keys()].sort((a, b) => b.length - a.length);
    const re = new RegExp("(?<![\\p{L}\\p{N}_])(" + all.map(esc).join("|") + ")(?![\\p{L}\\p{N}_])", "giu");
    const skip = "a,button,select,input,textarea,script,style,canvas,svg,code,kbd,h1,h2,h3,h4,h5,.kicker,.ctl,.pn,#side,#top,.tag,.g,sub,sup";
    const nodes = [], tw = document.createTreeWalker($("main"), NodeFilter.SHOW_TEXT);
    for (let n = tw.nextNode(); n; n = tw.nextNode()) if (n.nodeValue.trim() && !n.parentElement.closest(skip)) nodes.push(n);
    nodes.forEach(n => {
      const txt = n.nodeValue; let last = 0, hit = false; const frag = document.createDocumentFragment();
      for (const m of txt.matchAll(re)) {
        const i = cs.has(m[1]) ? cs.get(m[1]) : ci.get(m[1].toLowerCase());
        if (i === undefined) continue;
        frag.appendChild(document.createTextNode(txt.slice(last, m.index)));
        const sp = document.createElement("span"); sp.className = "g"; sp.tabIndex = 0; sp.dataset.g = i; sp.textContent = m[1];
        frag.appendChild(sp); last = m.index + m[1].length; hit = true;
      }
      if (hit) { frag.appendChild(document.createTextNode(txt.slice(last))); n.parentNode.replaceChild(frag, n); }
    });
    const tip = document.createElement("div"); tip.id = "gtip"; tip.setAttribute("role", "tooltip"); tip.hidden = true; document.body.appendChild(tip);
    let timer = 0, cur = null;
    const hide = () => { tip.hidden = true; cur = null; };
    const show = el => {
      clearTimeout(timer); if (cur === el) return; cur = el;
      const e = G[+el.dataset.g];
      tip.innerHTML = `<b>${e.t}</b><p>${e.d}</p>` + (e.h ? `<a href="${e.h}">${e.n} &rarr;</a>` : "");
      tip.hidden = false;
      const r = el.getBoundingClientRect(), w = tip.offsetWidth, h = tip.offsetHeight;
      const x = Math.max(8, Math.min(r.left, innerWidth - w - 8)), below = r.bottom + 6 + h < innerHeight || r.top - 6 - h < 0;
      tip.style.left = x + "px"; tip.style.top = (below ? r.bottom + 6 : r.top - 6 - h) + "px";
    };
    const later = () => { clearTimeout(timer); timer = setTimeout(hide, 220); };
    document.addEventListener("mouseover", ev => { const el = ev.target.closest(".g"); if (el) show(el); else if (ev.target.closest("#gtip")) clearTimeout(timer); });
    document.addEventListener("mouseout", ev => { if (ev.target.closest(".g, #gtip")) later(); });
    document.addEventListener("focusin", ev => { const el = ev.target.closest(".g"); if (el) show(el); });
    document.addEventListener("focusout", ev => { if (ev.target.closest(".g")) later(); });
    document.addEventListener("click", ev => { const el = ev.target.closest(".g"); if (el) { cur === el && !tip.hidden ? hide() : show(el); } else if (!ev.target.closest("#gtip")) hide(); });
    document.addEventListener("keydown", ev => { if (ev.key === "Escape") hide(); });
    addEventListener("scroll", hide, { passive: true });
  }

  // The page scripts are added in this order (async=false keeps it): the bar of commands first, then the settings button that goes into it
  function load(src) { const s = document.createElement("script"); s.src = src; s.async = false; document.head.appendChild(s); }
  // comandi.js: TCMD, the commands of the header; a11y.js: «Lettura facilitata» (tema.js has already put the choices on <html>);
  // lettura.js: menu sheet, reading position, figure viewer; cerca.js: search dialog (Ctrl/⌘K, TEORIA.cerca(), index read on first use);
  // strumenti/pannello.js: TEORIA.strumenti.* (each tool is read when opened)
  document.addEventListener("DOMContentLoaded", () => { layout(); glossary(); ["comandi.js", "a11y.js", "lettura.js", "cerca.js", "strumenti/pannello.js"].forEach(load); });
  return { reduced: () => document.documentElement.hasAttribute("data-a11y-reduce"), $, controls, buttons, canvas, axes, line, sticks, label, legend, nice, DASH, fmt, sci, sup };
})();
