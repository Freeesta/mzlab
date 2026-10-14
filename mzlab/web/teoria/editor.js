/* mzLab - Teoria: in-page visual and HTML chapter editor.
   Permette a docenti e studenti di modificare i capitoli direttamente dal browser:
   - Formattazione visuale (Grassetto, Corsivo, Pedice/Apice, m/z, Titoli H2/H3, Elenchi, Box mzLab)
   - Vista codice sorgente HTML completo
   - Salvataggio diretto su server locale (http://127.0.0.1:8765) con git commit e git push automatici
   - Fallback offline con esportazione/copia file.
*/
"use strict";

(function () {
  const SERVER_URL = "http://127.0.0.1:8765";
  const here = location.pathname.split("/").pop() || "index.html";

  // Solo i file .html della teoria (esclusi giochi pratici o indici speciali se non desiderato)
  if (!here.endsWith(".html") || here.startsWith("pratica-")) {
    return;
  }

  let isEditing = false;
  let viewMode = "visual"; // "visual" | "html"
  let originalDocHtml = "";
  let barEl = null;
  let codeWrapEl = null;
  let codeTextarea = null;

  // Inserimento icona o shortcut Alt+E
  function init() {
    setupShortcut();
    insertBarButton();
  }

  function setupShortcut() {
    window.addEventListener("keydown", (e) => {
      // Alt+E o Option+E
      if (e.altKey && (e.key === "e" || e.key === "E" || e.code === "KeyE")) {
        e.preventDefault();
        toggleEditor();
      }
    });
  }

  function insertBarButton() {
    const tbar = document.getElementById("tbar");
    if (!tbar) {
      // Se tbar non è ancora nel DOM, riprova al caricamento
      window.addEventListener("load", insertBarButton, { once: true });
      return;
    }
    if (document.getElementById("tc-editor")) return;

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "tb";
    btn.id = "tc-editor";
    btn.innerHTML = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M13.5 3.5l3 3L6.5 16.5l-4 1 1-4L13.5 3.5z"/><path d="M11.5 5.5l3 3"/></svg><span class="l">Modifica</span>`;
    btn.title = "Modifica questo capitolo (Alt+E)";
    btn.setAttribute("aria-label", "Modifica capitolo (Alt+E)");
    btn.onclick = toggleEditor;

    // Inserisci prima di impostazioni se presente, altrimenti in fondo
    const settingsBtn = document.getElementById("a11y-btn") || document.getElementById("tc-impostazioni");
    if (settingsBtn && settingsBtn.parentNode === tbar) {
      tbar.insertBefore(btn, settingsBtn);
    } else {
      tbar.appendChild(btn);
    }
  }

  async function toggleEditor() {
    if (isEditing) {
      exitEditor();
    } else {
      await enterEditor();
    }
  }

  async function enterEditor() {
    showToast("Apertura editor...", "info", 1500);

    // Carica il sorgente originale dal server locale se disponibile
    try {
      const res = await fetch(`${SERVER_URL}/carica?file=${encodeURIComponent(here)}`);
      if (res.ok) {
        const data = await res.json();
        if (data.ok && data.html) {
          originalDocHtml = data.html;
        }
      }
    } catch (_) {
      // Server non attivo: useremo il template ricostruito dal DOM
    }

    if (!originalDocHtml) {
      originalDocHtml = buildFullDocumentHtml();
    }

    isEditing = true;
    document.body.classList.add("tedit-active");

    const editBtn = document.getElementById("tc-editor");
    if (editBtn) {
      editBtn.classList.add("on");
      editBtn.setAttribute("aria-pressed", "true");
    }

    createEditorBar();
    enableVisualEditing();
    showToast(`Modalità modifica attiva per ${here}. Premi Salva e pubblica al termine.`, "info", 3000);
  }

  function exitEditor(force = false) {
    if (!force) {
      const confirmExit = confirm("Vuoi uscire dall'editor? Le modifiche non salvate andranno perse.");
      if (!confirmExit) return;
    }

    isEditing = false;
    document.body.classList.remove("tedit-active");

    const editBtn = document.getElementById("tc-editor");
    if (editBtn) {
      editBtn.classList.remove("on");
      editBtn.setAttribute("aria-pressed", "false");
    }

    disableVisualEditing();

    if (barEl) {
      barEl.remove();
      barEl = null;
    }
    if (codeWrapEl) {
      codeWrapEl.remove();
      codeWrapEl = null;
    }
  }

  function createEditorBar() {
    if (barEl) barEl.remove();

    barEl = document.createElement("div");
    barEl.id = "tedit-bar";
    barEl.setAttribute("role", "toolbar");
    barEl.setAttribute("aria-label", "Barra degli strumenti di modifica");

    barEl.innerHTML = `
      <div class="tedit-left">
        <span class="tedit-badge">Capitolo: <b>${here}</b></span>

        <div class="tedit-grp" title="Formattazione testo">
          <button type="button" class="tedit-btn" data-act="bold" title="Grassetto (Ctrl+B)"><b>B</b></button>
          <button type="button" class="tedit-btn" data-act="italic" title="Corsivo (Ctrl+I)"><i>I</i></button>
          <button type="button" class="tedit-btn" data-act="sub" title="Pedice chimico (es. H₂O)">X<sub>2</sub></button>
          <button type="button" class="tedit-btn" data-act="sup" title="Apice / carica (es. 10⁻⁵, M⁺)">X<sup>2</sup></button>
          <button type="button" class="tedit-btn" data-act="mz" title="Inserisci m/z in corsivo"><i>m/z</i></button>
          <button type="button" class="tedit-btn" data-act="link" title="Inserisci link">🔗</button>
        </div>

        <div class="tedit-grp" title="Struttura e titoli">
          <button type="button" class="tedit-btn" data-act="h2" title="Titolo sezione (H2)">H2</button>
          <button type="button" class="tedit-btn" data-act="h3" title="Sottotitolo (H3)">H3</button>
          <button type="button" class="tedit-btn" data-act="p" title="Paragrafo normale">P</button>
          <button type="button" class="tedit-btn" data-act="ul" title="Elenco puntato">• Lista</button>
          <button type="button" class="tedit-btn" data-act="ol" title="Elenco numerato">1. Lista</button>
        </div>

        <div class="tedit-grp" title="Box didattici di mzLab">
          <select class="tedit-sel" id="tedit-box-sel" aria-label="Inserisci box didattico">
            <option value="">+ Inserisci box...</option>
            <option value="key">🔑 Concetto chiave</option>
            <option value="warn">⚠️ Attenzione / Trappola</option>
            <option value="lab">🔬 In laboratorio</option>
            <option value="eq">📐 Formula matematica</option>
            <option value="q">❓ Domanda d'esame</option>
          </select>
        </div>

        <div class="tedit-grp" title="Modalità di visualizzazione">
          <button type="button" class="tedit-btn ${viewMode === 'visual' ? 'active' : ''}" id="tedit-btn-visual" title="Modifica visiva diretta">👁️ Visuale</button>
          <button type="button" class="tedit-btn ${viewMode === 'html' ? 'active' : ''}" id="tedit-btn-html" title="Visualizza e modifica codice sorgente HTML">💻 Codice HTML</button>
        </div>
      </div>

      <div class="tedit-right">
        <button type="button" class="tedit-save" id="tedit-btn-save" title="Salva, verifica ed esegui commit/push su GitHub">💾 Salva e pubblica</button>
        <button type="button" class="tedit-close" id="tedit-btn-close" title="Chiudi l'editor">✕ Chiudi</button>
      </div>
    `;

    // Posiziona sotto l'header principale #top
    const top = document.getElementById("top");
    if (top) {
      top.after(barEl);
    } else {
      document.body.prepend(barEl);
    }

    // Event listeners
    barEl.querySelectorAll(".tedit-btn[data-act]").forEach(b => {
      b.onclick = (e) => {
        e.preventDefault();
        handleFormatting(b.dataset.act);
      };
    });

    const boxSel = document.getElementById("tedit-box-sel");
    boxSel.onchange = () => {
      if (boxSel.value) {
        insertBox(boxSel.value);
        boxSel.value = "";
      }
    };

    document.getElementById("tedit-btn-visual").onclick = () => switchMode("visual");
    document.getElementById("tedit-btn-html").onclick = () => switchMode("html");
    document.getElementById("tedit-btn-save").onclick = salvaCapitolo;
    document.getElementById("tedit-btn-close").onclick = () => exitEditor(false);
  }

  function handleFormatting(action) {
    if (viewMode !== "visual") return;
    switch (action) {
      case "bold":
        document.execCommand("bold", false, null);
        break;
      case "italic":
        document.execCommand("italic", false, null);
        break;
      case "sub":
        document.execCommand("subscript", false, null);
        break;
      case "sup":
        document.execCommand("superscript", false, null);
        break;
      case "mz":
        insertHtmlAtCursor("<i>m/z</i>");
        break;
      case "link": {
        const url = prompt("Inserisci URL di destinazione (es. 21-glossario.html o https://...):", "");
        if (url) document.execCommand("createLink", false, url);
        break;
      }
      case "h2":
        document.execCommand("formatBlock", false, "<h2>");
        break;
      case "h3":
        document.execCommand("formatBlock", false, "<h3>");
        break;
      case "p":
        document.execCommand("formatBlock", false, "<p>");
        break;
      case "ul":
        document.execCommand("insertUnorderedList", false, null);
        break;
      case "ol":
        document.execCommand("insertOrderedList", false, null);
        break;
    }
  }

  function insertBox(type) {
    if (viewMode !== "visual") return;
    let snippet = "";
    switch (type) {
      case "key":
        snippet = '<div class="box key"><b>Concetto chiave</b><p>Scrivi qui il concetto fondamentale...</p></div><p></p>';
        break;
      case "warn":
        snippet = '<div class="box warn"><b>Attenzione</b><p>Scrivi qui l\'avviso o la trappola da evitare...</p></div><p></p>';
        break;
      case "lab":
        snippet = '<div class="box lab"><b>In laboratorio</b><p>Dettaglio pratico o strumentale...</p></div><p></p>';
        break;
      case "eq":
        snippet = '<span class="eq"><i>Formula</i> = <span class="frac"><span><i>Numeratore</i></span><span><i>Denominatore</i></span></span><span class="no">(eq)</span></span><p></p>';
        break;
      case "q":
        snippet = '<details class="q"><summary>Testo della domanda di verifica?</summary><p>Risposta dettagliata alla domanda.</p></details><p></p>';
        break;
    }
    if (snippet) insertHtmlAtCursor(snippet);
  }

  function insertHtmlAtCursor(html) {
    const sel = window.getSelection();
    if (!sel || !sel.rangeCount) {
      const mn = document.querySelector("main");
      if (mn) mn.insertAdjacentHTML("beforeend", html);
      return;
    }
    const range = sel.getRangeAt(0);
    range.deleteContents();
    const el = document.createElement("div");
    el.innerHTML = html;
    const frag = document.createDocumentFragment();
    let node, lastNode;
    while ((node = el.firstChild)) {
      lastNode = frag.appendChild(node);
    }
    range.insertNode(frag);
    if (lastNode) {
      range.setStartAfter(lastNode);
      range.collapse(true);
      sel.removeAllRanges();
      sel.addRange(range);
    }
  }

  function enableVisualEditing() {
    const mn = document.querySelector("main");
    if (!mn) return;

    // Rimuove temporaneamente i link o wrapper non desiderati durante l'editing
    mn.setAttribute("contenteditable", "true");
    mn.setAttribute("spellcheck", "false");

    // Proteggi simulazioni e navigazione
    mn.querySelectorAll(".sim, .pn, canvas, svg, .ctl, .btns").forEach(el => {
      el.setAttribute("contenteditable", "false");
    });

    // Nascondi pulsanti prev/next durante l'editing
    const pn = mn.querySelector(".pn");
    if (pn) pn.style.display = "none";

    // Disabilita click su link interni per facilitarne la modifica
    mn.addEventListener("click", preventLinkNav);
  }

  function disableVisualEditing() {
    const mn = document.querySelector("main");
    if (!mn) return;

    mn.removeAttribute("contenteditable");
    mn.removeAttribute("spellcheck");
    mn.querySelectorAll("[contenteditable]").forEach(el => el.removeAttribute("contenteditable"));

    const pn = mn.querySelector(".pn");
    if (pn) pn.style.display = "";

    mn.removeEventListener("click", preventLinkNav);
  }

  function preventLinkNav(e) {
    const a = e.target.closest("a");
    if (a && isEditing && viewMode === "visual") {
      e.preventDefault();
    }
  }

  function switchMode(newMode) {
    if (newMode === viewMode) return;
    viewMode = newMode;

    const btnVis = document.getElementById("tedit-btn-visual");
    const btnHtml = document.getElementById("tedit-btn-html");
    if (btnVis && btnHtml) {
      btnVis.classList.toggle("active", newMode === "visual");
      btnHtml.classList.toggle("active", newMode === "html");
    }

    const mn = document.querySelector("main");
    if (!mn) return;

    if (newMode === "html") {
      // Passa a Codice HTML: costruisci il documento completo con le modifiche visive attuali
      const currentFullHtml = buildUpdatedDocumentHtml();

      disableVisualEditing();
      mn.style.display = "none";

      if (!codeWrapEl) {
        codeWrapEl = document.createElement("div");
        codeWrapEl.id = "tedit-code-wrap";
        codeWrapEl.innerHTML = `
          <div class="tedit-code-header">
            <span>Editor sorgente HTML completo: <code>${here}</code></span>
            <small>Modifica direttamente il codice HTML; passando a «Visuale» o salvando le modifiche verranno applicate.</small>
          </div>
          <textarea id="tedit-code" spellcheck="false" aria-label="Codice sorgente HTML"></textarea>
        `;
        mn.after(codeWrapEl);
        codeTextarea = document.getElementById("tedit-code");
      }
      codeWrapEl.style.display = "flex";
      codeTextarea.value = currentFullHtml;
      codeTextarea.focus();
    } else {
      // Torna a Visuale: carica l'HTML dalla textarea dentro originalDocHtml e aggiorna <main>
      if (codeTextarea) {
        originalDocHtml = codeTextarea.value;
        const mainMatch = originalDocHtml.match(/<main[^>]*>([\s\S]*?)<\/main>/i);
        if (mainMatch) {
          mn.innerHTML = mainMatch[1];
        }
      }

      if (codeWrapEl) codeWrapEl.style.display = "none";
      mn.style.display = "";
      enableVisualEditing();
    }
  }

  // Estrae l'HTML pulito di <main> rimuovendo i nodi temporanei aggiunti da teoria.js
  function getCleanMainHtml() {
    const mn = document.querySelector("main");
    if (!mn) return "";
    const clone = mn.cloneNode(true);

    // Rimuovi pulsanti prev/next
    clone.querySelectorAll(".pn").forEach(e => e.remove());

    // Ripristina tabelle incapsulate in .tw
    clone.querySelectorAll(".tw").forEach(tw => {
      const table = tw.querySelector("table");
      if (table) tw.parentNode.replaceChild(table, tw);
      else tw.remove();
    });

    // Ripristina termini glossario <span class="g">...</span> -> puro testo
    clone.querySelectorAll(".g").forEach(sp => {
      const txt = document.createTextNode(sp.textContent);
      sp.parentNode.replaceChild(txt, sp);
    });

    // Rimuovi aria-level generati a runtime
    clone.querySelectorAll("[aria-level]").forEach(el => el.removeAttribute("aria-level"));

    // Rimuovi attributi contenteditable
    clone.querySelectorAll("[contenteditable]").forEach(el => el.removeAttribute("contenteditable"));

    // Ripristina didascalie figure / svg
    clone.querySelectorAll("svg[role=img][aria-labelledby]").forEach(svg => {
      const prev = svg.previousElementSibling;
      if (prev && prev.id && /^fig-cap-\d+$/.test(prev.id)) prev.removeAttribute("id");
      svg.removeAttribute("role");
      svg.removeAttribute("aria-labelledby");
    });

    return clone.innerHTML;
  }

  function buildUpdatedDocumentHtml() {
    if (viewMode === "html" && codeTextarea) {
      return codeTextarea.value;
    }
    const cleanMain = getCleanMainHtml();
    if (!originalDocHtml) {
      originalDocHtml = buildFullDocumentHtml();
    }
    // Sostituisci il blocco <main>...</main>
    if (/<main[^>]*>[\s\S]*?<\/main>/i.test(originalDocHtml)) {
      return originalDocHtml.replace(/(<main[^>]*>)[\s\S]*?(<\/main>)/i, `$1\n${cleanMain}\n$2`);
    }
    return originalDocHtml;
  }

  function buildFullDocumentHtml() {
    const cleanMain = getCleanMainHtml();
    const title = document.title || "Teoria mzLab";
    return `<!doctype html>
<html lang="it"><head><meta charset="utf-8"><!--tema--><meta name="color-scheme" content="light dark"><style>html{background:#fbfbf9}@media(prefers-color-scheme:dark){html:not([data-theme=light]){background:#14171b}}html[data-theme=dark]{background:#14171b}html[data-theme=light]{background:#fbfbf9}html.no-tr *,html.no-tr *::before,html.no-tr *::after{transition:none!important}</style><script>(function(){var R=document.documentElement,t=null;try{if(parent!==window)t=parent.document.documentElement.getAttribute("data-theme")}catch(e){}if(!t)try{t=JSON.parse(localStorage.getItem("qqq.prefs")||"{}").theme}catch(e){}if(t==="dark"||t==="light"){R.setAttribute("data-theme",t);R.style.colorScheme=t}R.classList.add("no-tr");addEventListener("load",function(){requestAnimationFrame(function(){requestAnimationFrame(function(){R.classList.remove("no-tr")})})})})()</script><!--/tema-->
<title>${title}</title><link rel="icon" href="../favicon-32.png"><link rel="stylesheet" href="teoria.css"><script src="tema.js"></script></head>
<body>
<div class="wrap"><nav id="side"></nav><main>
${cleanMain}
</main></div>
<script src="../appname.js"></script><script src="teoria.js"></script>
</body></html>
`;
  }

  async function salvaCapitolo() {
    const saveBtn = document.getElementById("tedit-btn-save");
    if (saveBtn) {
      saveBtn.disabled = true;
      saveBtn.textContent = "⏳ Salvataggio...";
    }

    const finalHtml = buildUpdatedDocumentHtml();

    try {
      showToast("Invio al server e verifica controlli...", "info", 10000);

      const res = await fetch(`${SERVER_URL}/salva`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file: here, html: finalHtml })
      });

      const data = await res.json();

      if (res.ok && data.ok) {
        showToast("✅ Capitolo salvato e pubblicato con successo su GitHub!", "ok", 4000);
        setTimeout(() => {
          location.reload();
        }, 1500);
      } else {
        const errMsg = data.error || data.warning || "Errore sconosciuto durante il salvataggio.";
        showToast("❌ Errore durante il salvataggio:\n" + errMsg, "err", 8000);
        showOfflineFallbackModal(finalHtml, errMsg);
      }
    } catch (err) {
      // Server offline
      showToast("⚠️ Server locale non raggiungibile su " + SERVER_URL, "err", 5000);
      showOfflineFallbackModal(finalHtml, "Server non attivo: " + err.message);
    } finally {
      if (saveBtn) {
        saveBtn.disabled = false;
        saveBtn.textContent = "💾 Salva e pubblica";
      }
    }
  }

  function showOfflineFallbackModal(htmlContent, errReason) {
    const old = document.getElementById("tedit-modal");
    if (old) old.remove();

    const modal = document.createElement("div");
    modal.id = "tedit-modal";
    modal.innerHTML = `
      <div class="tedit-modal-card">
        <h3>Salvataggio locale & Pubblicazione</h3>
        <p class="tedit-modal-err"><b>Avviso:</b> ${escapeHtml(errReason)}</p>
        <p>Per abilitare il salvataggio diretto con <code>git commit</code> e <code>git push</code> automatici:</p>
        <ol>
          <li>Apri il terminale nella cartella del progetto mzLab</li>
          <li>Esegui il comando: <code>python3 tools/server_teoria.py</code></li>
          <li>Torna su questa pagina e clicca di nuovo su <b>Salva e pubblica</b></li>
        </ol>
        <div class="tedit-modal-actions">
          <button type="button" class="tedit-btn" id="tedit-copy-btn">📋 Copia codice HTML</button>
          <button type="button" class="tedit-btn" id="tedit-dl-btn">📥 Scarica file ${here}</button>
          <button type="button" class="tedit-save" id="tedit-retry-btn">🔄 Riprova salvataggio</button>
          <button type="button" class="tedit-close" id="tedit-modal-close">Chiudi</button>
        </div>
      </div>
    `;
    document.body.appendChild(modal);

    document.getElementById("tedit-modal-close").onclick = () => modal.remove();
    document.getElementById("tedit-retry-btn").onclick = () => {
      modal.remove();
      salvaCapitolo();
    };
    document.getElementById("tedit-copy-btn").onclick = () => {
      navigator.clipboard.writeText(htmlContent).then(() => {
        alert("Codice HTML copiato negli appunti!");
      });
    };
    document.getElementById("tedit-dl-btn").onclick = () => {
      const blob = new Blob([htmlContent], { type: "text/html;charset=utf-8" });
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = here;
      a.click();
    };
  }

  function escapeHtml(str) {
    return String(str).replace(/[&<>"']/g, m => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[m]);
  }

  let toastTimer = null;
  function showToast(text, type = "info", ms = 3000) {
    let t = document.getElementById("tedit-toast");
    if (!t) {
      t = document.createElement("div");
      t.id = "tedit-toast";
      document.body.appendChild(t);
    }
    clearTimeout(toastTimer);
    t.className = type;
    t.textContent = text;
    t.style.display = "block";
    toastTimer = setTimeout(() => {
      if (t) t.style.display = "none";
    }, ms);
  }

  // Esponi per l'uso da console o script esterni
  window.TEORIA_EDITOR = {
    toggle: toggleEditor,
    enter: enterEditor,
    exit: exitEditor,
    salva: salvaCapitolo,
    serverUrl: SERVER_URL
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
