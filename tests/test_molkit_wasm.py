"""The molkit .wasm (crates/mzlab-mol-wasm) gives the `check/1` and the `cip/1` of a drawing byte for byte like `mzlab-cli mol check` and
`mol cip` and the app fixtures (tests/dati/check_disegno.json and cip_disegno.json, read by crates/mzlab-desktop/tests/labels.rs too), and a bad
molfile as a keyed JSON error; the `substructure_db/1` of a structure list like `mzlab-cli mol search --db` and the app (tests/dati/sub_db.json); the Flow
step `structure.moiety` (`flow_moiety`) like `mzlab-cli flow run --set` (tests/dati/moiety.json); the `highlight/1` of the evidence colours like
`mzlab-cli mol hilite` and the app (tests/dati/hilite_disegno.json). Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build, cli, cli_argv

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("molkit")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-mol-wasm", runner=run)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_mol_wasm.wasm")
    return out


def test_check_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "check_disegno.json").read_text(encoding="utf-8"))
    req = tmp_path / "req.json"
    req.write_text(json.dumps(fx["request"]), encoding="utf-8")
    site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"))
    assert site == cli("mol", "check", str(req), runner=run)
    assert site.strip() == fx["check"]
    assert [w["code"] for w in json.loads(site)["warnings"]] == ["valence", "valenceCharge", "isotope", "wedgeNoCentre", "pieces"]


def test_check_of_an_unstated_aromatic_hydrogen_through_wasm_equals_the_cli(pkg, tmp_path):
    # imidazole written with aromatic bonds: the reader puts the H on the first N in file order and the check says on which atom
    atoms = "".join(f"{i:10.4f}{0:10.4f}{0:10.4f} {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for i, e in enumerate("NCNCC"))
    bonds = "".join("%3d%3d  4  0  0  0  0\n" % (i + 1, (i + 1) % 5 + 1) for i in range(5))
    req = tmp_path / "req.json"
    req.write_text(json.dumps({"molfile": "\n  qqq\n\n  5  5  0  0  0  0  0  0  0  0999 V2000\n" + atoms + bonds + "M  END"}), encoding="utf-8")
    site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"))
    assert site == cli("mol", "check", str(req), runner=run)
    (w,) = json.loads(site)["warnings"]
    assert (w["code"], w["level"], w["atoms"]) == ("aromaticH", "note", [0])


def test_a_bad_molfile_is_a_keyed_error(pkg):
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"})], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.check.molfile"


def test_cip_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "cip_disegno.json").read_text(encoding="utf-8"))
    req = tmp_path / "req.json"
    req.write_text(json.dumps(fx["request"]), encoding="utf-8")
    site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"), "cip")
    assert site == cli("mol", "cip", str(req), runner=run)
    assert site.strip() == fx["cip"]
    assert json.loads(site) == {"schema": "cip/1", "atoms": [{"atom": 1, "label": "R"}], "bonds": [{"a": 3, "b": 4, "label": "E"}]}
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"}), "cip"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.cip.molfile"


def test_substructure_over_a_list_through_wasm_equals_the_cli_and_the_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "sub_db.json").read_text(encoding="utf-8"))
    for k, c in enumerate(fx["cases"]):
        r = c["request"]
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), json.dumps(r), "sub")
        assert site.strip() == c["answer"], c["name"]
        db = tmp_path / f"list{k}.txt"
        db.write_text(r["db"], encoding="utf-8")
        sets = (["--set", "max=1"] if "max" in r else []) + (["--set", "stereo=1"] if r.get("stereo") else [])
        assert site == cli("mol", "search", "--sub", r["query"], "--db", str(db), *sets, runner=run)
    bad = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"query": "C", "db": "# none\n"}), "sub"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert bad.returncode == 1 and json.loads(bad.stderr)["error_key"] == "error.sub.db"


def test_moiety_step_through_wasm_equals_flow_run_with_set_and_the_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "moiety.json").read_text(encoding="utf-8"))
    table = tmp_path / "list.csv"
    table.write_bytes(fx["table"].encode("utf-8"))
    plan = tmp_path / "plan.json"
    plan.write_text(json.dumps({"format": "mzflow/2", "inputs": [{"name": "list.csv", "type": "candidates"}],
                                "steps": [{"id": "m", "step": "structure.moiety@1", "in": {"candidates": "file:list.csv"}, "params": {}}]}), encoding="utf-8")
    defaults = {"query": "", "column": "smiles", "stereo": False, "keep": "containing"}
    for k, c in enumerate(fx["cases"]):
        params = json.loads(c["params"])
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({**defaults, **params}), "moiety", str(table))
        assert site == c["out"], c["params"]
        sets = [a for n, v in params.items() for a in ("--set", f"m.{n}={json.dumps(v) if isinstance(v, bool) else v}")]
        out = tmp_path / f"run{k}"
        cli("flow", "run", str(plan), "--in", str(table), "--out", str(out), *sets, runner=run)
        assert (out / "m.csv").read_text(encoding="utf-8") == site, c["params"]
    for c in fx["errors"]:
        bad = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), c["params"], "moiety", str(table)], cwd=ROOT, capture_output=True, encoding="utf-8")
        assert bad.returncode == 1 and json.loads(bad.stderr)["error_key"] == c["key"], c["params"]


def test_hilite_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "hilite_disegno.json").read_text(encoding="utf-8"))
    for req_key, want in (("request", "hilite"), ("lr", "hilite_lr")):
        req = tmp_path / (req_key + ".json")
        req.write_text(json.dumps(fx[req_key]), encoding="utf-8")
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"), "hilite")
        assert site == cli("mol", "hilite", str(req), runner=run)
        assert site.strip() == fx[want]
    for bad, key in fx["errors"]:
        r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps(bad), "hilite"], cwd=ROOT, capture_output=True, encoding="utf-8")
        assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == key


def test_open_through_wasm_equals_the_cli_for_every_fixture_and_feeding_variant(pkg, tmp_path):
    """`open/1` of a structure file: the `.wasm` and `mzlab-cli mol open` give the same bytes for the CDXML files of tests/dati/cdxml and for the variants
    of one of them (byte-order mark, UTF-16 both ways, 8-bit text, CRLF, no declaration, binary CDX, text that is not XML)."""
    import base64
    base = (ROOT / "tests" / "dati" / "cdxml" / "aromatics.cdxml").read_text(encoding="utf-8")
    decl = lambda enc: base.replace('encoding="UTF-8"', f'encoding="{enc}"')
    note = base.replace("</page>", '<t id="900"><s>Zürich</s></t></page>')
    variants = {
        "bom": b"\xef\xbb\xbf" + base.encode(),
        "utf16le": b"\xff\xfe" + decl("UTF-16").encode("utf-16-le"),
        "utf16be": b"\xfe\xff" + decl("UTF-16").encode("utf-16-be"),
        "latin1": note.replace('encoding="UTF-8"', 'encoding="ISO-8859-1"').encode("latin-1"),
        "utf16le_nomark": decl("UTF-16").encode("utf-16-le"),
        "utf8_bad_bytes": b'<?xml version="1.0" encoding="utf-8"?><CDXML a=""><s>\xff</s></CDXML>',
        "crlf": base.replace("\n", "\r\n").encode(),
        "cdx": b"VjCD0100\x03\x04\x02\x01" + bytes(range(0, 250)),
        "text": b"this is not a structure\n",
        "empty": b"",
    }
    files = {p.name: p.read_bytes() for p in sorted((ROOT / "tests" / "dati" / "cdxml").glob("*.cdxml"))} | {f"{k}.bin": v for k, v in variants.items()}
    for name, data in files.items():
        f = tmp_path / name
        f.write_bytes(data)
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), str(f), "open")
        assert site == cli("mol", "open", str(f), runner=run), name
        got = json.loads(site)
        assert got["schema"] == "open/1" and list(got) == ["schema", "text", "changes"], name
        changed = subprocess.run(cli_argv("mol", "open", str(f), "--check"), cwd=ROOT, capture_output=True, encoding="utf-8")
        assert changed.stdout == site and changed.returncode == (1 if got["changes"] else 0), name
    # what each change does to the text
    empty = json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "aromatics.cdxml"), "open"))
    assert empty["changes"] == [{"fix": "empty", "names": ["BondLength"]}] and "BondLength" not in empty["text"]
    assert json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "utf16le.bin"), "open"))["changes"][0] == {"fix": "charset", "charset": "UTF-16"}
    nomark = json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "utf16le_nomark.bin"), "open"))
    assert nomark["changes"][0] == {"fix": "charset", "charset": "UTF-16LE"} and 'encoding="UTF-8"' in nomark["text"]
    bad = json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "utf8_bad_bytes.bin"), "open"))
    assert bad["changes"] == [{"fix": "charset", "charset": "utf-8"}, {"fix": "replaced", "count": 1}, {"fix": "empty", "names": ["a"]}]
    latin = json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "latin1.bin"), "open"))
    assert latin["changes"][0] == {"fix": "charset", "charset": "ISO-8859-1"} and "Zürich" in latin["text"]
    cdx = json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "cdx.bin"), "open"))
    assert cdx["changes"] == [] and base64.b64decode(cdx["text"]) == variants["cdx"]
    assert json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), str(tmp_path / "empty.bin"), "open")) == {"schema": "open/1", "text": "", "changes": []}


def test_open_refuses_a_file_over_the_limit_the_same_way_in_the_wasm_and_the_cli(pkg, tmp_path):
    """8 MiB is the limit of `open/1`: the `.wasm` throws `error.open.tooBig` with the size, the CLI answers one `error/1` with exit 1."""
    big = tmp_path / "big.cdxml"
    size = (8 << 20) + 1000
    big.write_bytes(b" " * size)
    refusal = {"error_key": "error.open.tooBig", "params": {"size": str(size), "limit": str(8 << 20)}}
    site = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), str(big), "open"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert site.returncode == 1 and json.loads(site.stderr) == refusal
    # the size alone, as the page asks it before reading a dropped file: the same refusal, and the limit when the file is read
    asked = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), str(size), "size"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert asked.returncode == 1 and json.loads(asked.stderr) == refusal
    assert json.loads(run("node", "tests/molkit_wasm_node.js", str(pkg), "1000", "size")) == {"limit": 8 << 20}
    c = subprocess.run(cli_argv("mol", "open", str(big)), cwd=ROOT, capture_output=True, encoding="utf-8")
    e = json.loads(c.stdout)
    assert c.returncode == 1 and e["schema"] == "error/1" and e["error_key"] == "error.open.tooBig", c.stdout
    assert e["params"] == refusal["params"], c.stdout
