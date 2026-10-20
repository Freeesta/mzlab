// Undo/redo history and browser draft of the Teoria editor (mzlab/web/libro/storia.js); prints one JSON object. See tests/test_teoria_editor.py.
import { clearDraft, loadDraft, makeHistory, saveDraft } from "../mzlab/web/libro/storia.js";

const out = {};
const h = makeHistory("a", 3);
out.same = h.record("a");
h.record("b");
h.record("c");
out.undo = [h.undo(), h.undo(), h.undo()];
out.redo = [h.redo(), h.redo(), h.redo()];
h.undo();
h.record("x");
out.branch = { canRedo: h.canRedo(), now: h.current(), back: h.undo() };
const l = makeHistory("0", 3);
for (const t of ["1", "2", "3", "4"]) l.record(t);
out.limit = [l.undo(), l.undo(), l.undo()];

const mem = new Map();
const store = { getItem: (k) => (mem.has(k) ? mem.get(k) : null), setItem: (k, v) => mem.set(k, v), removeItem: (k) => mem.delete(k) };
out.saved = saveDraft("12-dati.md", { text: "testo", base: "sha1", summary: "s", saved: "2026-10-20T10:00:00Z", token: "github_pat_SECRET" }, store);
out.raw = [...mem.values()][0];
out.loaded = loadDraft("12-dati.md", store);
out.other = loadDraft("13-formula.md", store);
mem.set("mzlab.libro.teoria.draft.bad", "{not json");
out.bad = loadDraft("bad", store);
clearDraft("12-dati.md", store);
out.cleared = loadDraft("12-dati.md", store);
const broken = { getItem() { throw new Error("blocked"); }, setItem() { throw new Error("blocked"); }, removeItem() { throw new Error("blocked"); } };
out.blocked = [saveDraft("x", { text: "t", base: "b", summary: "", saved: "" }, broken), loadDraft("x", broken)];
console.log(JSON.stringify(out));
