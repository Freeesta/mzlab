/* mzLab - Teoria: the commands of the page header (TCMD). One list (id, icon, label, run) feeds the bar of the standalone Teoria
   and, from the program, the bar of the «Teoria» tab (barra.js reads window.TCMD of the iframe). Classic script, no network:
   it works from file:// too. Only navigation and tools: nothing here changes the text of a chapter. */
"use strict";
(function () {
  const $ = (q, el = document) => el.querySelector(q);
  const embedded = window.self !== window.top;
  const here = location.pathname.split("/").pop() || "index.html";
  const CH = typeof CHAPTERS !== "undefined" ? CHAPTERS : [];
  const idx = Math.max(0, CH.findIndex(c => c[0] === here));
  const mac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent || "");
  const T = () => (window.TEORIA = window.TEORIA || {});

  const ICON = {
    prec: '<path d="M12.5 4 6.5 10l6 6" />',
    succ: '<path d="M7.5 4l6 6-6 6" />',
    cerca: '<circle cx="9" cy="9" r="5.2" /><path d="M13 13l4 4" />',
    glossario: '<path d="M4.5 3.5h8a2 2 0 0 1 2 2v11h-8a2 2 0 0 1-2-2v-11z" /><path d="M6.5 7h6M6.5 10h6" />',
    formule: '<path d="M14.5 4.5h-8l5 5.5-5 5.5h8" />',
    esercizi: '<path d="M4 10.5l4 4 8-9" />',
    strumenti: '<path d="M11.5 3.5a3.7 3.7 0 0 0-3.2 5.3L3.5 13.6a1.6 1.6 0 0 0 2.3 2.3l4.8-4.8a3.7 3.7 0 0 0 5.3-3.2l-2.3 2.3-2-.5-.5-2z" />',
    impostazioni: '<circle cx="10" cy="10" r="2.6" /><path d="M10 2.8v2M10 15.2v2M2.8 10h2M15.2 10h2M4.9 4.9l1.4 1.4M13.7 13.7l1.4 1.4M4.9 15.1l1.4-1.4M13.7 6.3l1.4-1.4" />',
    altro: '<circle cx="4.5" cy="10" r="1.2" /><circle cx="10" cy="10" r="1.2" /><circle cx="15.5" cy="10" r="1.2" />',
  };
  const svg = k => `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICON[k]}</svg>`;

  // ---------------------------------------------------------------- the commands
  const go = file => { location.href = new URL(file, document.baseURI).href; };      // absolute: the program's bar calls this from its own page, which would resolve a relative address by itself
  /** Where the exercises of this chapter are: the first «Allenati» box, else the «Verifica» section; else the Pratica index. */
  function esercizi() {
    if (/^pratica/.test(here)) { const h = $("main h2, main h1"); if (h) { h.tabIndex = -1; h.scrollIntoView({ block: "start" }); h.focus({ preventScroll: true }); } return; }
    const t = $("main .box.gym") || [...document.querySelectorAll("main h2")].find(h => /^Verifica/i.test(h.textContent.trim()));
    if (!t) return go("pratica.html");
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches || document.documentElement.hasAttribute("data-a11y-reduce");
    t.scrollIntoView({ block: "start", behavior: reduced ? "auto" : "smooth" });
    if (!t.hasAttribute("tabindex")) t.tabIndex = -1;
    t.focus({ preventScroll: true });
  }
  const strum = (nome, sel) => async () => {
    const S = T().strumenti; if (!S) return;
    await S[nome]();
    if (sel) { const e = document.querySelector("#strum " + sel); if (e) e.scrollIntoView({ block: "start" }); }
  };
  const prevC = CH[idx - 1], nextC = CH[idx + 1];
  const nm = c => (/^[0-9]+$/.test(c[1]) && c[1] !== "0" ? c[1] + ". " : "") + c[2];

  const TCMD = {
    lista: [
      { id: "prec", icona: "prec", etichetta: "Precedente", titolo: prevC ? "Precedente: " + nm(prevC) : "", gruppo: "nav", href: prevC && prevC[0], disabilitato: !prevC, run: () => prevC && go(prevC[0]) },
      { id: "succ", icona: "succ", etichetta: "Successivo", titolo: nextC ? "Successivo: " + nm(nextC) : "", gruppo: "nav", href: nextC && nextC[0], disabilitato: !nextC, run: () => nextC && go(nextC[0]) },
      { id: "cerca", icona: "cerca", etichetta: "Cerca", titolo: "Cerca nella Teoria (" + (mac ? "⌘K" : "Ctrl+K") + ")", gruppo: "uso", chiave: mac ? "⌘K" : "Ctrl K", run: () => T().cerca && T().cerca() },
      { id: "glossario", icona: "glossario", etichetta: "Glossario", titolo: "Glossario e bibliografia", gruppo: "uso", href: "21-glossario.html", run: () => go("21-glossario.html") },
      { id: "formule", icona: "formule", etichetta: "Formule", titolo: "Formulario per l'orale", gruppo: "uso", href: "22-formulario.html", run: () => go("22-formulario.html") },
      { id: "esercizi", icona: "esercizi", etichetta: "Esercizi", titolo: "Esercizi di questo capitolo e Pratica", gruppo: "uso", run: esercizi },
      { id: "strumenti", icona: "strumenti", etichetta: "Strumenti", titolo: "Strumenti didattici", gruppo: "uso", figli: [
        { id: "frammenta", etichetta: "Frammenta", run: strum("frammenta", "#fr-rrk") },
        { id: "massa", etichetta: "Massa e addotti", run: strum("massa") },
        { id: "tavola", etichetta: "Tavola periodica", run: strum("tavola") },
        { id: "perdite", etichetta: "Perdite", run: strum("frammenta", "#fr-loss") },
      ] },
      { id: "impostazioni", icona: "impostazioni", etichetta: "Impostazioni", titolo: "Impostazioni di lettura", gruppo: "fine", run: () => { const b = $("#a11y-btn"); if (b) b.click(); } },
    ],
    icona: svg,
    per: id => TCMD.lista.find(c => c.id === id),
  };
  window.TCMD = TCMD;
  T().settings = () => { const b = $("#a11y-btn"); if (b) b.click(); };      // the gear of the program opens the settings of the Teoria
  if (embedded) { document.dispatchEvent(new Event("tcmd-pronto")); return; }   // in the program the bar of the app draws them

  // ---------------------------------------------------------------- the bar of the standalone page
  const top = $("#top"); if (!top) return;
  const phone = document.documentElement.classList.contains("phone");
  const bar = document.createElement("div"); bar.id = "tbar"; bar.setAttribute("role", "toolbar"); bar.setAttribute("aria-label", "Comandi della Teoria");
  const btn = c => {
    const el = document.createElement(c.href && !c.disabilitato ? "a" : "button");
    if (el.tagName === "BUTTON") el.type = "button"; else el.href = c.href;
    el.className = "tb"; el.id = "tc-" + c.id; el.dataset.cmd = c.id;
    el.innerHTML = svg(c.icona) + `<span class="l">${c.etichetta}</span>` + (c.figli ? '<span class="l car" aria-hidden="true">▾</span>' : "");
    el.setAttribute("aria-label", c.etichetta + (c.chiave ? " (" + c.chiave + ")" : "")); el.title = c.titolo || c.etichetta;
    if (c.disabilitato) { el.setAttribute("aria-disabled", "true"); el.tabIndex = -1; }
    if (!c.figli && el.tagName === "BUTTON") el.onclick = () => c.run();
    return el;
  };
  const sep = () => { const s = document.createElement("span"); s.className = "tsep"; s.setAttribute("aria-hidden", "true"); return s; };

  // dropdown menu (Strumenti, and «Altro» on a phone): menu button, arrows, Home/End, Esc, focus given back
  function menu(el, items, label) {
    const pop = document.createElement("div"); pop.className = "tmenu"; pop.id = el.id + "-m"; pop.setAttribute("role", "menu"); pop.setAttribute("aria-label", label); pop.hidden = true;
    items.forEach(f => {
      const b = document.createElement("button"); b.type = "button"; b.setAttribute("role", "menuitem"); b.textContent = f.etichetta; b.dataset.cmd = f.id; b.tabIndex = -1;
      b.onclick = () => { close(false); f.run(); }; pop.appendChild(b);
    });
    el.setAttribute("aria-haspopup", "menu"); el.setAttribute("aria-expanded", "false"); el.setAttribute("aria-controls", pop.id);
    el.after(pop); el.parentNode.classList.add("hasmenu");
    const its = () => [...pop.querySelectorAll("[role=menuitem]")];
    function open(first) { pop.hidden = false; el.setAttribute("aria-expanded", "true"); const l = its(); (first === "last" ? l[l.length - 1] : l[0]).focus(); }
    function close(back = true) { if (pop.hidden) return; pop.hidden = true; el.setAttribute("aria-expanded", "false"); if (back) el.focus(); }
    el.onclick = () => (pop.hidden ? open() : close());
    el.addEventListener("keydown", e => { if (e.key === "ArrowDown") { e.preventDefault(); open(); } else if (e.key === "ArrowUp") { e.preventDefault(); open("last"); } });
    pop.addEventListener("keydown", e => {
      const l = its(), i = l.indexOf(document.activeElement);
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); }
      else if (e.key === "ArrowDown") { e.preventDefault(); l[(i + 1) % l.length].focus(); }
      else if (e.key === "ArrowUp") { e.preventDefault(); l[(i + l.length - 1) % l.length].focus(); }
      else if (e.key === "Home") { e.preventDefault(); l[0].focus(); }
      else if (e.key === "End") { e.preventDefault(); l[l.length - 1].focus(); }
      else if (e.key === "Tab") close(false);
    });
    document.addEventListener("click", e => { if (!pop.hidden && !pop.contains(e.target) && !el.contains(e.target)) close(false); });
  }

  const cmd = id => TCMD.per(id);
  const put = (id, parent = bar) => { const el = btn(cmd(id)); parent.appendChild(el); return el; };
  if (phone) {
    put("cerca");
    const al = btn({ id: "altro", icona: "altro", etichetta: "Altro", titolo: "Altro: strumenti didattici" }); al.className += " tb-altro"; bar.appendChild(al);
    // the gear is added by a11y.js (#a11y-btn) at the end of the bar
    menu(al, cmd("strumenti").figli, "Strumenti didattici");
  } else {
    const n = document.createElement("span"); n.className = "tgrp"; n.id = "tnav"; put("prec", n); put("succ", n); bar.appendChild(n); bar.appendChild(sep());
    ["cerca", "glossario", "formule", "esercizi"].forEach(id => put(id));
    const st = put("strumenti"); menu(st, cmd("strumenti").figli, "Strumenti didattici");
    bar.appendChild(sep());
  }
  const sp = $(".sp", top);   // logo · title · tabs · spacer · commands
  if (sp) { sp.remove(); top.appendChild(sp); }
  if (phone && $("#info", top)) $("#info", top).before(bar); else top.appendChild(bar);
  document.documentElement.classList.add("tbar-on");

  // when the row is too narrow the labels go first, then the title (the icons keep their name for screen readers)
  function fit() {
    bar.classList.remove("ico"); top.classList.remove("cmp");
    if (top.scrollWidth <= top.clientWidth) return;
    bar.classList.add("ico");
    if (top.scrollWidth > top.clientWidth) top.classList.add("cmp");
  }
  addEventListener("resize", fit); addEventListener("load", fit);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(fit);
  new MutationObserver(fit).observe(bar, { childList: true });
  fit();

  // narrow screens: previous / next sit in the fixed bar of lettura.js, next to the index button
  function cbar() {
    const cb = $("#cbar"); if (!cb || $("#cb-nav", cb)) return;
    const g = document.createElement("span"); g.id = "cb-nav";
    ["prec", "succ"].forEach(id => { const c = cmd(id), el = btn(c); el.id = "cb-" + id; g.appendChild(el); });
    cb.insertBefore(g, $("#prog", cb));
  }
  TCMD.cbar = cbar;
  cbar(); addEventListener("load", cbar);
})();
