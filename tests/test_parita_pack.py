"""file pack / check-pack / unpack answer the same on the site's .wasm (mzlab-pack-wasm, through Node) and in the app (mzlab-cli): the same parts, manifest and
JSON documents byte for byte, on a synthetic campaign larger than the part size, intact, with a flipped byte and with a missing part. A pack of one part is a valid ZIP.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import zipfile
from pathlib import Path

import pytest
from cargo_util import build, cli_argv

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PART = 1024


def data(n, seed):
    return bytes(((i * 2654435761 + seed) >> 11) & 255 for i in range(n))


def campaign(root):
    files = {"camp/s1.D/AcqData/MSScan.bin": data(9000, 1), "camp/s1.D/AcqData/MSPeak.bin": data(2500, 2), "camp/s1.D/sample.xml": data(700, 3), "camp/s2.wiff": data(5000, 4), "camp/empty.txt": b""}
    for n, b in files.items():
        p = root / n
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b)
    return files


def cli(*a, code=0):
    r = subprocess.run(cli_argv(*a), cwd=ROOT, capture_output=True)
    assert r.returncode == code, (a, r.returncode, r.stderr[-2000:])
    return r.stdout.decode()


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("pkg")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-pack-wasm")
    subprocess.run(["wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), str(ROOT / "target/wasm32-unknown-unknown/release/mzlab_pack_wasm.wasm")], check=True, capture_output=True)
    build("-p", "mzlab-cli")
    return out


def wasm(pkg, tmp, **job):
    jf = tmp / f"job-{len(list(tmp.glob('job-*')))}.json"
    jf.write_text(json.dumps(job))
    r = subprocess.run(["node", str(ROOT / "tests/pack_wasm_node.js"), str(pkg), str(jf)], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads(r.stdout)


def folder_bytes(d):
    return {p.name: p.read_bytes() for p in sorted(d.iterdir())}


def test_the_site_and_the_app_write_the_same_parts_and_documents(pkg, tmp_path):
    src = tmp_path / "src"
    files = campaign(src)
    app, site = tmp_path / "app", tmp_path / "site"
    site.mkdir()
    listing = [[n, str(src / n)] for n in sorted(files)]
    doc_app = cli("file", "pack", str(src / "camp"), "--out", str(app), "--part-size", "1KiB", "--json")
    out = wasm(pkg, tmp_path, op="pack", files=listing, dir=str(site), name="pack", part_size=PART)
    assert "error" not in out, out
    assert out["doc"] == doc_app
    assert folder_bytes(site) == folder_bytes(app)
    # dry run: the same plan, nothing written
    plan_app = cli("file", "pack", str(src / "camp"), "--out", str(tmp_path / "none"), "--part-size", "1KiB", "--dry-run", "--json")
    assert wasm(pkg, tmp_path, op="plan", files=listing, name="pack", part_size=PART)["doc"] == plan_app
    # intact: verify and unpack agree
    man = str(app / "pack.pack.json")
    assert wasm(pkg, tmp_path, op="verify", manifest=man, dir=str(app))["doc"] == cli("file", "check-pack", man, "--json")
    b1, b2 = tmp_path / "b1", tmp_path / "b2"
    d_app = cli("file", "unpack", man, "--out", str(b1), "--json")
    d_wasm = wasm(pkg, tmp_path, op="unpack", manifest=man, dir=str(app), out=str(b2))["doc"]
    assert d_wasm == d_app
    for n, b in files.items():
        assert (b2 / n).read_bytes() == b == (b1 / n).read_bytes()
    # nothing is replaced without force
    assert "err.pack.exists" in wasm(pkg, tmp_path, op="unpack", manifest=man, dir=str(app), out=str(b2))["error"]


def test_a_flipped_byte_and_a_missing_part_are_reported_alike(pkg, tmp_path):
    src = tmp_path / "src"
    campaign(src)
    app = tmp_path / "app"
    cli("file", "pack", str(src / "camp"), "--out", str(app), "--part-size", "1KiB", "--json")
    man = str(app / "pack.pack.json")
    p3, p5 = app / "pack.part0003", app / "pack.part0005"
    b = bytearray(p3.read_bytes())
    b[10] ^= 0xFF
    p3.write_bytes(bytes(b))
    p5.unlink()
    v_app = cli("file", "check-pack", man, "--json", code=3)
    v_wasm = wasm(pkg, tmp_path, op="verify", manifest=man, dir=str(app))["doc"]
    assert v_wasm == v_app
    assert json.loads(v_app)["resend"] == ["pack.part0003", "pack.part0005"]
    u_app = cli("file", "unpack", man, "--out", str(tmp_path / "u1"), "--json", code=3)
    assert wasm(pkg, tmp_path, op="unpack", manifest=man, dir=str(app), out=str(tmp_path / "u2"))["doc"] == u_app


def test_a_pack_of_one_part_is_a_valid_zip(pkg, tmp_path):
    src = tmp_path / "src"
    files = campaign(src)
    app = tmp_path / "app"
    cli("file", "pack", str(src / "camp"), "--out", str(app), "--json")
    z = zipfile.ZipFile(app / "pack.part0001")
    assert z.testzip() is None
    assert {n: z.read(n) for n in z.namelist()} == files
