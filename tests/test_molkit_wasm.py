"""The molkit .wasm (crates/mzlab-mol-wasm) gives the `check/1` and the `cip/1` of a drawing byte for byte like `mzlab-cli mol check` and
`mol cip` and the app fixtures (tests/dati/check_disegno.json, cip_disegno.json and the refusal cip_complex.json, read by
crates/mzlab-desktop/tests/labels.rs too), and a bad molfile as a keyed JSON error; the `substructure_db/1` of a structure list like `mzlab-cli mol search --db` and the app (tests/dati/sub_db.json); the Flow
step `structure.moiety` (`flow_moiety`) like `mzlab-cli flow run --set` (tests/dati/moiety.json); the `highlight/1` of the evidence colours like
`mzlab-cli mol hilite` and the app (tests/dati/hilite_disegno.json); the 2D layout of a spec (`draw`, `draw/1` scene and molfile) like `mzlab-cli mol draw`
and the app, also as KET and CDXML (tests/dati/draw_d1.json, 30 specs; tests/dati/draw_d3.json, the edit patches of `--set edit=`); the `descriptors/1` of a drawing
(`descriptors`, the function mzDraw calls) like `mzlab-cli mol measure` and the app for V2000 and V3000 files (tests/dati/descriptors_disegno.json). Skipped without cargo,
wasm-bindgen or node."""
import json
import zlib
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


def test_check_of_an_aromatic_hydrogen_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    # imidazole written with aromatic bonds: unstated, the reader puts the H on the first N in file order and the check says on which atom;
    # stated by an MRV_IMPLICIT_H data group on the other N, there is nothing to say
    fx = json.loads((ROOT / "tests" / "dati" / "check_disegno.json").read_text(encoding="utf-8"))
    for case, codes in (("aromatic", ["aromaticH"]), ("stated", [])):
        req = tmp_path / f"{case}.json"
        req.write_text(json.dumps({"molfile": fx[f"{case}_request"]["molfile"]}), encoding="utf-8")
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"))
        assert site == cli("mol", "check", str(req), runner=run)
        assert site.strip() == fx[f"{case}_check"]
        assert [(w["code"], w["level"], w["atoms"]) for w in json.loads(site)["warnings"]] == [(c, "note", [0]) for c in codes]


def test_a_bad_molfile_is_a_keyed_error(pkg):
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"})], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.check.molfile"


V2000_ETHANOL = "\n  t\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.2500    0.7000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.5000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  2  3  1  0\nM  END\n"
V3000_ETHANOL = "\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS 3 2 0 0 0\nM  V30 BEGIN ATOM\nM  V30 1 C 0 0 0 0\nM  V30 2 C 1.25 0.7 0 0\nM  V30 3 O 2.5 0 0 0\nM  V30 END ATOM\nM  V30 BEGIN BOND\nM  V30 1 1 1 2\nM  V30 2 1 2 3\nM  V30 END BOND\nM  V30 END CTAB\nM  END\n"


def test_check_of_a_v3000_file_equals_the_v2000_one_and_the_cli_and_a_cut_short_one_is_refused(pkg, tmp_path):
    answers = []
    for name, molfile in (("v2000", V2000_ETHANOL), ("v3000", V3000_ETHANOL)):
        req = tmp_path / f"{name}.json"
        req.write_text(json.dumps({"molfile": molfile}), encoding="utf-8")
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"))
        assert site == cli("mol", "check", str(req), runner=run)
        answers.append(site)
    assert answers[0] == answers[1]
    # the counts line says two bonds, the block holds one: a file cut short is a keyed error on the site and exit 4 in the CLI, never a smaller molecule
    short = json.dumps({"molfile": V3000_ETHANOL.replace("M  V30 2 1 2 3\n", "")})
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), short], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.check.molfile"
    req = tmp_path / "short.json"
    req.write_text(short, encoding="utf-8")
    assert subprocess.run(cli_argv("mol", "check", str(req)), cwd=ROOT, capture_output=True).returncode == 4


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
    # a structure past the work limit: one key and the same evidence in the site, the CLI (exit 1) and the app (tests/dati/cip_complex.json)
    big = json.loads((ROOT / "tests" / "dati" / "cip_complex.json").read_text(encoding="utf-8"))
    req.write_text(json.dumps(big["request"]), encoding="utf-8")
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"), "cip"], cwd=ROOT, capture_output=True, encoding="utf-8")
    err = json.loads(r.stderr)
    assert r.returncode == 1 and (err["error_key"], err["params"]) == (big["error_key"], big["params"])
    c = subprocess.run(cli_argv("mol", "cip", str(req), "--json"), cwd=ROOT, capture_output=True, encoding="utf-8")
    out = json.loads(c.stdout)
    assert c.returncode == 1 and (out["error_key"], out["params"]) == (big["error_key"], big["params"])


def test_descriptors_through_wasm_equal_the_cli_and_the_app_fixture_for_v2000_and_v3000(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "descriptors_disegno.json").read_text(encoding="utf-8"))
    assert any("V3000" in c["request"]["molfile"] for c in fx["cases"])
    for i, c in enumerate(fx["cases"]):
        req = tmp_path / f"req{i}.json"
        req.write_text(json.dumps(c["request"]), encoding="utf-8")
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"), "descr")
        assert site == cli("mol", "measure", str(req), runner=run), c["name"]
        assert site.strip() == c["descriptors"], c["name"]
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"}), "descr"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.descr.molfile"


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


def check_draw_fixture(pkg, tmp_path, name):
    """Every case of tests/dati/NAME: the `.wasm` and the release CLI give each answer it holds (or its error key and exit code); returns the fixture and the sorted exit codes."""
    fx = json.loads((ROOT / "tests" / "dati" / name).read_text(encoding="utf-8"))
    build("-p", "mzlab-cli")
    cli_bin = str(ROOT / "target" / "release" / "mzlab-cli")
    db = tmp_path / "db.csv"
    db.write_text(fx.get("db", ""), encoding="utf-8")
    node = lambda req, mode="draw": subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps(req), mode], cwd=ROOT, capture_output=True)
    for c in fx["cases"]:
        r = c["request"]
        sets = [x for k in ("layout", "name", "edit") if k in r for x in ("--set", f"{k}={r[k]}")] + (["--set", f"db={db}"] if "db" in r else [])
        multi = "\n" in r["spec"]
        for fmt, want in [(f, c[f]) for f in ("scene", "molfile", "ket", "cdxml", "svg", "png") if f in c]:
            p = node({**r, "format": fmt}, "draw_png" if fmt == "png" else "draw")
            assert (p.returncode == 0) == (c["exit"] == 0), c["name"]
            args = ["mol", "draw", "-" if multi else r["spec"], *sets, *(["--json"] if fmt == "scene" else []), *(["--format", fmt] if fmt in ("ket", "cdxml", "svg", "png") else [])]
            q = subprocess.run([cli_bin, *args], cwd=ROOT, input=r["spec"].encode() if multi else None, capture_output=True)
            if c["exit"]:
                # the same failure everywhere: the key of the wasm, the key of the CLI and its exit code
                assert json.loads(p.stderr)["error_key"] == c["error_key"], c["name"]
                assert q.returncode == c["exit"], (c["name"], fmt)
                if fmt == "scene":
                    err = [x for x in (q.stdout.decode() + "\n" + q.stderr.decode()).splitlines() if x.startswith('{"schema":"error/1"')]
                    assert json.loads(err[0])["error_key"] == c["error_key"], c["name"]
                continue
            if fmt == "png":
                # the same bytes in the wasm and the CLI, and the file the fixture describes
                assert p.stdout == q.stdout and (len(p.stdout), zlib.crc32(p.stdout)) == (want["len"], want["crc32"]), c["name"]
                continue
            assert p.stdout.decode() == want + "\n", (c["name"], fmt)  # the node runner adds one newline
            assert q.stdout.decode() == want + ("\n" if fmt == "scene" else ""), (c["name"], fmt)  # the CLI ends a JSON answer with a newline
            if fmt in ("ket", "cdxml"):
                # `--json` carries the same text in the scene, and so does a scene request of the .wasm
                j = subprocess.run([cli_bin, *args, "--json"], cwd=ROOT, input=r["spec"].encode() if multi else None, capture_output=True)
                w = node({**r, "format": "scene", "file": fmt})
                assert list(json.loads(j.stdout))[0] == "schema" and json.loads(j.stdout)["file"] == want, (c["name"], fmt)
                assert w.stdout == j.stdout, (c["name"], fmt)
    for im in fx.get("images", []):
        r = im["request"]
        sets = [x for k in ("scale", "cip", "highlight", "name") if k in r for x in ("--set", f"{k}={r[k]}")]
        q = subprocess.run([cli_bin, "mol", "draw", r["spec"], "--format", r["format"], *sets, *(["--json"] if r.get("json") else [])], cwd=ROOT, capture_output=True)
        if im["png"]:
            p = node(r, "draw_png")
            assert p.stdout == q.stdout and (len(p.stdout), zlib.crc32(p.stdout)) == (im["png"]["len"], im["png"]["crc32"]), im["name"]
        else:
            p = node(r)
            assert p.stdout.decode() == im["text"] + "\n", im["name"]
            assert q.stdout.decode() == im["text"] + ("\n" if r.get("json") else ""), im["name"]
    return fx, sorted(c["exit"] for c in fx["cases"] if c["exit"])


def test_draw_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx, exits = check_draw_fixture(pkg, tmp_path, "draw_d1.json")
    assert len(fx["cases"]) == 30
    assert exits == [2, 2, 5]


def test_draw_edit_patches_through_wasm_equal_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx, exits = check_draw_fixture(pkg, tmp_path, "draw_d3.json")
    assert len(fx["cases"]) >= 20
    # a failed operation is a usage error with its key (exit 2); only a drawing over the size limit is exit 1
    big = [c for c in fx["cases"] if c.get("error_key") == "err.mol.draw.tooBig"]
    assert big and exits.count(1) == len(big) and set(exits) == {1, 2}


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
    # a size that is no size is no request, as on the app's route
    for bad in ("-1", "NaN"):
        r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), bad, "size"], cwd=ROOT, capture_output=True, encoding="utf-8")
        assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.open.request", bad
    c = subprocess.run(cli_argv("mol", "open", str(big)), cwd=ROOT, capture_output=True, encoding="utf-8")
    e = json.loads(c.stdout)
    assert c.returncode == 1 and e["schema"] == "error/1" and e["error_key"] == "error.open.tooBig", c.stdout
    assert e["params"] == refusal["params"], c.stdout
