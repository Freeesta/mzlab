"""CITATION.cff (read by GitHub) and .zenodo.json (the only file Zenodo reads when both exist) must say the same thing."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CFF = (ROOT / "CITATION.cff").read_text(encoding="utf-8")
ZEN = json.loads((ROOT / ".zenodo.json").read_text(encoding="utf-8"))


def _cff(key):
    m = re.search(rf"^{key}:\s*\"?(.*?)\"?\s*$", CFF, re.M)
    assert m, key
    return m.group(1)


def test_title_and_license_are_the_same():
    assert _cff("title") == ZEN["title"]
    assert _cff("license") == ZEN["license"]


def test_author_and_affiliation_are_the_same():
    fam = re.search(r"family-names:\s*(.+)", CFF).group(1).strip()
    giv = re.search(r"given-names:\s*(.+)", CFF).group(1).strip()
    aff = re.search(r"affiliation:\s*\"?(.*?)\"?\s*$", CFF, re.M).group(1)
    c = ZEN["creators"][0]
    assert c["name"] == f"{fam}, {giv}" and c["affiliation"] == aff


def test_orcid_is_in_both_or_in_neither():
    cff = re.search(r"^\s*orcid:\s*\"https://orcid\.org/(\d{4}-\d{4}-\d{4}-\d{3}[\dX])\"", CFF, re.M)
    zen = ZEN["creators"][0].get("orcid")
    assert (cff.group(1) if cff else None) == zen, "write the ORCID in CITATION.cff AND in .zenodo.json (docs/zenodo.md, step 6)"


def test_keywords_are_the_same():
    block = re.search(r"^keywords:\n((?:  - .+\n)+)", CFF, re.M).group(1)
    assert {k.strip().lower() for k in re.findall(r"  - (.+)", block)} == {k.lower() for k in ZEN["keywords"]}


def test_zenodo_file_has_no_version_and_documents_the_site_instead_of_being_identical():
    assert "version" not in ZEN, "the version of the deposit comes from the tag of the release"
    rels = {r["identifier"]: r["relation"] for r in ZEN["related_identifiers"]}
    assert rels["https://freeesta.github.io/mzlab/"] == "isDocumentedBy"
    assert ZEN.get("language") == "eng"
