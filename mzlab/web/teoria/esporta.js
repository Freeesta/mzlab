/* mzLab - Teoria: esportazione capitoli singoli e pacchetto completo (PDF, Word, Markdown, HTML). */
"use strict";

(function () {
  const here = location.pathname.split("/").pop() || "index.html";
  const T = () => (window.TEORIA = window.TEORIA || {});

  function getChaptersList() {
    const raw = typeof CHAPTERS !== "undefined" ? CHAPTERS : [];
    return raw.filter(c => !c[0].startsWith("pratica") && c[0] !== "00-uso.html");
  }

  function getCurrentChapter() {
    const raw = typeof CHAPTERS !== "undefined" ? CHAPTERS : [];
    return raw.find(c => c[0] === here) || [here, "", document.title || "Capitolo"];
  }

  function cleanMainElement(mainNode) {
    const cl = mainNode.cloneNode(true);
    cl.querySelectorAll(".pn, .skip, #top, #side, script, style, #tedit-bar, #export-modal, .fzoom, #lb, .ctl, .btns, canvas").forEach(e => e.remove());
    cl.querySelectorAll(".tw").forEach(tw => { const t = tw.querySelector("table"); if (t) tw.replaceWith(t); else tw.remove(); });
    cl.querySelectorAll(".g").forEach(sp => sp.replaceWith(document.createTextNode(sp.textContent)));
    cl.querySelectorAll("[contenteditable], [aria-level]").forEach(e => { e.removeAttribute("contenteditable"); e.removeAttribute("aria-level"); });
    return cl;
  }

  function walkMd(node) {
    if (node.nodeType === Node.TEXT_NODE) return node.textContent.replace(/\s+/g, " ");
    if (node.nodeType !== Node.ELEMENT_NODE) return "";
    const tag = node.tagName.toLowerCase();
    const ch = () => Array.from(node.childNodes).map(walkMd).join("");
    switch (tag) {
      case "h1": return `\n\n# ${ch().trim()}\n\n`;
      case "h2": return `\n\n## ${ch().trim()}\n\n`;
      case "h3": return `\n\n### ${ch().trim()}\n\n`;
      case "h4": return `\n\n#### ${ch().trim()}\n\n`;
      case "p": return `\n\n${node.classList.contains("lead") ? `> *${ch().trim()}*` : ch().trim()}\n\n`;
      case "b": case "strong": return `**${ch().trim()}**`;
      case "i": case "em": return `*${ch().trim()}*`;
      case "sub": return `~${ch().trim()}~`;
      case "sup": return `^${ch().trim()}^`;
      case "code": return `\`${ch().trim()}\``;
      case "pre": return `\n\`\`\`\n${node.textContent.trim()}\n\`\`\`\n`;
      case "a": return `[${ch().trim()}](${node.getAttribute("href") || ""})`;
      case "ul": return `\n\n${Array.from(node.children).filter(c => c.tagName === "LI").map(li => `- ${walkMd(li).trim()}`).join("\n")}\n\n`;
      case "ol": return `\n\n${Array.from(node.children).filter(c => c.tagName === "LI").map((li, i) => `${i + 1}. ${walkMd(li).trim()}`).join("\n")}\n\n`;
      case "li": return ch();
      case "div":
        if (node.classList.contains("box")) {
          const type = node.classList.contains("warn") ? "ATTENZIONE" : (node.classList.contains("lab") ? "LABORATORIO" : "CONCETTO CHIAVE");
          return `\n\n> **[${type}]**\n${ch().trim().split("\n").map(l => `> ${l}`).join("\n")}\n\n`;
        }
        return ch();
      case "span": return node.classList.contains("eq") ? `\n\n$$\n${node.textContent.trim()}\n$$\n\n` : ch();
      case "details":
        if (node.classList.contains("q")) {
          const q = node.querySelector("summary")?.textContent.trim() || "Domanda";
          const a = node.querySelector("p, div")?.textContent.trim() || "";
          return `\n\n> **Domanda di ripasso:** ${q}\n>\n> *Risposta:* ${a}\n\n`;
        }
        return ch();
      case "table": {
        const rows = Array.from(node.querySelectorAll("tr")); if (!rows.length) return "";
        let t = "\n\n";
        rows.forEach((tr, i) => {
          const cells = Array.from(tr.querySelectorAll("th, td")).map(c => c.textContent.trim().replace(/\|/g, "\\|"));
          t += `| ${cells.join(" | ")} |\n`;
          if (i === 0) t += `| ${cells.map(() => "---").join(" | ")} |\n`;
        });
        return t + "\n";
      }
      default: return ch();
    }
  }

  function htmlToMarkdown(element, title = "") {
    const main = cleanMainElement(element);
    const md = (title ? `# ${title}\n\n` : "") + walkMd(main);
    return md.replace(/\n{3,}/g, "\n\n").trim() + "\n";
  }

  function wrapWordDocument(bodyHtml, title = "mzLab Teoria") {
    return `<html xmlns:o='urn:schemas-microsoft-com:office:office' xmlns:w='urn:schemas-microsoft-com:office:word' xmlns='http://www.w3.org/TR/REC-html40'><head><meta charset='utf-8'><title>${title}</title><style>@page{size:A4;margin:2cm}body{font-family:Calibri,sans-serif;font-size:11pt;line-height:1.5;color:#222}h1{font-size:22pt;color:#2b5c8a;border-bottom:2pt solid #2b5c8a}h2{font-size:15pt;color:#2b5c8a;border-bottom:1pt solid #ccc}p{margin:6pt 0;text-align:justify}.box{border:1pt solid #ccc;border-left:4pt solid #2b5c8a;background:#f8fafc;padding:8pt 12pt;margin:10pt 0}.box.warn{border-left-color:#c2410c}.box.lab{border-left-color:#047857}.eq{background:#f5f5f5;border:1pt solid #e0e0e0;padding:8pt;margin:8pt 0;font-style:italic}table{border-collapse:collapse;width:100%;margin:10pt 0}th,td{border:1pt solid #bbb;padding:6pt 8pt;text-align:left}th{background:#eef3f8}details.q{border:1pt solid #2b5c8a;background:#f4f8fb;padding:8pt;margin:10pt 0;display:block}.page-break{page-break-before:always}</style></head><body>${bodyHtml}</body></html>`;
  }

  function wrapStandaloneHtml(bodyHtml, title = "mzLab Teoria") {
    return `<!doctype html><html lang="it"><head><meta charset="utf-8"><title>${title}</title><style>body{max-width:860px;margin:30px auto;padding:0 20px;font:16px/1.6 system-ui,sans-serif;color:#24231f;background:#fbfbf9}h1{font-size:28px;color:#2b5c8a}h2{font-size:22px;border-top:1px solid #e4e2dc;padding-top:16px;margin-top:36px}a{color:#2b5c8a}.box{border:1px solid #e4e2dc;border-left:4px solid #2b5c8a;background:#fff;border-radius:6px;padding:12px 16px;margin:16px 0}.box.warn{border-left-color:#c2410c}.box.lab{border-left-color:#047857}.eq{background:#fff;border:1px solid #e4e2dc;border-radius:6px;padding:10px 14px;margin:12px 0}table{border-collapse:collapse;width:100%;margin:14px 0}th,td{border:1px solid #e4e2dc;padding:6px 10px}th{background:#f4f3ef}details.q{border:1px solid #2b5c8a;border-radius:6px;background:#fff;padding:12px 14px;margin:14px 0}.chapter-block{margin-bottom:60px;padding-bottom:40px;border-bottom:2px dashed #e4e2dc}@media print{body{max-width:100%;margin:0;padding:0}.page-break{page-break-before:always}}</style></head><body>${bodyHtml}</body></html>`;
  }

  function triggerDownload(content, filename, mimeType) {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = filename;
    document.body.appendChild(a); a.click();
    setTimeout(() => { document.body.removeChild(a); URL.revokeObjectURL(url); }, 150);
  }

  async function fetchChapterCleanHtml(filename) {
    if (filename === here) {
      const mn = document.querySelector("main");
      if (mn) return cleanMainElement(mn).innerHTML;
    }
    let raw = "";
    try {
      const res = await fetch(filename); if (res.ok) raw = await res.text();
    } catch (_) {
      try {
        const res2 = await fetch(`http://127.0.0.1:8765/carica?file=${encodeURIComponent(filename)}`);
        if (res2.ok) { const d = await res2.json(); if (d.ok) raw = d.html; }
      } catch (_) {}
    }
    if (!raw) return null;
    const doc = new DOMParser().parseFromString(raw, "text/html");
    const mn = doc.querySelector("main") || doc.body;
    return cleanMainElement(mn).innerHTML;
  }

  function openExportModal() {
    let m = document.getElementById("export-modal");
    if (m) { m.style.display = "flex"; return; }
    const cur = getCurrentChapter(), curTitle = `${cur[1] && cur[1] !== '0' ? cur[1] + '. ' : ''}${cur[2]}`;
    m = document.createElement("div"); m.id = "export-modal"; m.className = "exp-backdrop";
    m.innerHTML = `
      <div class="exp-card">
        <div class="exp-header">
          <div><h2>📥 Esporta e Scarica la Teoria</h2><p class="exp-sub">Scarica per studiare offline, prendere appunti in Word o stampare.</p></div>
          <button type="button" class="exp-close" id="exp-btn-close">✕</button>
        </div>
        <div class="exp-grid">
          <div class="exp-sec">
            <div class="exp-sec-hd"><span class="exp-badge">Questo capitolo</span><h3>${curTitle}</h3></div>
            <div class="exp-actions">
              <button type="button" class="exp-item" id="exp-cur-pdf"><span class="exp-ico">📄</span><div class="exp-txt"><b>Salva in PDF (Stampa)</b><span>Layout per stampa o salvataggio PDF</span></div></button>
              <button type="button" class="exp-item" id="exp-cur-md"><span class="exp-ico">📝</span><div class="exp-txt"><b>Markdown (.md)</b><span>Per Obsidian, Notion o appunti</span></div></button>
              <button type="button" class="exp-item" id="exp-cur-doc"><span class="exp-ico">📄</span><div class="exp-txt"><b>Word (.doc)</b><span>Modificabile con Word o LibreOffice</span></div></button>
              <button type="button" class="exp-item" id="exp-cur-html"><span class="exp-ico">🌐</span><div class="exp-txt"><b>HTML autonomo (.html)</b><span>Standalone offline</span></div></button>
            </div>
          </div>
          <div class="exp-sec exp-sec-all">
            <div class="exp-sec-hd"><span class="exp-badge exp-badge-gold">Pacchetto Completo</span><h3>Tutta la Teoria (23 capitoli)</h3></div>
            <div class="exp-actions">
              <button type="button" class="exp-item" id="exp-all-pdf"><span class="exp-ico">📚</span><div class="exp-txt"><b>Dispensa completa PDF</b><span>Libro completo stampabile</span></div></button>
              <button type="button" class="exp-item" id="exp-all-md"><span class="exp-ico">📝</span><div class="exp-txt"><b>Tutta la Teoria in Markdown</b><span>Un unico file .md con tutti i capitoli</span></div></button>
              <button type="button" class="exp-item" id="exp-all-doc"><span class="exp-ico">📄</span><div class="exp-txt"><b>Tutta la Teoria in Word (.doc)</b><span>Libro completo in formato Word</span></div></button>
              <button type="button" class="exp-item" id="exp-all-html"><span class="exp-ico">🌐</span><div class="exp-txt"><b>Tutta la Teoria in HTML</b><span>Unico file HTML leggibile offline</span></div></button>
            </div>
          </div>
        </div>
        <div id="exp-status" class="exp-status" style="display:none"></div>
      </div>`;
    document.body.appendChild(m);
    bindExportEvents(m, curTitle);
  }

  function bindExportEvents(modal, curTitle) {
    const slug = here.replace(/\.html$/, "");
    const close = () => { modal.style.display = "none"; };
    document.getElementById("exp-btn-close").onclick = close;
    modal.onclick = e => { if (e.target === modal) close(); };
    window.addEventListener("keydown", e => { if (e.key === "Escape" && modal.style.display !== "none") close(); });

    document.getElementById("exp-cur-pdf").onclick = () => { close(); window.print(); };
    document.getElementById("exp-cur-md").onclick = () => {
      const mn = document.querySelector("main"); if (mn) triggerDownload(htmlToMarkdown(mn, curTitle), `${slug}.md`, "text/markdown;charset=utf-8");
    };
    document.getElementById("exp-cur-doc").onclick = () => {
      const mn = document.querySelector("main"); if (mn) triggerDownload(wrapWordDocument(cleanMainElement(mn).innerHTML, curTitle), `${slug}.doc`, "application/msword;charset=utf-8");
    };
    document.getElementById("exp-cur-html").onclick = () => {
      const mn = document.querySelector("main"); if (mn) triggerDownload(wrapStandaloneHtml(cleanMainElement(mn).innerHTML, curTitle), `${slug}-standalone.html`, "text/html;charset=utf-8");
    };
    document.getElementById("exp-all-pdf").onclick = () => exportAll("pdf");
    document.getElementById("exp-all-md").onclick = () => exportAll("md");
    document.getElementById("exp-all-doc").onclick = () => exportAll("doc");
    document.getElementById("exp-all-html").onclick = () => exportAll("html");
  }

  function showStatus(msg, isErr = false) {
    const s = document.getElementById("exp-status"); if (!s) return;
    s.style.display = "block"; s.className = "exp-status" + (isErr ? " err" : " ok"); s.textContent = msg;
  }

  async function exportAll(fmt) {
    const chs = getChaptersList();
    showStatus(`Caricamento capitoli... (0/${chs.length})`);
    const loaded = [];
    for (let i = 0; i < chs.length; i++) {
      showStatus(`Caricamento capitolo ${i + 1}/${chs.length}: ${chs[i][2]}`);
      const h = await fetchChapterCleanHtml(chs[i][0]);
      if (h) loaded.push({ num: chs[i][1], title: chs[i][2], html: h });
    }
    if (!loaded.length) { showStatus("Impossibile caricare i capitoli. Avvia: python3 tools/server_teoria.py", true); return; }
    showStatus(`Generazione pacchetto completo (${fmt.toUpperCase()})...`);
    if (fmt === "md") {
      let md = `# mzLab · Dispensa Completa di Teoria\n\n## Indice\n` + loaded.map(c => `- **Capitolo ${c.num}**: ${c.title}`).join("\n") + "\n\n---\n\n";
      loaded.forEach(c => {
        const doc = new DOMParser().parseFromString(`<main>${c.html}</main>`, "text/html");
        md += htmlToMarkdown(doc.querySelector("main"), `Capitolo ${c.num}: ${c.title}`) + "\n\n---\n\n";
      });
      triggerDownload(md, "mzLab_Teoria_Completa.md", "text/markdown;charset=utf-8");
      showStatus("✅ Tutta la Teoria scaricata in Markdown!");
    } else if (fmt === "doc") {
      let b = `<div style="text-align:center;padding:80pt 0"><h1 style="font-size:28pt;color:#2b5c8a">mzLab · Dispensa di Teoria</h1><p>Spettrometria di massa (QqQ, HRMS, ESI, EI) e prodotti di trasformazione (TP)</p></div><div class="page-break"></div><h2>Indice dei capitoli</h2><ul>` + loaded.map(c => `<li><b>Capitolo ${c.num}</b>: ${c.title}</li>`).join("") + `</ul><div class="page-break"></div>`;
      loaded.forEach(c => { b += `<div class="chapter-block">` + c.html + `</div><div class="page-break"></div>`; });
      triggerDownload(wrapWordDocument(b, "mzLab - Teoria Completa"), "mzLab_Teoria_Completa.doc", "application/msword;charset=utf-8");
      showStatus("✅ Tutta la Teoria scaricata in Word (.doc)!");
    } else if (fmt === "html" || fmt === "pdf") {
      let b = `<header style="text-align:center;margin-bottom:40px"><h1 style="font-size:32px;color:#2b5c8a">mzLab · Dispensa Completa</h1><p>Spettrometria di massa e cromatografia</p></header><nav style="background:#fff;border:1px solid #e4e2dc;padding:16px;margin-bottom:30px"><h2>Indice</h2><ol>` + loaded.map(c => `<li><b>${c.num}.</b> ${c.title}</li>`).join("") + `</ol></nav>`;
      loaded.forEach(c => { b += `<section class="chapter-block page-break">` + c.html + `</section>`; });
      if (fmt === "html") {
        triggerDownload(wrapStandaloneHtml(b, "mzLab - Teoria Completa"), "mzLab_Teoria_Completa.html", "text/html;charset=utf-8");
        showStatus("✅ Tutta la Teoria scaricata in HTML!");
      } else {
        const w = window.open("", "_blank");
        if (w) { w.document.write(wrapStandaloneHtml(`<div style="position:sticky;top:0;background:#2b5c8a;color:#fff;padding:10px 20px;display:flex;justify-content:space-between"><span>📚 Dispensa Completa</span><button onclick="window.print()" style="padding:6px 14px;background:#fff;color:#2b5c8a;border:none;border-radius:4px;cursor:pointer;font-weight:bold">🖨️ Stampa o Salva PDF</button></div>` + b, "mzLab - Dispensa Completa")); w.document.close(); showStatus("✅ Finestra di stampa aperta!"); }
        else showStatus("Popup bloccato dal browser", true);
      }
    }
  }

  T().esporta = openExportModal;

  function ensureButtonInBar() {
    const tbar = document.getElementById("tbar");
    if (!tbar || document.getElementById("tc-esporta")) return;
    const btn = document.createElement("button"); btn.type = "button"; btn.className = "tb"; btn.id = "tc-esporta";
    btn.innerHTML = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5v9.5M6 9l4 4 4-4M3.5 15.5v1a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5v-1" /></svg><span class="l">Esporta</span>`;
    btn.title = "Esporta in PDF, Word, Markdown o HTML"; btn.onclick = openExportModal;
    const sBtn = document.getElementById("a11y-btn") || document.getElementById("tc-impostazioni");
    if (sBtn && sBtn.parentNode === tbar) tbar.insertBefore(btn, sBtn); else tbar.appendChild(btn);
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", ensureButtonInBar); else ensureButtonInBar();
  window.addEventListener("load", ensureButtonInBar);
})();
