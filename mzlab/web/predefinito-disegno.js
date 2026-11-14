// @ts-strict
// @ts-check
// Lazy (carica("predefinito-disegno.js")) from predefinito.js, desktop app only: a `.ket` handed over by the system opens in the drawing editor.
// Saving stays the editor's own export, which asks where; nothing is written back here.
window.DESKDEFKET = {
  async open(/** @type {{name: string, text: string | null, error: string | null}} */ d) {
    if (d.text == null) { alert(I18N.t("predef.ket.error", { name: d.name, detail: d.error || "" })); return; }
    for (let i = 0; i < 300 && !(window.TPDraw && typeof setView === "function"); i++) await new Promise(ok => setTimeout(ok, 100));
    if (!window.TPDraw || typeof setView !== "function") return;
    setView("draw");
    try { await window.TPDraw.load(d.text); } catch (_) { alert(I18N.t("predef.ket.error", { name: d.name, detail: "" })); }
  }
};
