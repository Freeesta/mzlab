// @ts-strict
// @ts-check
// The smartphone card (index.html, #phone). Loaded only on a phone (telefono.js sets window.QQQ_PHONE): the Dati and Disegno scripts,
// the page shell and the engine are never fetched there. Shows the card, wires "info" and "Apri comunque", and warms the Teoria in the
// background so that "Vai alla Teoria" opens at once.
(function () {
  const $ = s => document.querySelector(s);
  const en = I18N.lang === "en";
  const go = $("#phone .phb");
  if (en) go.textContent = I18N.t("theory.sorry.openItalian"); // the Teoria exists only in Italian: the button says so
  $("#phone").hidden = false;

  const hd = $("header"), nav = $("#nav");
  nav.insertAdjacentHTML("afterend", '<button id="phinfo" type="button" aria-label="' + I18N.t("phone.info") + '" title="' + I18N.t("phone.info") + '">i</button>');
  $("#phinfo").onclick = () => { const m = $("#phmore"); m.hidden = !m.hidden; if (!m.hidden) m.scrollIntoView({ behavior: "smooth", block: "nearest" }); };
  nav.querySelector("[data-v=theory]")?.addEventListener("click", () => { location.href = "static/teoria/index.html"; });
  // "Apri comunque": the choice is kept for this tab (same key as telefono.js) and the page starts again as the full program
  $("#phopen").onclick = () => { try { sessionStorage.setItem("qqq.telefono", "0"); } catch (_) { /* storage blocked: the URL decides */ } location.replace(location.pathname + "?telefono=0"); };
  let y0 = scrollY;
  // the header goes away scrolling down, comes back scrolling up
  addEventListener("scroll", () => { const y = scrollY; hd.classList.toggle("away", y > y0 && y > 60); y0 = y; }, { passive: true });

  // Teoria warm-up: only the files of the Teoria home, low priority, when the browser is idle, never on "save data". The service worker
  // answers through its normal path (network, then the app cache), so the later visit finds them without a second download.
  const conn = /** @type {any} */ (navigator).connection;
  if (conn && conn.saveData) return;
  const warm = () => {
    for (const u of ["static/teoria/index.html", "static/teoria/teoria.css", "static/teoria/tema.js", "static/teoria/teoria.js", "static/teoria/lettura.js", "static/teoria/01-tp.html", "static/favicon-32.png"]) {
      try { fetch(u, { priority: "low" }).then(r => r.arrayBuffer()).catch(() => {}); } catch (_) { /* prefetch is optional */ }
    }
  };
  if (/** @type {any} */ (window).requestIdleCallback) requestIdleCallback(warm, { timeout: 3000 }); else setTimeout(warm, 800);
})();
