"""The agreement with mzmine row by ion (`cancello_misure.concordanza_mzmine_ioni`), the reasons of the rows without a partner (`tools/bench/motivi_mzmine.py`) and the gold
data of mzmine 3.9.0 per stage (`tests/data/untargeted/mzmine39/`, `tools/bench/oro_mzmine.py`). No mzmine, no cargo and no private data needed."""
import hashlib
import re
import sys
import zipfile
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(ROOT), str(ROOT / "tools")]
if not (ROOT / "TP_Mine" / "py").exists():
    pytest.skip("TP_Mine/py not available", allow_module_level=True)
from bench import corri_mzmine, motivi_mzmine as MM, oro_mzmine  # noqa: E402
from bench.motivi_mzmine import ListaFile  # noqa: E402
from valida import cancello_misure as M  # noqa: E402
from valida.tabelle import Tabella  # noqa: E402

ORO = ROOT / "tests" / "data" / "untargeted" / "mzmine39"


def tabelle(n: int = 40, k: int = 3):
    """Two identical tables but for the unit of the areas (ours counts x s, mzmine intensity x min) and the RT (+0.01 min)."""
    rng = np.random.default_rng(7)
    mz, rt = np.sort(rng.uniform(150, 800, n)), rng.uniform(1, 20, n)
    area = np.exp(rng.uniform(np.log(1e6), np.log(1e9), (n, k)))
    alt = area / 10.0
    nat = Tabella("nativa", list("xyz"[:k]), mz, rt, area, altezza=alt.copy())
    mm = Tabella("mzmine", list("xyz"[:k]), mz * (1 + 1e-6), rt + 0.01, area / M.MIN_A_S, altezza=alt.copy())
    return nat, mm


def test_converted_units_give_zero_log_ratio_on_equal_areas():
    nat, mm = tabelle()
    c = M.concordanza_mzmine_ioni(nat, mm, [0, 1, 2], 1e3)
    assert c["richiamo"] == 1.0 and c["precisione"] == 1.0 and c["spearman"] > 0.999
    assert c["log2_mediano"] < 1e-9 and c["log2_centrato"] < 1e-9


def test_without_the_conversion_the_ratio_is_the_factor_60():
    nat, mm = tabelle()
    mm.area = mm.area * M.MIN_A_S                      # mzmine areas in counts x s of ours would be 60 times too large after the conversion
    c = M.concordanza_mzmine_ioni(nat, mm, [0, 1, 2], 1e3)
    assert abs(c["log2_mediano"] - np.log2(60)) < 1e-9 and c["log2_centrato"] < 1e-9


def test_a_row_under_the_floor_leaves_both_sides():
    nat, mm = tabelle(n=10, k=1)
    nat.altezza[3, 0] = mm.altezza[3, 0] = 5.0         # under the floor in both
    c = M.concordanza_mzmine_ioni(nat, mm, [0], 100.0)
    assert (c["righe_mzmine"], c["righe_native"], c["abbinate"]) == (9, 9, 9) and c["richiamo"] == 1.0 and c["precisione"] == 1.0
    mm.altezza[5, 0] = 5.0                              # under the floor only in mzmine: the native row has no partner, it is not a precision loss of the other side
    c = M.concordanza_mzmine_ioni(nat, mm, [0], 100.0)
    assert (c["righe_mzmine"], c["righe_native"]) == (8, 9) and c["richiamo"] == 1.0 and c["precisione"] < 1.0


def test_heights_are_required():
    nat, mm = tabelle(n=5, k=1)
    nat.altezza = None
    with pytest.raises(ValueError, match="no heights"):
        M.concordanza_mzmine_ioni(nat, mm, [0], 1.0)


def uno(mz: float, rt: float, alt: float = 1e5, nome: str = "t") -> Tabella:
    return Tabella(nome, ["a"], np.array(mz, float).reshape(-1), np.array(rt, float).reshape(-1), np.full(len(np.atleast_1d(mz)), alt * 10.0).reshape(-1, 1),
                   altezza=np.full(len(np.atleast_1d(mz)), alt).reshape(-1, 1))


def lista(mz, rt, a, b) -> list:
    return [ListaFile(*(np.array(x, float) for x in (mz, rt, a, b)))]


def test_reason_family_a_row_of_mzmine_that_is_a_member_of_one_of_ours():
    nat = uno([300.0], [5.0])
    mm = uno([300.0, 301.0034], [5.0, 5.02], nome="mm")             # the second row of mzmine is the M+1 of the first
    membri = (np.array([0, 2]), np.array([300.0, 301.0034]), np.array([5.0, 5.02]))
    vuota = lista([], [], [], [])
    e = MM.classifica(nat, mm, [0], 1e4, vuota, vuota, None, membri)
    assert MM.conteggi(e)["mzmine"]["family"] == 1 and e["mzmine"]["family"] == [1]
    assert sum(len(v) for v in e["mzmine"].values()) == 1


def test_reason_split_mzmine_cuts_a_peak_that_we_keep_whole():
    nat = uno([400.0], [3.45])
    mm = uno([400.0, 400.0], [3.40, 3.50], nome="mm")                # mzmine closes at a zero: two features, ours one
    nat_file = lista([400.0], [3.45], [3.30], [3.62])
    mm_file = lista([400.0, 400.0], [3.40, 3.50], [3.30, 3.50], [3.40, 3.62])
    e = MM.classifica(nat, mm, [0], 1e4, nat_file, mm_file)
    assert len(e["mzmine"]["split"]) == 1
    assert sum(len(v) for v in e["mzmine"].values()) == 1
    # the other way round: a row of ours with no partner that holds two features of mzmine
    nat2, mm2 = uno([400.0], [3.45]), uno([400.0, 400.0], [3.30, 3.62], nome="mm")
    e2 = MM.classifica(nat2, mm2, [0], 1e4, lista([400.0], [3.45], [3.25], [3.70]), lista([400.0, 400.0], [3.30, 3.62], [3.25, 3.5], [3.5, 3.70]))
    assert len(e2["mzmine"]["split"]) == 2 and e2["nativa"]["split"] == [0]


def test_reason_below_and_background_and_area():
    nat = uno([200.0, 250.0], [1.0, 2.0], 9e3)
    mm = uno([200.0, 300.0], [1.0, 4.0], 1e5, nome="mm")
    nat.altezza[1, 0], mm.altezza[0, 0] = 5e4, 1e5
    mm.area[0, 0] = nat.area[0, 0] / M.MIN_A_S * 8.0               # area 8 times larger than ours
    nat.altezza[0, 0] = 1e5
    adap = lista([300.0], [4.0], [0.5], [9.0])                      # a trace of 8.5 min under the row of mzmine at 300
    e = MM.classifica(nat, mm, [0], 3e4, lista([], [], [], []), lista([], [], [], []), adap, None, fondo_min=5.0)
    assert e["mzmine"]["background"] == [1] and e["area"] == [(0, 0)]
    assert e["nativa"]["other"] == [1]


# ---- gold data of mzmine 3.9.0 ----

def test_the_synthetic_files_are_written_again_byte_for_byte(tmp_path):
    shas = oro_mzmine.genera(tmp_path)
    testo = (ORO / "LEGGIMI.md").read_text(encoding="utf-8")
    assert len(shas) == 4
    for nome, h in shas.items():
        assert f"`{nome}`: `{h}`" in testo, nome
        assert (tmp_path / nome).stat().st_size < 2_000_000


def test_the_gold_csv_have_the_documented_content():
    assert sum(p.stat().st_size for p in ORO.glob("*.csv")) <= 300_000
    righe = {n: [r.split(",") for r in (ORO / f"{n}.csv").read_text(encoding="utf-8").splitlines()] for n in ("adap", "resolved", "aligned", "gapfilled", "isotopes")}
    assert righe["adap"][0] == "file,id,mz,rt,rt_start,rt_end,height".split(",")
    assert righe["resolved"][0] == "file,id,mz,rt,rt_start,rt_end,height,area".split(",")
    assert righe["aligned"][0][:6] == "row,mz,rt,aligned_features,score,file".split(",")
    stati = {r[-1] for r in righe["gapfilled"][1:]}
    assert stati == {"DETECTED", "ESTIMATED"} and {r[-1] for r in righe["aligned"][1:]} == {"DETECTED"}
    ruoli = {r[6] for r in righe["isotopes"][1:]}
    assert ruoli == {"alone", "principal", "isotopologue"}
    cariche = {r[5] for r in righe["isotopes"][1:] if r[6] == "principal"}
    assert cariche == {"1", "2"}                                   # the pairs of charge 1 and 2 are both found


# ---- the batch ----

def test_the_batch_uses_the_names_of_the_parameters_of_mzmine():
    par = corri_mzmine.corri.parametri("mzmine", "predefinita")
    xml = corri_mzmine.batch_xml([Path("a.mzML")], Path("o.csv"), par)
    assert 'name="Min # of data points"' in xml and "Minimum scans (data points)" not in xml.split("MinimumSearch")[1].split("JoinAligner")[0]


def test_unknown_parameter_names_are_reported(tmp_path):
    jar = tmp_path / "lib" / "work-3.9.0.jar"
    jar.parent.mkdir()
    with zipfile.ZipFile(jar, "w") as z:
        z.writestr("io/github/mzmine/modules/dataprocessing/featdet_x/XParameters.class", b"\x00Noise level\x00Other\x00")
        z.writestr("io/github/mzmine/parameters/parametertypes/tolerances/ToleranceType.class", b"sample-to-sample")
    xml = tmp_path / "b.xml"
    xml.write_text('<batch><batchstep method="io.github.mzmine.modules.dataprocessing.featdet_x.XModule"><parameter name="Noise level">1</parameter>'
                   '<parameter name="Noyse level">1</parameter><parameter name="m/z tolerance (sample-to-sample)"/></batchstep></batch>', encoding="utf-8")
    assert corri_mzmine.parametri_non_letti(xml, tmp_path) == ["XModule: Noyse level"]
    assert re.fullmatch(r"[0-9a-f]{64}", corri_mzmine.BIOCONDA_SHA256) and hashlib  # the digest of the package is the one of the bioconda repodata
