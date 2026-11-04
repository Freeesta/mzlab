// Ketcher on tablets (Draw tab): its hidden clipboard textarea must neither scroll the page nor be a keyboard stop, and the top bar
// must stay reachable in a narrow window (iPad split view). Mounted into the Ketcher iframe document by draw.js.
const CSS =
  // the textarea sits at the bottom of the iframe: focusing it made the parent scroll to it; fixed at the corner it stays invisible and harmless
  '.cliparea,[data-cliparea]{position:fixed!important;top:0!important;left:0!important}' +
  // below 600 px the top bar wraps to a second row (undo and redo stay in the first); the height change is read back by fit() on resize
  '@media (max-width:599px){html [class*="App-module_top"],html [class*="App-module_top"]>*{flex-wrap:wrap!important;row-gap:2px}html [class*="App-module_top"],html [class*="App-module_top"]>*{height:auto!important}html [class*="App-module_app"]{grid-template-rows:auto minmax(0,1fr)!important}}';   // html: wins over the nowrap of draw.js, which is added later

/** @param {Document} d  the document of the Ketcher iframe */
export function mountTablet(d) {
  if (!d.head || !d.body || d.getElementById("tp-tablet")) return;
  const st = d.createElement("style"); st.id = "tp-tablet"; st.textContent = CSS; d.head.appendChild(st);
  // not a Tab stop, and no on-screen keyboard after a tap; again whenever Ketcher re-creates the element
  const fix = () => { for (const c of d.querySelectorAll(".cliparea,[data-cliparea]")) { if (c.getAttribute("tabindex") !== "-1") c.setAttribute("tabindex", "-1"); if (c.getAttribute("inputmode") !== "none") c.setAttribute("inputmode", "none"); } };
  fix(); new MutationObserver(fix).observe(d.body, { childList: true, subtree: true });
}
