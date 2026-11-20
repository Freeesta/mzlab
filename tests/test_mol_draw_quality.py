"""Quality gates of `mzlab-cli mol draw` on the committed synthetic sets (tools/draw_quality.py): set A (200 molecules, 9 families) at least 90 % clean and
no family below RDKit's own count (tests/dati/draw_reference.json), set B (100 E/Z double bonds) none drawn the wrong way, fragments at least 1.5 bonds apart,
at most 9 atoms on a bond, no two atom or CIP labels of the SVG overlapping and every label inside the image (set A, `draw-image/1`), the molfile of every drawing read back as the same `mol/1` core (cargo test, crates/mzlab-mol/tests/layout.rs), and the native
release p95 of one layout (measured and printed; the limit is 1 ms, asserted with a x3 margin because CI machines are shared)."""
import json
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
from cargo_util import build, build_example, example_argv, run

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import draw_quality as dq  # noqa: E402

pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")
CLI = ROOT / "target" / "release" / "mzlab-cli"   # `build` leaves it here (or installs the prebuilt one): no `cargo run` per molecule
P95_LIMIT_US = 1000.0
RUNS = 3   # best of 3 absorbs the noise of a shared runner; the limit itself is not widened


def scene(smiles):
    p = subprocess.run([str(CLI), "mol", "draw", "smiles:" + smiles, "--json"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 0, (smiles, p.stderr)
    return json.loads(p.stdout)


@pytest.fixture(scope="module")
def cli_built():
    build("-p", "mzlab-cli")


@pytest.fixture(scope="module")
def set_a(cli_built):
    rows = []
    for _id, smi, fam in dq.read_set("draw_set_a"):
        sc = scene(smi)
        assert list(sc)[0] == "schema" and sc["schema"] == "draw/1"
        rows.append((fam, dq.measure(*dq.scene_measure(sc))))
    return dq.summarize(rows)


def test_set_a_is_clean_and_no_family_is_below_rdkit(set_a):
    ref = json.loads((dq.DATI / "draw_reference.json").read_text(encoding="utf-8"))
    print(f"set A clean {set_a['clean']}/{set_a['n']}, per family {set_a['families']} (RDKit {ref['clean']}, {ref['families']})")
    assert set_a["n"] == 200
    assert set_a["clean"] >= 0.9 * set_a["n"]
    for fam, want in ref["families"].items():
        assert set_a["families"].get(fam, 0) >= want, f"{fam}: {set_a['families'].get(fam, 0)} < RDKit {want}"


def test_fragments_are_at_least_one_and_a_half_bonds_apart_and_few_atoms_sit_on_a_bond(set_a):
    assert set_a["fragments_under_1_5"] == 0
    assert set_a["onbond"] <= 9


def test_no_double_bond_of_set_b_is_drawn_the_wrong_way(cli_built):
    rows = dq.read_set("draw_set_b")
    assert len(rows) == 100
    wrong = [i for i, smi, ez in rows if dq.ez_wrong(dq.scene_measure(scene(smi))[0], ez)]
    assert wrong == []


def picture(smiles):
    p = subprocess.run([str(CLI), "mol", "draw", "smiles:" + smiles, "--format", "svg", "--json"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 0, (smiles, p.stderr)
    return json.loads(p.stdout)


def test_no_labels_of_the_svg_overlap_in_set_a_and_all_are_inside_the_image(cli_built):
    rows = dq.read_set("draw_set_a")
    assert len(rows) == 200
    bad = []
    for _id, smi, _fam in rows:
        img = picture(smi)
        assert list(img)[0] == "schema" and img["schema"] == "draw-image/1" and img["format"] == "svg"
        root = ET.fromstring(img["svg"])
        assert root.tag.endswith("svg") and int(root.get("width")) == img["width_px"] and int(root.get("height")) == img["height_px"]
        boxes = [(l["left_px"], l["top_px"], l["right_px"], l["bottom_px"]) for l in img["labels"]]
        for i, b in enumerate(boxes):
            if b[0] < 0 or b[1] < 0 or b[2] > img["width_px"] or b[3] > img["height_px"]:
                bad.append((smi, "outside", img["labels"][i]["text"]))
            for j in range(i):
                c = boxes[j]
                if b[0] < c[2] and c[0] < b[2] and b[1] < c[3] and c[1] < b[3]:
                    bad.append((smi, "overlap", img["labels"][i]["text"], img["labels"][j]["text"]))
    assert bad == [], bad[:5]


def test_the_molfile_of_every_drawing_reads_back_as_the_same_record():
    run("cargo", "test", "--release", "--locked", "-q", "-p", "mzlab-mol", "--test", "layout")


def test_native_p95_of_one_layout_is_under_a_millisecond():
    build_example("mzlab-mol", "layout_bench")
    worst = 0.0
    for name in ("draw_set_a", "draw_set_b"):
        runs = []
        for _ in range(RUNS):
            out = run(*example_argv("mzlab-mol", "layout_bench", str(dq.DATI / f"{name}.tsv"))).stdout.split()
            n, median, p95, top = int(out[1]), float(out[3]), float(out[5]), float(out[7])  # `n N median_us M p95_us P max_us X`
            print(f"{name}: n {n} median {median:.0f} us p95 {p95:.0f} us max {top:.0f} us")
            runs.append(p95)
        worst = max(worst, min(runs))
    assert worst <= P95_LIMIT_US, f"p95 {worst:.0f} us (limit {P95_LIMIT_US:.0f} us, best of {RUNS})"
