"""The pipeline of mzFinder in WebAssembly (crates/mzlab-feat-wasm) gives the same bytes as the native engine: the `.mzfeatf` of every file, the `.mzfeat` of the
series and the one with the graph of the proofs have the same SHA-256 whether the example `native` or the `.wasm` (through Node) makes them. The series is the
synthetic one of tools/feat_mirato.py (profile `piccolo`; the tests of the `Spill` of the page use the profile `debole`). Skipped without cargo, the wasm32 target, wasm-bindgen, node or numpy."""
import re
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import pytest
from cargo_util import build

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PARENT = ["--parent", "C21H32N2O2", "--rules", str(ROOT / "TP_Mine/data/regole_tp_2.json"), "--parent-mz", "345.253655", "--parent-rt", "1.0"]


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    """The synthetic series, the native example and the .wasm (nodejs target), built once."""
    pytest.importorskip("numpy")
    root = tmp_path_factory.mktemp("feat")
    ser = root / "ser"
    # the series (python) is independent of the two builds (cargo takes the lock of the target folder one after the other): they run at the same time
    with ThreadPoolExecutor(2) as pool:
        made = pool.submit(run, "python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser))
        build("-p", "mzlab-feat-wasm", "--example", "native", runner=run)
        build("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", runner=run)
        made.result()
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    assert len(idx) == 6
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    return root, idx, pkg


def native(root, idx, name, *extra):
    out = root / name
    out.mkdir(exist_ok=True)
    got = run("target/release/examples/native", str(out), *idx, *PARENT, *extra)
    return got, out


def wasm(root, idx, pkg, name, *extra):
    out = root / name
    out.mkdir(exist_ok=True)
    return run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, *PARENT, *extra), out


def test_wasm_and_native_give_the_same_mzfeat_bytes(built):
    root, idx, pkg = built
    nat, nat_dir = native(root, idx, "nat")
    got, wasm_dir = wasm(root, idx, pkg, "wasm")
    assert got == nat
    assert "series_evid.mzfeat" in got and "hypotheses" in got
    for f in sorted(p.name for p in nat_dir.iterdir()):
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f


ADAP = '{"fill": true, "layers": [{"step": "features.detect", "params": {"algorithm": "adap_localmin", "floor": 10000, "min_height": 30000, "smooth_window": 5, "noise_factor": 1.5}}]}'


def test_the_adap_localmin_detection_gives_the_same_bytes_native_and_in_node(built):
    """The algorithm `adap_localmin` of the Flow step `features.detect` (layers of a Flow run) writes the same `.mzfeatf` of every file and the same
    `series.mzfeat` in the .wasm as natively, and not those of the default `masstrace`. `series_evid.mzfeat` is left out: the proofs entries of the
    .wasm read the recipe without its layers (`recipe_from_json`), so a layered recipe gives other proofs there whatever the algorithm."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_adap", "--recipe", ADAP)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_adap", "--recipe", ADAP)
    _, mt_dir = native(root, idx, "nat_mt", "--recipe", '{"fill": true, "layers": []}')
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    assert (nat_dir / "series.mzfeat").read_bytes() != (mt_dir / "series.mzfeat").read_bytes(), "adap_localmin and masstrace give the same table: the test proves nothing"


JOIN = '{"fill": true, "layers": [{"step": "features.align", "params": {"algorithm": "join"}}, {"step": "features.gapfill", "params": {"algorithm": "peak_finder"}}]}'


def test_the_join_aligner_and_the_peak_finder_give_the_same_bytes_native_and_in_node(built):
    """The algorithms `join` of `features.align` and `peak_finder` of `features.gapfill` write the same files in the .wasm as natively, and not the
    table of the defaults `groups` and `integrate`. `series_evid.mzfeat` is left out for the reason given in the test above."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_join", "--recipe", JOIN)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_join", "--recipe", JOIN)
    _, def_dir = native(root, idx, "nat_groups", "--recipe", '{"fill": true, "layers": []}')
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    assert (nat_dir / "series.mzfeat").read_bytes() != (def_dir / "series.mzfeat").read_bytes(), "join and groups give the same table: the test proves nothing"



GROUPER = '{"fill": true, "layers": [{"step": "features.detect", "params": {"isotopes": "grouper", "iso_rt_tol": 0.1, "iso_representative": "lowest_mz"}}]}'


def test_the_isotope_grouper_gives_the_same_bytes_native_and_in_node(built):
    """The isotope grouping `grouper` of `features.detect` (the isotope grouper of mzmine) writes the same `.mzfeatf` of every file and the same
    `series.mzfeat` in the .wasm as natively, and not the files of the default `averagine`. `series_evid.mzfeat` is left out for the reason given
    in the test of `adap_localmin`."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_grouper", "--recipe", GROUPER)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_grouper", "--recipe", GROUPER)
    _, def_dir = native(root, idx, "nat_averagine", "--recipe", '{"fill": true, "layers": []}')
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    feats = [f for f in names if f.endswith(".mzfeatf")]
    assert feats and any((nat_dir / f).read_bytes() != (def_dir / f).read_bytes() for f in feats), "grouper and averagine give the same files: the test proves nothing"

FINDER = '{"fill": true, "layers": [{"step": "features.detect", "params": {"isotopes": "finder", "finder_auto": false}}]}'


def test_the_isotope_finder_gives_the_same_bytes_native_and_in_node(built):
    """The isotope grouping `finder` of `features.detect` (the isotope finder of mzmine, on the apex spectrum of each feature) writes the same
    `.mzfeatf` of every file and the same `series.mzfeat` in the .wasm as natively, and not the files of the default `averagine`.
    `series_evid.mzfeat` is left out for the reason given in the test of `adap_localmin`."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_finder", "--recipe", FINDER)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_finder", "--recipe", FINDER)
    _, def_dir = native(root, idx, "nat_averagine_f", "--recipe", '{"fill": true, "layers": []}')
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    feats = [f for f in names if f.endswith(".mzfeatf")]
    assert feats and any((nat_dir / f).read_bytes() != (def_dir / f).read_bytes() for f in feats), "finder and averagine give the same files: the test proves nothing"


WINDOW = '{"fill": true, "layers": [{"step": "features.detect", "params": {"area_from": "window"}}]}'


def test_the_window_area_gives_the_same_bytes_native_and_in_node(built):
    """`area_from` = `window` of `features.detect` (the area of a row summed over its quality window) writes the same `.mzfeatf` of every file and
    the same `series.mzfeat` in the .wasm as natively, and not the files of the default `channel`. `series_evid.mzfeat` is left out as above."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_window", "--recipe", WINDOW)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_window", "--recipe", WINDOW)
    _, def_dir = native(root, idx, "nat_channel", "--recipe", '{"fill": true, "layers": []}')
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    feats = [f for f in names if f.endswith(".mzfeatf")]
    assert feats and any((nat_dir / f).read_bytes() != (def_dir / f).read_bytes() for f in feats), "window and channel give the same files: the test proves nothing"


XIC = '{"fill": true, "layers": [{"step": "features.detect", "params": {"area_from": "xic"}}]}'


def test_the_xic_area_gives_the_same_bytes_native_and_in_node(built):
    """`area_from` = `xic` (the trapezoid of the quality trace on the real retention times) writes the same files in the .wasm as natively, and
    not the files of `window`. `series_evid.mzfeat` is left out as above."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_xic", "--recipe", XIC)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_xic", "--recipe", XIC)
    _, win_dir = native(root, idx, "nat_xic_window", "--recipe", WINDOW)
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    feats = [f for f in names if f.endswith(".mzfeatf")]
    assert feats and any((nat_dir / f).read_bytes() != (win_dir / f).read_bytes() for f in feats), "xic and window give the same files: the test proves nothing"


WIDE = '{"fill": true, "layers": [{"step": "features.detect", "params": {"stitch": "wide"}}]}'


def test_the_wide_stitch_gives_the_same_bytes_native_and_in_node(built):
    """`stitch` = `wide` of `features.detect` (touching pieces of neighbouring channels joined at any height) writes the same files in the .wasm as
    natively, and not the files of the default `strict`. `series_evid.mzfeat` is left out as above."""
    root, idx, pkg = built
    _, nat_dir = native(root, idx, "nat_wide", "--recipe", WIDE)
    _, wasm_dir = wasm(root, idx, pkg, "wasm_wide", "--recipe", WIDE)
    _, def_dir = native(root, idx, "nat_strict", "--recipe", '{"fill": true, "layers": []}')
    names = sorted(p.name for p in nat_dir.iterdir() if p.name != "series_evid.mzfeat")
    assert "series.mzfeat" in names and len(names) > 1
    for f in names:
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f
    feats = [f for f in names if f.endswith(".mzfeatf")]
    assert feats and any((nat_dir / f).read_bytes() != (def_dir / f).read_bytes() for f in feats), "wide and strict give the same files: the test proves nothing"

@pytest.mark.parametrize("mode", ["ion", "family"])
def test_a_reduced_budget_gives_the_same_sha256_native_and_in_node(built, mode):
    """The contract of memory: the `.mzfeatf`, the `.mzfeat` and the one with `evid` have the same SHA-256 in memory and under a budget (slabs of m/z,
    graph made identity by identity), natively and in the .wasm. Per family too (the groups of ions made slab by slab, the votes read a file at a time)."""
    root, idx, pkg = built
    recipe = ("--recipe", '{"mode":"%s"}' % mode)
    ref, _ = native(root, idx, f"ref-{mode}", *recipe)
    for mb in (192, 3, 1.5) if mode == "family" else (192, 1, 0.5):
        nat, _ = native(root, idx, f"nat-{mode}-{mb}", *recipe, "--budget-mb", str(mb))
        assert nat == ref, f"native, {mb} MB"
        got, _ = wasm(root, idx, pkg, f"wasm-{mode}-{mb}", *recipe, "--budget-mb", str(mb))
        assert got == ref, f"wasm, {mb} MB"


def test_the_alignment_tells_the_page_the_same_steps_native_and_in_node(built):
    """The progress of `align_series_to` (slabs and files of gap filling, at most once per 1 %) ends where the native one ends, with the same bytes."""
    root, idx, pkg = built
    args = (*idx, *PARENT, "--recipe", '{"mode":"family"}', "--budget-mb", "1.5")
    got = []
    for name, cmd in (("nat-steps", ["target/release/examples/native"]), ("wasm-steps", ["node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg)])):
        (root / name).mkdir(exist_ok=True)
        p = subprocess.run([*cmd, str(root / name), *args], cwd=ROOT, capture_output=True, encoding="utf-8")
        assert p.returncode == 0, p.stderr
        steps, done, total = map(int, re.search(r"align steps (\d+) last (\d+) of (\d+)", p.stderr).groups())
        got.append((p.stdout, steps, done, total))
    assert got[0] == got[1]
    _, steps, done, total = got[0]
    assert total > 1 and done == total and 1 < steps <= 102, got[0][1:]

@pytest.fixture(scope="module")
def weak(built):
    """The series `debole` (tools/feat_mirato.py: `piccolo` plus thousands of weak features in every file) and the bytes of the native engine in memory."""
    root, _, _ = built
    ser = root / "ser-debole"
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "debole", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    assert len(idx) == 6
    ref, _ = native(root, idx, "ref-debole", "--recipe", '{"mode":"family"}')
    return idx, ref


def spilling(built, idx, name, mb, spill, *extra):
    root, _, pkg = built
    out = root / name
    out.mkdir(exist_ok=True)
    args = ["--budget-mb", str(mb)] + (["--spill", spill] if spill else []) + list(extra)
    return subprocess.run(["node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, *PARENT, "--recipe", '{"mode":"family"}', *args],
                          cwd=ROOT, capture_output=True, encoding="utf-8")


@pytest.mark.parametrize("mb", [1.5, 0.5])
def test_the_weak_tables_in_a_spill_of_the_page_give_the_same_sha256(built, weak, mb):
    """The optional `spill` of `align_series_to` (an object of the page with `put` and `get`) changes where the weak tables wait, not the bytes: with the
    series `debole` the weak tables do not fit in half of what is free, so each of the 6 files goes to `put` and every slab reads all the files with `get`."""
    idx, ref = weak
    p = spilling(built, idx, f"spill-{mb}", mb, "1")
    assert p.returncode == 0, p.stderr
    assert p.stdout == ref, f"{mb} MB"
    puts, gets = map(int, re.search(r"spill puts (\d+) gets (\d+)", p.stderr).groups())
    assert puts == len(idx)
    assert gets >= 2 * len(idx) and gets % len(idx) == 0       # more than one slab, each reading every file


@pytest.mark.parametrize("mb,spill", [(192, None), (1.5, "1")])
def test_the_weak_candidates_of_the_detection_give_the_same_sha256_with_fewer_reads(built, weak, mb, spill):
    """The weak candidates the worker keeps from each detection (`detect_weak_file_to`, as the app keeps its `.weak` files), handed to `align_series_to`,
    give the bytes of the alignment that detects every file again, in memory and with the weak tables in a spill; the alignment then reads the indexes less."""
    idx, ref = weak
    again = spilling(built, idx, f"again-{mb}", mb, spill)
    p = spilling(built, idx, f"kept-{mb}", mb, spill, "--weak", "1")
    assert again.returncode == 0 and p.returncode == 0, p.stderr
    assert p.stdout == ref == again.stdout, f"{mb} MB"
    reads = [int(re.search(r"align reads (\d+)", q.stderr).group(1)) for q in (p, again)]
    assert reads[0] < reads[1], reads


def test_a_budget_where_the_weak_tables_do_not_fit_is_refused_without_a_spill(built, weak):
    idx, _ = weak
    p = spilling(built, idx, "no-spill", 1.5, None)
    assert p.returncode != 0
    assert '"error_key":"error.memoryBudget"' in p.stderr and '"fase":"align"' in p.stderr


def test_a_get_that_throws_is_a_refusal_with_the_key_and_not_a_panic(built, weak):
    idx, _ = weak
    p = spilling(built, idx, "get-fails", 1.5, "fail")
    assert p.returncode != 0
    assert '"error_key":"error.memoryBudget"' in p.stderr and '"fase":"align"' in p.stderr
    assert "panick" not in p.stderr and "unreachable" not in p.stderr


def test_a_budget_that_fits_nothing_is_refused_with_the_key_and_the_files_saved(built):
    root, idx, pkg = built
    out = root / "refused"
    out.mkdir(exist_ok=True)
    p = subprocess.run(["node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, "--recipe", '{"mode":"ion"}', "--budget-mb", "0.05"],
                       cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode != 0
    assert "error.memoryBudget" in p.stderr and "align" in p.stderr
    # the phase before is already saved
    assert len(list(out.glob("*.mzfeatf"))) == len(idx) and not (out / "series_evid.mzfeat").exists()


@pytest.fixture(scope="module")
def built_sizes(built, tmp_path_factory):
    """The same series with a .wasm that also carries `untargeted` (default features + untargeted)."""
    root, idx, _ = built
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--features", "untargeted", runner=run)
    pkg = tmp_path_factory.mktemp("pkg_sizes")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    return idx, pkg


def test_fewer_sizes_than_files_is_an_error_and_not_a_trap(built_sizes):
    idx, pkg = built_sizes
    out = run("node", str(ROOT / "tests" / "feat_wasm_sizes_node.js"), str(pkg), str(ROOT / "TP_Mine/data/regole_tp_2.json"), *idx)
    assert out.splitlines() == ["ok align_untargeted_to", "ok align_untargeted_to afterwards", "ok build_evidence_to", "ok build_evidence_to afterwards"], out
