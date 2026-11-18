// @ts-check
// @ts-strict
"use strict";
// Central switches of dormant features: the code stays, the interface does not show it. A switch is turned on only for development and
// tests, never from the interface: «?qaqc=1» in the address or localStorage «qqq.dev.qaqc» = «1». «?qaqc=0» turns it off again.
//   lists: the reference-lists panel (button, palette, «Altro», context action); off only in the lab build (window.MZLAB_LAB, tools/lab/lab.js).
//   targetScreening: the HRMS Target Screening tab of the Known contaminants panel (liste.js, liste-target.js; hidden since the panel shows the lists directly): «?targetscreening=1» or «qqq.dev.targetscreening» = «1».
//   qaqc: the QA/QC panel and everything that points to it (button, «Altro», palette, normalising on an internal standard, section 4 of the report).
// Read by barra.js (commands: `feature: "qaqc"`), profilo.js, rapporto.js. The roles (QC, dark) of CAMPIONI and the workflow file keep working.
const FEATURES = (() => {
  const on = name => {
    try {
      const q = new URLSearchParams(location.search).get(name);
      if (q === "1") return true;
      if (q === "0") return false;
    } catch (_) { /* no address */ }
    try { return localStorage.getItem(NOMI.KEY("dev." + name)) === "1"; } catch (_) { return false; }
  };
  return { qaqc: on("qaqc"), lists: !window.MZLAB_LAB, get targetScreening() { return on("targetscreening"); } };      // a getter: tests turn it on without a reload
})();
window.FEATURES = FEATURES;
if (!FEATURES.lists) { const p = document.getElementById("sb-panel-lists"); if (p) p.remove(); }
if (!FEATURES.qaqc) { const p = document.getElementById("sb-panel-qaqc"); if (p) p.remove(); }
