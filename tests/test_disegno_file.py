"""Structure files for the Drawing tab (mzlab/web/disegno-file.js) run by node: what the notice says and which pasted text is CDXML. What the text is changed to
(empty attributes, charsets, base64 CDX) is the engine's `open/1`: its tests are in crates/mzlab-feat/src/chem/structure_file_tests.rs and tests/test_molkit_wasm.py."""
import json, subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


# an engine that refuses over a limit of 100 bytes, in place of ./disegno-verifica.js (which needs a window and the `.wasm`): node's resolve hook gives the page code this module
STUB_ENGINE = (
    "export const openSize = async n => { if (n > 100) throw JSON.stringify({ error_key: 'error.open.tooBig', params: { size: String(n), limit: '100' } }); return { limit: 100 }; };"
    "export const openFile = async b => ({ text: new TextDecoder().decode(b), changes: [] });"
    "export const message = e => 'refused ' + JSON.parse(e).params.size;"
)


def run_stub(body):
    hooks = "export async function resolve(s, c, next) { return s.endsWith('disegno-verifica.js') ? { url: 'data:text/javascript,' + encodeURIComponent(process.env.STUB), shortCircuit: true } : next(s, c); }"
    reg = "import { register } from 'node:module'; register('data:text/javascript,' + encodeURIComponent(process.env.HOOKS));"
    r = subprocess.run(["node", "--input-type=module", "-e", reg + "\nawait import('data:text/javascript,' + encodeURIComponent(process.env.MAIN));"],
                       capture_output=True, encoding="utf-8", env={**__import__("os").environ, "STUB": STUB_ENGINE, "HOOKS": hooks, "MAIN": f"import {{ mountImport, readStructure }} from {(WEB / 'disegno-file.js').as_uri()!r};\n(async () => {{ console.log(JSON.stringify(await (async () => {{ {body} }})())); }})();"})
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout)


def run(body):
    src = f"import {{ changesNote, isCdxml, mountImport }} from {(WEB / 'disegno-file.js').as_uri()!r};\n(async () => {{ console.log(JSON.stringify(await (async () => {{ {body} }})())); }})();"
    r = subprocess.run(["node", "--input-type=module", "-e", src], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout)


def test_the_notice_names_the_file_and_every_change_and_stays_empty_for_none():
    got = run("""
    const t = (k, p) => k + JSON.stringify(p || {});
    return [changesNote("a.cdxml", [], t), changesNote("a.cdxml", [{ key: "draw.import.fix.charset", params: { charset: "UTF-16" } }, { key: "draw.import.fix.empty", params: { names: "BondLength" } }], t)];""")
    assert got[0] == ""
    assert got[1] == 'draw.import.changed{"name":"a.cdxml","what":"draw.import.fix.charset{\\"charset\\":\\"UTF-16\\"}; draw.import.fix.empty{\\"names\\":\\"BondLength\\"}"}'


def test_pasted_text_is_cdxml_only_when_it_opens_with_the_declaration_a_doctype_or_the_root():
    cases = ['<CDXML a="1"/>', '<?xml version="1.0"?>\n<CDXML>', '\ufeff<?xml version="1.0"?><!DOCTYPE CDXML SYSTEM "x.dtd"><CDXML ', '  <CDXML>', '<foo bar>', '<CDXMLX>', 'CC(=O)O', 'a <CDXML> inside text']
    assert run(f"return {json.dumps(cases)}.map(isCdxml);") == [True, True, True, True, False, False, False, False]


# a Ketcher and a document that record what reaches them; `import("./disegno-verifica.js")` has no window and no `.wasm` in node, so the engine is unavailable here
WIRED = """
    const L = {}, notes = [], added = [];
    const doc = { body: { addEventListener: (n, f) => { L["body:" + n] = f; } }, addEventListener: (n, f) => { L[n] = f; }, defaultView: {} };
    let canvas = 0;
    const k = { getKet: async () => String(canvas), addFragment: async (x) => { added.push(x); canvas++; } };
    const t = (key, p) => key + JSON.stringify(p || {});
    mountImport(doc, k, { t, note: x => notes.push(x), done: () => {} });
    const ev = (target, text) => { const e = { target, clipboardData: { getData: () => text }, prevented: false, preventDefault() { this.prevented = true; }, stopImmediatePropagation() {}, stopPropagation() {} }; return e; };
"""


def test_a_drop_still_reaches_ketcher_when_the_engine_cannot_be_loaded():
    got = run(WIRED + """
    const bytes = new TextEncoder().encode('<CDXML a=""/>');
    const e = { dataTransfer: { files: [{ name: "a.cdxml", arrayBuffer: async () => bytes.buffer }], getData: () => "" }, preventDefault() {}, stopPropagation() {} };
    await L["body:drop"](e);
    return { added, notes };""")
    assert got == {"added": ['<CDXML a=""/>'], "notes": [""]}, got


def test_a_paste_into_a_text_field_is_never_taken_over_and_a_cdxml_paste_on_the_canvas_is():
    got = run(WIRED + """
    const cdxml = '<CDXML a=""/>';
    const field = tag => ({ closest: sel => sel.split(",").map(x => x.trim()).includes(tag) ? {} : null, isContentEditable: false });
    const out = {};
    for (const tag of ["textarea", "input", "select"]) { const e = ev(field(tag), cdxml); L.paste(e); out[tag] = e.prevented; }
    const editable = ev({ closest: () => null, isContentEditable: true }, cdxml); L.paste(editable); out.editable = editable.prevented;
    const canvasEv = ev({ closest: () => null, isContentEditable: false }, cdxml); L.paste(canvasEv); out.canvas = canvasEv.prevented;
    const plain = ev({ closest: () => null, isContentEditable: false }, "CC(=O)O"); L.paste(plain); out.plain = plain.prevented;
    return out;""")
    assert got == {"textarea": False, "input": False, "select": False, "editable": False, "canvas": True, "plain": False}, got


def test_a_drop_over_the_engines_limit_is_refused_before_the_file_is_read():
    got = run_stub(WIRED + """
    let read = 0;
    const file = size => ({ name: "big.cdxml", size, arrayBuffer: async () => { read++; return new TextEncoder().encode("<CDXML/>").buffer; } });
    const drop = f => ({ dataTransfer: { files: [f], getData: () => "" }, preventDefault() {}, stopPropagation() {} });
    await L["body:drop"](drop(file(2e9)));
    const refused = { read, notes: notes.slice() };
    await L["body:drop"](drop(file(50)));
    return { refused, after: { read, added } };""")
    assert got["refused"] == {"read": 0, "notes": ['draw.import.failed{"message":"refused 2000000000"}']}, got
    assert got["after"] == {"read": 1, "added": ["<CDXML/>"]}, got
