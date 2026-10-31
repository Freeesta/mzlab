"""«Group by» of the side list (mzlab/web/gruppi.js, run by node with the page globals stubbed): an answer of the engine is mapped to labels (one key, «Then by»,
«Contains» with «Other»), choosing a key is ONE undoable step that replaces manual groups, a manual edit sets the key `custom`, later files take the key."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

STUBS = """
globalThis.window = globalThis;
globalThis.I18N = { t: (k, p) => ({ "groups.other": "Other" }[k] || k) + (p && p.n ? " " + p.n : "") };
globalThis.EH = s => String(s);
globalThis.fileBase = n => String(n).split("#")[0];
globalThis.document = { getElementById: () => null };
globalThis.steps = 0; globalThis.renders = 0; globalThis.fetched = [];
globalThis.fileViewDo = fn => { steps++; fn(); };
globalThis.renderFileList = () => { renders++; }; globalThis.uiSave = () => {};
globalThis.lrTog = () => true; globalThis.ESP = { voce: () => ({ spettri: true }) };
globalThis.E = { files: ["MQ_H2O2_T00.mzML", "MQ_N2_T00.mzML", "TW_H2O2_T00.mzML", "QC_std.mzML"].map(file => ({ file })) };
// the engine stub: the first token of the name is the matrix (none for QC), the second the condition; shaped as `nomivar/1`
const NAMES = n => n.replace(/\\.mzML$/, "").split("_");
globalThis.ask = async () => null; globalThis.STORIA = null;
globalThis.fetch = async (_url, { body }) => {
  // the engine's group name: the first token the names share
  if (_url.endsWith("nome_gruppo")) { const t = JSON.parse(body).names.map(n => n.split("_")[0]); return { json: async () => ({ nome: t.every(x => x === t[0]) ? t[0] : "" }) }; }
  const { files, manuale } = JSON.parse(body); fetched.push(files.map(f => f.name));
  const col = (i, skip) => { const by = new Map(), senza = []; files.forEach((f, k) => { const t = NAMES(f.name)[i]; if (!t || skip(t)) senza.push(k); else by.set(t, [...(by.get(t) || []), k]); });
    return { valori: [...by].sort((a, b) => b[1].length - a[1].length).map(([etichetta, file]) => ({ etichetta, file })), senza }; };
  const ans = { files: files.map(f => ({ name: f.name })), comune: "", variabili: [{ id: "matrix", ...col(0, t => t === "QC") }, { id: "cond", ...col(1, t => t === "T00" || t === "std") }] };
  if (manuale && manuale.has) { const c = col(1, () => false); ans.manuale = { id: "has", valori: c.valori.filter(v => manuale.has.includes(v.etichetta)), senza: files.map((_, k) => k).filter(k => !manuale.has.includes(NAMES(files[k].name)[1])), multipli: [] }; }
  return { json: async () => ans };
};
"""


def run(code):
    gr = (WEB / "gruppi.js").read_text(encoding="utf-8")
    lf = (WEB / "lista-file.js").read_text(encoding="utf-8")
    js = STUBS + f"""
const vm = require("vm"); vm.runInThisContext({json.dumps(gr)});
globalThis.COMANDI = {{ gestore() {{}}, fai: (_a, _b, fn) => {{ steps++; fn(); }}, GLOBALE: 0 }};
vm.runInThisContext({json.dumps(lf)});
const G = () => E.files.map(f => f.ugrp || null);
(async () => {{ {code} }})().catch(e => {{ console.error(e); process.exit(1); }});
"""
    out = subprocess.run(["node", "-"], input=js, check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


@needs_node
def test_a_key_maps_every_value_to_its_label_and_dash_files_get_none():
    out = run("""await GRUPPI.by({ by: "matrix" }); console.log(JSON.stringify([G(), E.ugBy, E.ugOrder, steps]))""")
    assert out == [["MQ", "MQ", "TW", None], {"by": "matrix"}, ["MQ", "TW"], 1]


@needs_node
def test_then_by_joins_the_two_labels_and_orders_by_the_first_key():
    out = run("""await GRUPPI.by({ by: "matrix", then: "cond" }); console.log(JSON.stringify([G(), E.ugOrder]))""")
    assert out == [["MQ · H2O2", "MQ · N2", "TW · H2O2", None], ["MQ · H2O2", "MQ · N2", "TW · H2O2"]]


@needs_node
def test_contains_sends_the_files_without_a_term_to_other():
    out = run("""await GRUPPI.by({ by: "manuale", manuale: { has: ["N2"], substring: false } }); console.log(JSON.stringify([G(), E.ugOrder, E.ugBy.manuale]))""")
    assert out == [["Other", "N2", "Other", "Other"], ["N2", "Other"], {"has": ["N2"], "substring": False}]


@needs_node
def test_a_key_replaces_manual_groups_in_one_step_and_none_removes_them():
    out = run("""ugSet([E.files[0], E.files[3]], "Mine"); const manual = [G(), E.ugBy.by]; steps = 0;
await GRUPPI.by({ by: "matrix" }); const keyed = G();
await GRUPPI.by(null); console.log(JSON.stringify([manual, keyed, G(), E.ugBy ?? null, steps]))""")
    assert out == [[["Mine", None, None, "Mine"], "custom"], ["MQ", "MQ", "TW", None], [None] * 4, None, 2]


@needs_node
def test_a_manual_edit_sets_custom_and_set_writes_a_group_by_file_name():
    out = run("""await GRUPPI.by({ by: "matrix" }); ugSet([E.files[1]], ""); const a = [G(), E.ugBy];
GRUPPI.set(["MQ_N2_T00.mzML", "QC_std.mzML"], "Pair"); console.log(JSON.stringify([a, G(), E.ugBy]))""")
    assert out == [[["MQ", None, "TW", None], {"by": "custom"}], ["MQ", "Pair", "TW", "Pair"], {"by": "custom"}]


@needs_node
def test_later_files_take_the_key_unless_it_is_custom():
    out = run("""await GRUPPI.by({ by: "matrix" }); fetched.length = 0;
E.files.push({ file: "TW_N2_T00.mzML" }); GRUPPI.follow(); await new Promise(r => setTimeout(r, 450)); const keyed = G();
E.ugBy = { by: "custom" }; E.files.push({ file: "MQ_PMS_T00.mzML" }); fetched.length = 0; GRUPPI.follow(); await new Promise(r => setTimeout(r, 450));
console.log(JSON.stringify([keyed, G()[5], fetched.length]))""")
    assert out == [["MQ", "MQ", "TW", None, "TW"], None, 0]


@needs_node
def test_the_user_groups_follow_the_order_of_the_key():
    out = run("""E.ugOrder = ["TW", "MQ"]; const gs = [{ g: "Full Scan" }, { g: "u:MQ" }, { g: "u:TW" }, { g: "u:Mine" }];
GRUPPI.order(gs); console.log(JSON.stringify(gs.map(x => x.g)))""")
    assert out == ["Full Scan", "u:TW", "u:MQ", "u:Mine"]


@needs_node
def test_a_file_dropped_onto_an_ungrouped_file_makes_a_group_named_by_the_engine_and_onto_a_grouped_one_joins_it():
    out = run("""
      const g1 = await GRUPPI.onto(["MQ_N2_T00.mzML"], "MQ_H2O2_T00.mzML");            // new group of the two, named by the common token
      const a = [g1, G(), steps];
      const g2 = await GRUPPI.onto(["QC_std.mzML"], "MQ_N2_T00.mzML");                 // the target is grouped: the file joins, no nesting
      console.log(JSON.stringify([a, g2, G(), steps]));""")
    assert out == [["MQ", ["MQ", "MQ", None, None], 1], "MQ", ["MQ", "MQ", None, "MQ"], 2]


@needs_node
def test_the_name_of_a_new_group_falls_back_to_group_n_when_the_engine_has_none_or_it_is_taken():
    out = run("""
      const a = await GRUPPI.newName(["MQ_H2O2_T00.mzML", "TW_H2O2_T00.mzML"]);        // no common token
      await GRUPPI.set(["MQ_H2O2_T00.mzML"], "MQ");
      const b = await GRUPPI.newName(["MQ_N2_T00.mzML", "MQ_H2O2_T00.mzML"]);          // «MQ» exists
      await GRUPPI.set(["QC_std.mzML"], b);
      console.log(JSON.stringify([a, b, await GRUPPI.newName(["MQ_N2_T00.mzML", "MQ_H2O2_T00.mzML"])]));""")
    assert out == ["files.ugrp.name 1", "files.ugrp.name 1", "files.ugrp.name 2"]


@needs_node
def test_every_group_action_is_one_step_named_after_the_action():
    out = run("""
      const pila = { v: [] }; globalThis.STORIA = { pila };
      COMANDI.fai = (_a, _b, fn) => { fn(); pila.v.push({ cmd: "files.view", w: "storia.campi", t: Date.now() }); };
      await GRUPPI.by({ by: "matrix" }); const w = [pila.v.at(-1).w];
      GRUPPI.set(["QC_std.mzML"], "MQ"); w.push(pila.v.at(-1).w);
      await GRUPPI.onto(["QC_std.mzML"], "TW_H2O2_T00.mzML"); w.push(pila.v.at(-1).w);
      GRUPPI.ungroup("MQ"); w.push(pila.v.at(-1).w);
      await GRUPPI.by(null); w.push(pila.v.at(-1).w);
      console.log(JSON.stringify([w, pila.v.length]));""")
    assert out == [["groups.undo.by", "groups.undo.move", "groups.undo.join", "groups.undo.ungroup", "groups.undo.none"], 5]
