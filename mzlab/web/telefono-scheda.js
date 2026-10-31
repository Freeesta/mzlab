// @ts-strict
// @ts-check
// The smartphone card (index.html, #phone). Loaded only on a phone (telefono.js sets window.QQQ_PHONE): the Dati and Disegno scripts,
// the page shell and the engine are never fetched there. Shows the card (a plain link to the Teoria site, BasePeak) and wires "info" and "Apri comunque".
(function () {
  const $ = s => document.querySelector(s);
  $("#phone").hidden = false;

  const hd = $("header"), nav = $("#nav");
  nav.insertAdjacentHTML("afterend", '<button id="phinfo" type="button" aria-label="' + I18N.t("phone.info") + '" title="' + I18N.t("phone.info") + '">i</button>');
  $("#phinfo").onclick = () => { const m = $("#phmore"); m.hidden = !m.hidden; if (!m.hidden) m.scrollIntoView({ behavior: "smooth", block: "nearest" }); };
  // "Apri comunque": the choice is kept for this tab (same key as telefono.js) and the page starts again as the full program
  $("#phopen").onclick = () => { try { sessionStorage.setItem(NOMI.KEY("telefono"), "0"); } catch (_) { /* storage blocked: the URL decides */ } location.replace(location.pathname + "?telefono=0"); };
  let y0 = scrollY;
  // the header goes away scrolling down, comes back scrolling up
  addEventListener("scroll", () => { const y = scrollY; hd.classList.toggle("away", y > y0 && y > 60); y0 = y; }, { passive: true });

})();
