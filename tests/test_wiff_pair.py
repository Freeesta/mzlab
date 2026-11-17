"""The rule «which file is the companion of which» in a SCIEX pair: the engine's (crates/mzlab-lettori/src/pair.rs, through `mzlab-cli pair --json`)
and the page's twin (mzlab/web/wiff-pair.js, run by node) give the same answer for every name of a synthetic folder, case variants and decoys included."""
import json, shutil, subprocess, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = ["Run 1.wiff", "run 1.WIFF.SCAN", "B.WIFF", "B.wiff.scan", "b.Wiff.Scan", "C.wiff", "C.scan", "D.wiff.scan", "e.wiff2", "e.wiff.scan.bak", "A.mzML", "Ünï.wiff", "ünï.wiff.scan"]


def cli_bin():
    b = ROOT / "target" / "debug" / "mzlab-cli"
    if not b.exists():
        subprocess.run(["cargo", "build", "-p", "mzlab-cli"], cwd=ROOT, check=True, capture_output=True)
    return b


def test_site_and_engine_pair_the_same_files():
    d = Path(tempfile.mkdtemp(prefix="wiffpair_"))
    try:
        for n in FILES:
            (d / n).write_bytes(b"x")
        listing = sorted(p.name for p in d.iterdir())  # the real names of the folder (case-insensitive file systems may merge variants)
        js = ("const W=require(%r);const names=%s;const out={};for(const n of names)out[n]=[W.half(n),W.companionName(n),W.pick(n,names)];console.log(JSON.stringify(out))"
              % (str(ROOT / "mzlab/web/wiff-pair.js"), json.dumps(listing)))
        site = json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, text=True).stdout)
        for n in listing:
            half, want, found = site[n]
            r = subprocess.run([str(cli_bin()), "pair", str(d / n), "--json"], capture_output=True, text=True)
            if not half:
                assert r.returncode == 2, n
                assert found is None
                continue
            row = json.loads(r.stdout)["pairs"][0]
            assert row["half"] == half and row["wanted"] == want, n
            cli_found = Path(row["found"]).name if row["found"] else None
            assert cli_found == found, (n, cli_found, found)
            assert r.returncode == (0 if found else 1), n
    finally:
        shutil.rmtree(d)
