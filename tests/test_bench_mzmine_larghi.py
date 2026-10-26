"""Form test of the MZmine configurations of the smeared-peak benchmark (parametri/mzmine/mzmine_default.json, mzmine_wide.json): the JSON files parse, the batch XML built from
them parses and carries the resolver values, and the two differ only in the minimum search resolver. MZmine itself is never run here."""
import json
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from bench import corri, corri_mzmine, oro_mzmine410  # noqa: E402

CONFIG = ("mzmine_default", "mzmine_wide")


def _par(nome):
    return corri.parametri("mzmine", nome)


def _risolutore(xml: str) -> dict:
    """Parameters of the minimum search resolver step of a batch -> {name: text}."""
    radice = ET.fromstring(xml)
    passo = next(s for s in radice.iter("batchstep") if s.get("method", "").endswith("MinimumSearchFeatureResolverModule"))
    out = {}
    for p in passo.findall("parameter"):
        out[p.get("name")] = p.text if len(p) == 0 else {c.tag: c.text for c in p}
    return out


def test_default_is_the_documented_workflow():
    base = _par("predefinita")
    d = _par("mzmine_default")
    assert {k: v for k, v in d.items() if k != "nota"} == {k: v for k, v in base.items() if k != "nota"}


def test_wide_changes_only_the_resolver():
    d, w = _par("mzmine_default"), _par("mzmine_wide")
    for chiave in d:
        if chiave not in ("nota", "MinimumSearchResolver"):
            assert d[chiave] == w[chiave], chiave
    cambiati = {k for k in d["MinimumSearchResolver"] if d["MinimumSearchResolver"][k] != w["MinimumSearchResolver"][k]}
    assert cambiati == {"chromatographic_threshold", "search_rt_range", "duration_max"}
    r = w["MinimumSearchResolver"]
    assert r["chromatographic_threshold"] == pytest.approx(0.60)
    assert r["duration_max"] >= 8.0 > d["MinimumSearchResolver"]["duration_max"]


@pytest.mark.parametrize("nome", CONFIG)
def test_batch_xml_parses_and_carries_the_values(nome, tmp_path):
    par = _par(nome)
    f = tmp_path / "a.mzML"
    xml = corri_mzmine.batch_xml([f], tmp_path / "out.csv", par)
    r = _risolutore(xml)
    ris = par["MinimumSearchResolver"]
    assert float(r["Chromatographic threshold"]) == ris["chromatographic_threshold"]
    assert float(r["Minimum search range RT/Mobility (absolute)"]) == ris["search_rt_range"]
    assert float(r["Peak duration range (min/mobility)"]["max"]) == ris["duration_max"]
    assert xml.count("<batchstep ") == 7                      # import, detect, chromatograms, resolver, align, gap filling, export


@pytest.mark.parametrize("nome", CONFIG)
def test_batch_xml_converts_to_410(nome, tmp_path):
    xml = oro_mzmine410.batch_xml([tmp_path / "a.mzML"], tmp_path, _par(nome), "4.10.90")
    ET.fromstring(xml)
    assert "MinimumSearchFeatureResolverModule" in xml


def test_configs_are_valid_json_with_all_sections():
    for nome in CONFIG:
        j = json.loads((ROOT / "tools/bench/parametri/mzmine" / f"{nome}.json").read_text(encoding="utf-8"))
        assert {"MassDetection", "ADAPChromatogramBuilder", "MinimumSearchResolver", "JoinAligner", "GapFilling", "polarita"} <= set(j)
