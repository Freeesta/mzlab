"use strict";
// Central switches of dormant features: the code stays, the interface does not show it. A switch is turned on only for development and
// tests, never from the interface: «?qaqc=1» in the address or localStorage «qqq.dev.qaqc» = «1». «?qaqc=0» turns it off again.
//   qaqc: the QA/QC panel and everything that points to it (button, «Altro», palette, normalising on an internal standard, section 4 of the report).
// Read by barra.js (commands: `feature: "qaqc"`), profilo.js, rapporto.js. The roles (QC, dark) of CAMPIONI and the workflow file keep working.
const FEATURES = (() => {
  const on = name => {
    try {
      const q = new URLSearchParams(location.search).get(name);
      if (q === "1") return true;
      if (q === "0") return false;
    } catch (_) { /* no address */ }
    try { return localStorage.getItem("qqq.dev." + name) === "1"; } catch (_) { return false; }
  };
  return { qaqc: on("qaqc") };
})();
window.FEATURES = FEATURES;
if (!FEATURES.qaqc) { const p = document.getElementById("sb-panel-qaqc"); if (p) p.remove(); }
