"""Structure files for the Drawing tab (mzlab/web/disegno-file.js) run by node: which attributes are dropped from CDXML, how the bytes are decoded, what the notice says."""
import json, subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
CDXML = Path(__file__).resolve().parent / "dati" / "cdxml" / "aromatics.cdxml"


def run(body):
    src = f"import {{ tidyCdxml, readStructure, changesNote }} from {(WEB / 'disegno-file.js').as_uri()!r};\n(async () => {{ console.log(JSON.stringify(await (async () => {{ {body} }})())); }})();"
    r = subprocess.run(["node", "--input-type=module", "-e", src], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout)


def test_an_empty_attribute_of_a_cdxml_start_tag_is_dropped_and_named_once():
    got = run("""return [tidyCdxml('<CDXML a="1" BondLength="" b=""><n x=""/></CDXML>'), tidyCdxml('<s>Q=""</s><CDXML/>')];""")
    assert got[0] == {"text": '<CDXML a="1"><n/></CDXML>', "removed": ["BondLength", "b", "x"]}
    assert got[1]["text"] == '<s>Q=""</s><CDXML/>' and got[1]["removed"] == []      # text between tags is never touched


def test_text_that_is_not_cdxml_and_tags_with_single_quotes_are_left_alone():
    got = run("""return [tidyCdxml('<molecule a="">'), tidyCdxml("<CDXML><s t='x y=\\"\\"'/></CDXML>")];""")
    assert got[0]["removed"] == [] and got[0]["text"] == '<molecule a="">'
    assert got[1]["removed"] == []


def test_the_bytes_are_decoded_as_their_mark_or_declaration_says():
    got = run("""
    const file = bytes => ({ arrayBuffer: async () => new Uint8Array(bytes).buffer });
    const xml = enc => `<?xml version="1.0" encoding="${enc}" ?><CDXML BondLength=""><s>\u00fc</s></CDXML>`;
    const u16 = s => { const b = []; for (const c of "\ufeff" + s) { const n = c.charCodeAt(0); b.push(n & 255, n >> 8); } return b; };
    const out = {};
    out.utf8 = await readStructure(file([0xef, 0xbb, 0xbf, ...new TextEncoder().encode(xml("UTF-8"))]));
    out.utf16 = await readStructure(file(u16(xml("UTF-16"))));
    out.latin1 = await readStructure(file([...xml("ISO-8859-1")].map(c => c.charCodeAt(0))));
    out.cdx = await readStructure(file([0x56, 0x6a, 0x43, 0x44, 1, 2, 3]));
    return out;""")
    clean = '<?xml version="1.0" encoding="%s" ?><CDXML><s>\u00fc</s></CDXML>'
    assert got["utf8"]["text"] == clean % "UTF-8" and [c["key"] for c in got["utf8"]["changes"]] == ["draw.import.fix.empty"]
    assert got["utf16"]["text"] == clean % "UTF-8"                                   # the declaration no longer names an encoding the text is not in
    assert [c["key"] for c in got["utf16"]["changes"]] == ["draw.import.fix.charset", "draw.import.fix.empty"]
    assert got["latin1"]["text"] == clean % "ISO-8859-1" and got["latin1"]["changes"][0]["params"] == {"charset": "ISO-8859-1"}
    assert got["cdx"] == {"text": "VmpDRAECAw==", "changes": []}                     # binary CDX goes in as base64, as the Open dialog gives it


def test_the_notice_names_the_file_and_every_change_and_stays_empty_for_none():
    got = run("""
    const t = (k, p) => k + JSON.stringify(p || {});
    return [changesNote("a.cdxml", [], t), changesNote("a.cdxml", [{ key: "draw.import.fix.charset", params: { charset: "UTF-16" } }, { key: "draw.import.fix.empty", params: { names: "BondLength" } }], t)];""")
    assert got[0] == ""
    assert got[1] == 'draw.import.changed{"name":"a.cdxml","what":"draw.import.fix.charset{\\"charset\\":\\"UTF-16\\"}; draw.import.fix.empty{\\"names\\":\\"BondLength\\"}"}'


def test_the_rdkit_fixture_really_carries_the_empty_attribute_the_page_removes():
    text = CDXML.read_text(encoding="utf-8")
    assert 'BondLength=""' in text
    got = run(f"return tidyCdxml({json.dumps(text)}).removed;")
    assert got == ["BondLength"]
