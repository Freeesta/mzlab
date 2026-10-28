// @ts-check
// The first time a 2D map is opened, each open file builds its per-scan cells (`api/map?warm=1`, one file at a time while the browser is idle): the first map and
// the zooms then read a table, not the peaks. Classic script, loaded before mappa.js, which calls `MAPPA_WARM`; the engine ignores the call where it has no table to build.
(() => {
  let started = false;
  const idle = () => new Promise(ok => (/** @type {any} */ (window).requestIdleCallback ? /** @type {any} */ (window).requestIdleCallback(ok, { timeout: 4000 }) : setTimeout(ok, 500)));
  /** @type {any} */ (window).MAPPA_WARM = async function () {
    if (started) return; started = true;
    await idle();
    for (const f of ((typeof E !== "undefined" && E && E.files) || []).slice()) {
      if (!f || f.lv === "pda") continue;
      await idle();
      try { await (await fetch(`api/map?k=${f.k}&level=${f.lv || 1}&warm=1`)).json(); } catch (e) { /* the map asks for the table itself */ }
    }
  };
})();
