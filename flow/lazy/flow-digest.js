// @ts-check
// Flow (administrator mode): the settings digest of a campaign folder. It is read from the summary the engine writes with the evidence export (`evidence_<campaign>.json`), so no engine route is needed;
// the card shows the 12-digit sha1 of the thresholds, the list they come from and a button that copies the digest. Used by the campaign view (flow-campaign.js).
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);

  /** the digest, the settings and the number of decided features from the answer of the evidence export; null when the summary is missing @param {{ files?: { name: string, text?: string }[] }} answer */
  F.settingsDigest = answer => {
    const f = (answer.files || []).find(x => /^evidence_.*\.json$/.test(x.name) && typeof x.text === "string");
    if (!f) return null;
    try {
      const j = JSON.parse(/** @type {string} */ (f.text));
      return j && j.settings_hash ? { hash: String(j.settings_hash), settings: String(j.settings || ""), features: Number(j.features) || 0 } : null;
    } catch { return null; }
  };

  /** a button that asks the engine for the digest and shows it with a copy button @param {HTMLElement} host @param {any} c the campaign of an item @param {HTMLElement} say */
  F.digestMount = (host, c, say) => {
    const { el } = F, out = el("p", { class: "fl-note flw-digest", role: "status" });
    const show = async () => {
      say.textContent = "Reading the settings…";
      try {
        const d = F.settingsDigest(await (await G.ATTRIBUTIO_CORE.pool()).attributioExport(c.files, c.root, "evidence", {}));
        say.textContent = "";
        out.textContent = "";
        if (!d) { out.textContent = "No settings digest for this folder."; return; }
        const copy = F.btn("copy", "Copy the settings digest", async () => {
          try { await navigator.clipboard.writeText(d.hash); say.textContent = "Digest copied."; } catch { say.textContent = "The browser did not allow copying."; }
        }, "Copy");
        out.append(el("span", {}, "Settings digest "), el("code", {}, d.hash), " ", copy, el("br"), el("span", { class: "fl-note" }, d.settings));
      } catch (e) { say.textContent = "Settings digest: " + ((/** @type {any} */ (e)).message || e); }
    };
    host.append(F.btn("info", "Show the digest of the settings that made these levels", show, "Settings digest"), out);
  };
})();
