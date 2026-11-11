"""Every mzML `mzlab-cli convert` writes from the synthetic dataset of each reader family is well-formed XML for a second, independent parser
(lxml/libxml2, strict: no recovery), in both compressions. The readers modules write the same bytes (`test_lettori_parita.py`), so this covers the
site too. A vendor model text that is not a CV term name (angle brackets, quotes) stays in a userParam, and MS:1000031 keeps its CV name.
lxml is a test-only dependency (`.[test]`); without it the test is skipped (MZLAB_GOLDEN_STRICT=1: fails)."""
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from test_lettori_parita import create, mzxml, need, run  # noqa: E402

etree = pytest.importorskip("lxml.etree")
NS = "{http://psi.hupo.org/ms/mzml}"
FAMILIES = ["aperti", "agilent", "waters", "shimadzu", "sciex", "thermo", "bruker", "andi"]


@pytest.fixture(scope="module")
def cli(tmp_path_factory):
    need(shutil.which("cargo"), "cargo missing")
    b = run(["cargo", "build", "--release", "--locked", "-p", "mzlab-cli"])
    assert b.returncode == 0, b.stderr[-2000:]
    return {"cli": ROOT / "target/release/mzlab-cli", "tmp": tmp_path_factory.mktemp("secondo")}


def convert(t, ds: Path, tag: str, *extra: str) -> Path:
    out = t["tmp"] / f"out_{tag}"
    r = run([str(t["cli"]), "convert", str(ds), "--out", str(out), "--fresh", "--workers", "1", *extra])
    assert r.returncode == 0, r.stdout[-1500:] + r.stderr[-1500:]
    files = sorted(out.glob("*.mzML"))
    assert len(files) == 1, files
    return files[0]


def strict(p: Path):
    """The tree of `p` read by libxml2 with no recovery: any well-formedness fault raises."""
    return etree.parse(str(p), etree.XMLParser(recover=False, huge_tree=True, resolve_entities=False, no_network=True))


@pytest.mark.parametrize("compression", ["standard", "smallest"])
@pytest.mark.parametrize("fam", FAMILIES)
def test_every_converted_fixture_is_well_formed_for_libxml2(cli, fam, compression):
    ds = create(fam, cli["tmp"] / f"in_{fam}")
    root = strict(convert(cli, ds, f"{fam}_{compression}", "--compression", compression)).getroot()
    assert root.tag == NS + "indexedmzML"
    assert root.find(f"{NS}mzML/{NS}run/{NS}spectrumList") is not None


def test_a_model_text_is_a_user_param_not_a_cv_name(cli):
    d = cli["tmp"] / "in_model"
    d.mkdir(parents=True, exist_ok=True)
    p = mzxml(d / "modello.mzXML", n=3)
    p.write_bytes(p.read_bytes().replace(b'value="Modello X"', b'value="&lt;unknown&gt; &amp; &quot;X&quot;"'))
    tree = strict(convert(cli, p, "model"))
    ic = tree.find(f".//{NS}instrumentConfiguration")
    terms = [(c.get("accession"), c.get("name")) for c in ic.findall(f"{NS}cvParam")]
    assert ("MS:1000031", "instrument model") in terms, terms
    users = {u.get("name"): u.get("value") for u in ic.findall(f"{NS}userParam")}
    assert users.get("instrument model") == '<unknown> & "X"', users
