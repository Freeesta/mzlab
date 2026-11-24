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


def test_unusable_names_and_part_sizes_are_refused_alike(pkg, tmp_path):
    # names no pack can carry (two that differ only in case, a colon, a file with a folder of its name): the same error key on the site and in the app, in a real run and in a
    # dry run, nothing written
    cases = {
        "err.pack.duplicate": ["A.txt", "a.txt"],
        "err.pack.path": ["c:x.txt"],
        "err.pack.duplicate#folder": ["a", "A/b"],
    }
    for tag, names in cases.items():
        key = tag.split("#")[0]
        src = tmp_path / f"src-{tag}"
        for n in names:
            (src / n).parent.mkdir(parents=True, exist_ok=True)
            (src / n).write_bytes(b"x")
        listing = [[n, str(src / n)] for n in names]
        for op in ("pack", "plan"):
            site = tmp_path / f"site-{tag}-{op}"
            site.mkdir()
            out = wasm(pkg, tmp_path, op=op, files=listing, dir=str(site), name="pack", part_size=PART)
            assert key in out["error"], (op, out)
            assert not list(site.iterdir())
        for extra in ([], ["--dry-run"]):
            cli_out = tmp_path / f"cli-{tag}-{len(extra)}"
            r = subprocess.run(cli_argv("file", "pack", str(src), "--out", str(cli_out), "--json", *extra), cwd=ROOT, capture_output=True)
            assert r.returncode == 3 and key in r.stdout.decode() + r.stderr.decode(), (tag, extra, r.returncode, r.stdout, r.stderr)
            assert not (cli_out / "pack.pack.json").exists()
    src = tmp_path / "src-err.pack.duplicate"
    one = [["A.txt", str(src / "A.txt")]]
    # a part size that is not a number used to loop for ever
    for bad in ("NaN", 0, 1023):
        out = wasm(pkg, tmp_path, op="pack", files=one, dir=str(tmp_path), name="pack", part_size=bad)
        assert "err.run.input" in out["error"], (bad, out)


def test_the_page_code_over_a_stand_in_for_opfs_does_what_the_app_does(pkg, tmp_path):
    # mzlab/web/rust-pack.js on the nodejs build of the module and an in-memory File System API: the app's parts, no overwrite without force, a failed run keeps the older pack,
    # a dry run and a bad manifest are refused before anything is created, a lost part makes the app's tree
    src, app, lost, cli_lost, work = (tmp_path / n for n in ("src", "app", "lost", "cli_lost", "work"))
    campaign(src)
    work.mkdir()
    cli("file", "pack", str(src / "camp"), "--out", str(app), "--part-size", "1KiB", "--json")
    shutil.copytree(app, lost)
    (lost / "pack.part0005").unlink()
    cli("file", "unpack", str(lost / "pack.pack.json"), "--out", str(cli_lost), "--json", code=1)
    r = subprocess.run(["node", str(ROOT / "tests/pack_opfs_node.mjs"), str(pkg), str(src), str(app), str(lost), str(cli_lost), str(work)], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-3000:]
    got = json.loads(r.stdout)
    want = {
        "same_bytes_as_the_app": True, "document": "pack-run/1", "exists_refused": "err.pack.exists", "exists_changed_nothing": True,
        "failure_key": "err.pack.growing", "failure_kept_the_old_pack": True, "force_replaced": True,
        "dry_run_case": "err.pack.duplicate", "dry_run_case_real": "err.pack.duplicate", "dry_run_case_empty": True,
        "dry_run_folder": "err.pack.duplicate", "dry_run_folder_real": "err.pack.duplicate", "dry_run_folder_empty": True,
        "dry_run_colon": "err.pack.path", "dry_run_colon_real": "err.pack.path", "dry_run_colon_empty": True,
        "bad_manifest_stream": "err.pack.path", "bad_manifest_stream_empty": True, "bad_manifest_dotdot": "err.pack.path", "bad_manifest_dotdot_empty": True,
        "bad_manifest_abs": "err.pack.path", "bad_manifest_abs_empty": True,
        "unpack_round_trip": "unpack-run/1", "lost_part_same_tree_as_the_app": True, "nothing_left_over": True, "unpack_exists": "err.pack.exists", "file_where_a_folder_is_wanted": "err.run.output", "file_where_a_folder_is_wanted_left": True,
    }
    assert got == want
