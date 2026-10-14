/* mzLab - Teoria: in-page visual and HTML chapter editor. */
"use strict";

(function () {
  const SERVER_URL = "http://127.0.0.1:8765";
  const here = location.pathname.split("/").pop() || "index.html";
  if (!here.endsWith(".html") || here.startsWith("pratica-")) return;

  let isEditing = false, viewMode = "visual", originalDocHtml = "", barEl = null, codeWrapEl = null, codeTextarea = null;

  function init() {
    if (!window.QADMIN || !QADMIN.on()) return;   // the editor belongs to the administrator mode: no button, no shortcut otherwise
    window.addEventListener("keydown", e => {
      if (e.altKey && (e.key === "e" || e.key === "E" || e.code === "KeyE")) {
        e.preventDefault(); toggleEditor();
      }
    });
    insertBarButton();
  }

  function insertBarButton() {
    const tbar = document.getElementById("tbar");
    if (!tbar) { window.addEventListener("load", insertBarButton, { once: true }); return; }
    if (document.getElementById("tc-editor")) return;
    const btn = document.createElement("button");
    btn.type = "button"; btn.className = "tb"; btn.id = "tc-editor";
    btn.innerHTML = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M13.5 3.5l3 3L6.5 16.5l-4 1 1-4L13.5 3.5z"/><path d="M11.5 5.5l3 3"/></svg><span class="l">Modifica</span>`;
    btn.title = "Modifica capitolo (Alt+E)"; btn.onclick = toggleEditor;
    const sBtn = document.getElementById("a11y-btn") || document.getElementById("tc-impostazioni");
    if (sBtn && sBtn.parentNode === tbar) tbar.insertBefore(btn, sBtn); else tbar.appendChild(btn);
  }

  async function toggleEditor() {
    if (isEditing) exitEditor(); else await enterEditor();
  }

  async function enterEditor() {
    showToast("Apertura editor...", "info", 1500);
    try {
      const res = await fetch(`${SERVER_URL}/carica?file=${encodeURIComponent(here)}`);
      if (res.ok) { const d = await res.json(); if (d.ok && d.html) originalDocHtml = d.html; }
    } catch (_) {}
    if (!originalDocHtml) originalDocHtml = buildFullDocumentHtml();
    isEditing = true;
    document.body.classList.add("tedit-active");
    const eb = document.getElementById("tc-editor");
    if (eb) { eb.classList.add("on"); eb.setAttribute("aria-pressed", "true"); }
    createEditorBar();
    enableVisualEditing();
    showToast(`Modalità modifica attiva per ${here}.`, "info", 2500);
  }

  function exitEditor(force = false) {
    if (!force && !confirm("Vuoi uscire dall'editor? Le modifiche non salvate andranno perse.")) return;
    isEditing = false;
    document.body.classList.remove("tedit-active");
    const eb = document.getElementById("tc-editor");
    if (eb) { eb.classList.remove("on"); eb.setAttribute("aria-pressed", "false"); }
    disableVisualEditing();
    if (barEl) { barEl.remove(); barEl = null; }
    if (codeWrapEl) { codeWrapEl.remove(); codeWrapEl = null; }
  }

  function createEditorBar() {
    if (barEl) barEl.remove();
    barEl = document.createElement("div");
    barEl.id = "tedit-bar"; barEl.setAttribute("role", "toolbar");
    barEl.innerHTML = `
      <div class="tedit-left">
        <span class="tedit-badge">Capitolo: <b>${here}</b></span>
        <div class="tedit-grp">
          <button type="button" class="tedit-btn" data-act="bold"><b>B</b></button>
          <button type="button" class="tedit-btn" data-act="italic"><i>I</i></button>
          <button type="button" class="tedit-btn" data-act="sub">X<sub>2</sub></button>
          <button type="button" class="tedit-btn" data-act="sup">X<sup>2</sup></button>
          <button type="button" class="tedit-btn" data-act="mz"><i>m/z</i></button>
          <button type="button" class="tedit-btn" data-act="link">🔗</button>
        </div>
        <div class="tedit-grp">
          <button type="button" class="tedit-btn" data-act="h2">H2</button>
          <button type="button" class="tedit-btn" data-act="h3">H3</button>
          <button type="button" class="tedit-btn" data-act="p">P</button>
          <button type="button" class="tedit-btn" data-act="ul">• Lista</button>
          <button type="button" class="tedit-btn" data-act="ol">1. Lista</button>
        </div>
        <div class="tedit-grp">
          <select class="tedit-sel" id="tedit-box-sel">
            <option value="">+ Inserisci box...</option>
            <option value="key">🔑 Concetto chiave</option>
            <option value="warn">⚠️ Attenzione</option>
            <option value="lab">🔬 Laboratorio</option>
            <option value="eq">📐 Formula</option>
            <option value="q">❓ Domanda</option>
          </select>
        </div>
        <div class="tedit-grp">
          <button type="button" class="tedit-btn active" id="tedit-btn-visual">👁️ Visuale</button>
          <button type="button" class="tedit-btn" id="tedit-btn-html">💻 Codice HTML</button>
        </div>
      </div>
      <div class="tedit-right">
        <button type="button" class="tedit-save" id="tedit-btn-save">💾 Salva e pubblica</button>
        <button type="button" class="tedit-close" id="tedit-btn-close">✕ Chiudi</button>
      </div>`;
    const top = document.getElementById("top");
    if (top) top.after(barEl); else document.body.prepend(barEl);
    attachBarEvents();
  }

  function attachBarEvents() {
    barEl.querySelectorAll(".tedit-btn[data-act]").forEach(b => {
      b.onclick = e => { e.preventDefault(); handleFormatting(b.dataset.act); };
    });
    const bSel = document.getElementById("tedit-box-sel");
    bSel.onchange = () => { if (bSel.value) { insertBox(bSel.value); bSel.value = ""; } };
    document.getElementById("tedit-btn-visual").onclick = () => switchMode("visual");
    document.getElementById("tedit-btn-html").onclick = () => switchMode("html");
    document.getElementById("tedit-btn-save").onclick = salvaCapitolo;
    document.getElementById("tedit-btn-close").onclick = () => exitEditor(false);
  }

  function handleFormatting(act) {
    if (viewMode !== "visual") return;
    if (act === "mz") { insertHtmlAtCursor("<i>m/z</i>"); return; }
    if (act === "link") {
      const u = prompt("URL link:", ""); if (u) document.execCommand("createLink", false, u); return;
    }
    const map = { bold: ["bold"], italic: ["italic"], sub: ["subscript"], sup: ["superscript"], h2: ["formatBlock", "<h2>"], h3: ["formatBlock", "<h3>"], p: ["formatBlock", "<p>"], ul: ["insertUnorderedList"], ol: ["insertOrderedList"] };
    if (map[act]) document.execCommand(map[act][0], false, map[act][1] || null);
  }

  function insertBox(type) {
    if (viewMode !== "visual") return;
    const tpls = {
      key: '<div class="box key"><b>Concetto chiave</b><p>Testo del concetto fondamentale...</p></div><p></p>',
      warn: '<div class="box warn"><b>Attenzione</b><p>Avviso o trappola da evitare...</p></div><p></p>',
      lab: '<div class="box lab"><b>In laboratorio</b><p>Dettaglio pratico o strumentale...</p></div><p></p>',
      eq: '<span class="eq"><i>Formula</i> = <span class="frac"><span><i>Num</i></span><span><i>Den</i></span></span><span class="no">(eq)</span></span><p></p>',
      q: '<details class="q"><summary>Domanda di verifica?</summary><p>Risposta alla domanda.</p></details><p></p>'
    };
    if (tpls[type]) insertHtmlAtCursor(tpls[type]);
  }

  function insertHtmlAtCursor(html) {
    const sel = window.getSelection();
    if (!sel || !sel.rangeCount) { document.querySelector("main")?.insertAdjacentHTML("beforeend", html); return; }
    const range = sel.getRangeAt(0); range.deleteContents();
    const el = document.createElement("div"); el.innerHTML = html;
    const frag = document.createDocumentFragment();
    let node, last;
    while ((node = el.firstChild)) last = frag.appendChild(node);
    range.insertNode(frag);
    if (last) { range.setStartAfter(last); range.collapse(true); sel.removeAllRanges(); sel.addRange(range); }
  }

  function enableVisualEditing() {
    const mn = document.querySelector("main"); if (!mn) return;
    mn.setAttribute("contenteditable", "true"); mn.setAttribute("spellcheck", "false");
    mn.querySelectorAll(".sim, .pn, canvas, svg, .ctl, .btns").forEach(e => e.setAttribute("contenteditable", "false"));
    const pn = mn.querySelector(".pn"); if (pn) pn.style.display = "none";
    mn.addEventListener("click", preventLinkNav);
  }

  function disableVisualEditing() {
    const mn = document.querySelector("main"); if (!mn) return;
    mn.removeAttribute("contenteditable"); mn.removeAttribute("spellcheck");
    mn.querySelectorAll("[contenteditable]").forEach(e => e.removeAttribute("contenteditable"));
    const pn = mn.querySelector(".pn"); if (pn) pn.style.display = "";
    mn.removeEventListener("click", preventLinkNav);
  }

  function preventLinkNav(e) {
    if (e.target.closest("a") && isEditing && viewMode === "visual") e.preventDefault();
  }

  function switchMode(newMode) {
    if (newMode === viewMode) return;
    viewMode = newMode;
    document.getElementById("tedit-btn-visual")?.classList.toggle("active", newMode === "visual");
    document.getElementById("tedit-btn-html")?.classList.toggle("active", newMode === "html");
    const mn = document.querySelector("main"); if (!mn) return;

    if (newMode === "html") {
      const full = buildUpdatedDocumentHtml();
      disableVisualEditing(); mn.style.display = "none";
      if (!codeWrapEl) {
        codeWrapEl = document.createElement("div"); codeWrapEl.id = "tedit-code-wrap";
        codeWrapEl.innerHTML = `<div class="tedit-code-header"><span>Sorgente HTML: <code>${here}</code></span></div><textarea id="tedit-code" spellcheck="false"></textarea>`;
        mn.after(codeWrapEl); codeTextarea = document.getElementById("tedit-code");
      }
      codeWrapEl.style.display = "flex"; codeTextarea.value = full; codeTextarea.focus();
    } else {
      if (codeTextarea) {
        originalDocHtml = codeTextarea.value;
        const m = originalDocHtml.match(/<main[^>]*>([\s\S]*?)<\/main>/i);
        if (m) mn.innerHTML = m[1];
      }
      if (codeWrapEl) codeWrapEl.style.display = "none";
      mn.style.display = ""; enableVisualEditing();
    }
  }

  function getCleanMainHtml() {
    const mn = document.querySelector("main"); if (!mn) return "";
    const cl = mn.cloneNode(true);
    cl.querySelectorAll(".pn").forEach(e => e.remove());
    cl.querySelectorAll(".tw").forEach(tw => { const t = tw.querySelector("table"); if (t) tw.replaceWith(t); else tw.remove(); });
    cl.querySelectorAll(".g").forEach(sp => sp.replaceWith(document.createTextNode(sp.textContent)));
    cl.querySelectorAll("[aria-level], [contenteditable]").forEach(e => { e.removeAttribute("aria-level"); e.removeAttribute("contenteditable"); });
    cl.querySelectorAll("svg[role=img][aria-labelledby]").forEach(svg => {
      const prev = svg.previousElementSibling;
      if (prev?.id && /^fig-cap-\d+$/.test(prev.id)) prev.removeAttribute("id");
      svg.removeAttribute("role"); svg.removeAttribute("aria-labelledby");
    });
    return cl.innerHTML;
  }

  function buildUpdatedDocumentHtml() {
    if (viewMode === "html" && codeTextarea) return codeTextarea.value;
    const clean = getCleanMainHtml();
    if (!originalDocHtml) originalDocHtml = buildFullDocumentHtml();
    if (/<main[^>]*>[\s\S]*?<\/main>/i.test(originalDocHtml)) {
      return originalDocHtml.replace(/(<main[^>]*>)[\s\S]*?(<\/main>)/i, `$1\n${clean}\n$2`);
    }
    return originalDocHtml;
  }

  function buildFullDocumentHtml() {
    return `<!doctype html>\n<html lang="it"><head><meta charset="utf-8"><title>${document.title || "Teoria"}</title><link rel="stylesheet" href="teoria.css"></head>\n<body><div class="wrap"><nav id="side"></nav><main>\n${getCleanMainHtml()}\n</main></div><script src="../appname.js"></script><script src="teoria.js"></script></body></html>`;
  }

  async function salvaCapitolo() {
    const btn = document.getElementById("tedit-btn-save");
    if (btn) { btn.disabled = true; btn.textContent = "⏳ Salvataggio..."; }
    const finalHtml = buildUpdatedDocumentHtml();
    try {
      showToast("Invio e controllo...", "info", 8000);
      const res = await fetch(`${SERVER_URL}/salva`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file: here, html: finalHtml })
      });
      const data = await res.json();
      if (res.ok && data.ok) {
        showToast("✅ Salvato e pubblicato su GitHub!", "ok", 4000);
        setTimeout(() => location.reload(), 1500);
      } else {
        const msg = data.error || data.warning || "Errore sconosciuto";
        showToast("❌ Errore:\n" + msg, "err", 8000);
        showFallbackModal(finalHtml, msg);
      }
    } catch (e) {
      showToast("⚠️ Server non raggiungibile", "err", 5000);
      showFallbackModal(finalHtml, e.message);
    } finally {
      if (btn) { btn.disabled = false; btn.textContent = "💾 Salva e pubblica"; }
    }
  }

  function showFallbackModal(html, err) {
    document.getElementById("tedit-modal")?.remove();
    const m = document.createElement("div"); m.id = "tedit-modal";
    m.innerHTML = `<div class="tedit-modal-card"><h3>Salvataggio</h3><p class="tedit-modal-err">${err}</p><p>Avvia: <code>python3 tools/server_teoria.py</code> nel terminale.</p><div class="tedit-modal-actions"><button type="button" class="tedit-btn" id="tedit-dl">📥 Scarica file</button><button type="button" class="tedit-close" id="tedit-mc">Chiudi</button></div></div>`;
    document.body.appendChild(m);
    document.getElementById("tedit-mc").onclick = () => m.remove();
    document.getElementById("tedit-dl").onclick = () => {
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([html], { type: "text/html;charset=utf-8" }));
      a.download = here; a.click();
    };
  }

  let toastTimer = null;
  function showToast(txt, type = "info", ms = 3000) {
    let t = document.getElementById("tedit-toast");
    if (!t) { t = document.createElement("div"); t.id = "tedit-toast"; document.body.appendChild(t); }
    clearTimeout(toastTimer); t.className = type; t.textContent = txt; t.style.display = "block";
    toastTimer = setTimeout(() => { if (t) t.style.display = "none"; }, ms);
  }

  window.TEORIA_EDITOR = {
    toggle: toggleEditor,
    salva: salvaCapitolo,
    attiva: () => {
      insertBarButton();
    }
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init); else init();
})();
