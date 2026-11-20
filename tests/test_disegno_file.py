"""Structure files for the Drawing tab (mzlab/web/disegno-file.js) run by node: what the notice says and which pasted text is CDXML. What the text is changed to
(empty attributes, charsets, base64 CDX) is the engine's `open/1`: its tests are in crates/mzlab-feat/src/chem/structure_file_tests.rs and tests/test_molkit_wasm.py."""
import json, subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def run(body):
    src = f"import {{ changesNote, isCdxml }} from {(WEB / 'disegno-file.js').as_uri()!r};\n(async () => {{ console.log(JSON.stringify(await (async () => {{ {body} }})())); }})();"
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
