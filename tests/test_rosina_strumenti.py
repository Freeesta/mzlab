"""The tools of the Rosina bench (tools/rosina/, docs/studi/ROSINA.md F): the truth made from an mzML is valid and stable, TRFP and timsdata inputs are folded in, the downloader
resumes and refuses, the index checker rejects what the vocabulary does not allow, and the metadata map covers every row of the data sheets."""
import csv
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
STR = ROOT / "tools" / "rosina"
FIX = ROOT / "crates/mzlab-lettori/tests/rosina"
sys.path.append(str(STR))
import importlib.util


def _carica(nome: str):
    """A tool of tools/rosina/ by path: tools/indice.py and tools/bench/verita.py have the same names."""
    spec = importlib.util.spec_from_file_location("rosina_" + nome, STR / (nome + ".py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


indice = _carica("indice")
scarica = _carica("scarica")
verita = _carica("verita")
verita_timsdata = _carica("verita_timsdata")
verita_trfp = _carica("verita_trfp")


def _senza_variabili(d: dict) -> dict:
    d = json.loads(json.dumps(d))
    d["oracolo"].pop("data", None)
    d["oracolo"].pop("piattaforma", None)
    d["file"].pop("sha256", None)
    return d


def test_la_verita_del_sintetico_e_valida_e_uguale_a_quella_nel_repository():
    d = verita.da_mzml(FIX / "sintetico.mzML", ("generatore sintetico", "1", "tools/mzml_msconvert_sintetico.py"), FIX / "sintetico.mzML", None)
    assert verita.valida(d) == []
    assert _senza_variabili(d) == _senza_variabili(json.loads((FIX / "sintetico.verita.json").read_text()))
    assert d["conteggi"]["spettri"] == {"ms1": 1, "ms2": 3}
    assert len(d["punti"]) <= 8 and all(0 <= int(k) < len(d["scansioni"]) for k in d["punti"])


def test_il_validatore_rifiuta_una_verita_rotta():
    d = json.loads((FIX / "sintetico.verita.json").read_text())
    d["scansioni"][0]["polarita"] = 2
    del d["conteggi"]
    d["schema"] = "verita/2"
    d["cromatogrammi"][0]["campione"] = [[0.0, 1.0]] * 17
    problemi = "\n".join(verita.valida(d))
    for atteso in ("missing conteggi", "polarita", "verita/1", "more than 16"):
        assert atteso in problemi, problemi


def test_da_riga_di_comando_la_verita_e_json_valido(tmp_path):
    out = subprocess.run([sys.executable, str(STR / "verita.py"), str(FIX / "sintetico.mzML"), "--oracolo", "msconvert", "3.0.24", "-", "--campione", "2"], capture_output=True, text=True, check=True)
    d = json.loads(out.stdout)
    assert d["oracolo"]["nome"] == "msconvert" and d["file"]["campione"] == 2 and out.stderr == ""


def test_un_array_numpress_o_senza_larghezza_e_un_errore_chiaro(tmp_path):
    p = tmp_path / "x.mzML"
    p.write_text(
        '<mzML xmlns="http://psi.hupo.org/ms/mzml"><run><spectrumList><spectrum id="s"><binaryDataArrayList><binaryDataArray>'
        '<cvParam cvRef="MS" accession="MS:1002312" name="numpress"/><cvParam cvRef="MS" accession="MS:1000514" name="m/z array"/><binary>AAAA</binary>'
        "</binaryDataArray></binaryDataArrayList></spectrum></spectrumList></run></mzML>"
    )
    with pytest.raises(ValueError, match="numpress"):
        verita.da_mzml(p, ("x", "1", ""), None, None)


def test_trfp_appiattisce_il_json_senza_conoscerne_la_forma(tmp_path):
    meta = tmp_path / "m.json"
    meta.write_text(json.dumps({"FileProperties": [{"Name": "Instrument model", "Value": "Q Exactive"}], "Sample": {"Vial": "A1", "Inj": 5}, "Lista": [{"x": 1}]}))
    d = verita_trfp.da_trfp(FIX / "sintetico.mzML", meta, "1.4.5", None)
    assert d["metadati"]["trfp:FileProperties.Instrument model"]["valore"] == "Q Exactive"
    assert d["metadati"]["trfp:Sample.Vial"]["valore"] == "A1"
    assert d["metadati"]["trfp:Lista.0.x"]["valore"] == 1
    assert d["oracolo"]["nome"] == "ThermoRawFileParser" and verita.valida(d) == []


def test_timsdata_aggiunge_la_tabella_di_mobilita(tmp_path):
    t = tmp_path / "k0.tsv"
    t.write_text("scan\tone_over_k0\n0\t1.45\n1\t1.449\n")
    assert verita_timsdata.leggi_tabella(t) == {"scan": [0, 1], "one_over_k0": [1.45, 1.449]}
    t.write_text("frame\tk0\n0\t1.4\n")
    with pytest.raises(SystemExit):
        verita_timsdata.leggi_tabella(t)
    d = verita.da_mzml(FIX / "sintetico.mzML", ("msconvert+timsdata", "1", ""), None, None)
    d["mobilita"] = {"scan": [0], "one_over_k0": [1.4]}
    assert verita.valida(d) == []


CORPO = bytes(range(256)) * 400


def _grandi(tmp_path, righe):
    p = tmp_path / "GRANDI.tsv"
    with p.open("w", newline="") as f:
        w = csv.writer(f, delimiter="\t")
        w.writerow(["percorso", "url", "destinazione", "byte", "sha256"])
        w.writerows(righe)
    return p


def test_scarica_riprende_un_file_parziale_e_controlla_il_checksum(tmp_path, monkeypatch):
    corpo = CORPO
    monkeypatch.setattr(scarica.urllib.request, "urlopen", lambda req, timeout=0: _http(req, corpo))
    dest = tmp_path / "dest"
    (dest / "a").mkdir(parents=True)
    (dest / "a/grande.bin.parte").write_bytes(corpo[:1000])
    tsv = _grandi(tmp_path, [["a", "https://x.test/grande.bin", "a/grande.bin", len(corpo), hashlib.sha256(corpo).hexdigest()]])
    assert scarica.main([str(tsv), str(dest)]) == 0
    assert (dest / "a/grande.bin").read_bytes() == corpo
    assert not (dest / "a/grande.bin.parte").exists()
    # a wrong checksum is an error, a file already complete is not downloaded again
    tsv = _grandi(tmp_path, [["a", "https://x.test/grande.bin", "a/grande.bin", len(corpo), "0" * 64]])
    assert scarica.main([str(tsv), str(dest)]) == 1


class _Risposta:
    def __init__(self, dati, status):
        self.dati, self.status, self.pos = dati, status, 0

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False

    def read(self, n):
        b = self.dati[self.pos : self.pos + n]
        self.pos += n
        return b


def _http(req, corpo):
    r = req.headers.get("Range")
    if r:
        return _Risposta(corpo[int(re.match(r"bytes=(\d+)-", r).group(1)) :], 206)
    return _Risposta(corpo, 200)


def test_scarica_rifiuta_il_tetto_i_percorsi_che_escono_e_gli_url_non_https(tmp_path, capsys):
    tsv = _grandi(tmp_path, [["a", "https://x.test/a", "a.bin", 30_000_000_000, ""]])
    assert scarica.main([str(tsv), str(tmp_path / "d"), "--tetto-gb", "20"]) == 2
    assert "ceiling" in capsys.readouterr().err
    tsv = _grandi(tmp_path, [["a", "https://x.test/a", "../fuori.bin", 10, ""], ["b", "http://x.test/b", "b.bin", 10, ""]])
    assert scarica.main([str(tsv), str(tmp_path / "d"), "--prova"]) == 1
    err = capsys.readouterr().err
    assert "outside DEST" in err and "not https" in err
    assert not (tmp_path / "fuori.bin").exists()


RIGA = {
    "percorso": "thermo/raw/PXD1_x/a.raw", "produttore": "thermo", "modello_nel_file": "Q Exactive", "versione_formato": "66", "esperimenti": "fullscan,dda",
    "fonte": "PRIDE", "accession": "PXD1", "doi": "", "licenza": "CC0", "ridistribuibile": "si", "byte": "123", "sha256": "", "url": "https://x", "riferimento": "trfp-1.4.5",
    "verita": "si", "note": "",
}


def _indice(tmp_path, *righe):
    p = tmp_path / "INDICE.tsv"
    with p.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=indice.COLONNE, delimiter="\t")
        w.writeheader()
        w.writerows(righe)
    return p


def test_indice_accetta_una_riga_giusta_e_scrive_il_markdown(tmp_path):
    p = _indice(tmp_path, RIGA)
    md = tmp_path / "INDICE.md"
    assert indice.main([str(p), "--md", str(md)]) == 0
    assert "thermo/raw/PXD1_x/a.raw" in md.read_text() and "## thermo" in md.read_text()


@pytest.mark.parametrize(
    "campo,valore,messaggio",
    [
        ("produttore", "ibm", "produttore"),
        ("esperimenti", "fullscan,xyz", "vocabulary"),
        ("ridistribuibile", "forse", "ridistribuibile"),
        ("licenza", "", "without a licence"),
        ("verita", "boh", "verita"),
        ("byte", "12MB", "byte"),
        ("sha256", "abc", "sha256"),
        ("percorso", "", "empty percorso"),
    ],
)
def test_indice_rifiuta_cio_che_il_vocabolario_non_ammette(tmp_path, capsys, campo, valore, messaggio):
    p = _indice(tmp_path, {**RIGA, campo: valore})
    assert indice.main([str(p), "--controlla"]) == 1
    assert messaggio in capsys.readouterr().err


def test_indice_rifiuta_percorsi_ripetuti_e_colonne_mancanti(tmp_path, capsys):
    assert indice.main([str(_indice(tmp_path, RIGA, RIGA)), "--controlla"]) == 1
    assert "repeated" in capsys.readouterr().err
    p = tmp_path / "corto.tsv"
    p.write_text("percorso\tproduttore\nx\tthermo\n")
    assert indice.main([str(p), "--controlla"]) == 1
    assert "missing column" in capsys.readouterr().err


def test_la_mappa_dei_metadati_copre_ogni_riga_delle_schede():
    with (STR / "mappa_metadati.tsv").open(newline="", encoding="utf-8") as f:
        mappa = list(csv.DictReader(f, delimiter="\t"))
    attese = {}
    for scheda in sorted((ROOT / "docs/studi/rosina").glob("SCHEDA_*.md")):
        nome = scheda.stem.removeprefix("SCHEDA_").lower()
        dentro = False
        for riga in scheda.read_text(encoding="utf-8").splitlines():
            if riga.startswith("| Campo del produttore"):
                dentro = True
            elif dentro and riga.startswith("|---"):
                continue
            elif dentro and riga.startswith("|"):
                c = [x.strip() for x in riga.strip().strip("|").split("|")]
                attese[(nome, c[0])] = re.sub(r"\s+", " ", c[2].replace("`", "")) or "-"
            else:
                dentro = False
    trovate = {(r["scheda"], r["campo_produttore"]): r["metodo"] for r in mappa}
    assert set(trovate) == set(attese), (set(attese) ^ set(trovate))
    assert trovate == attese, "metodo/1 column differs from the data sheet: update mappa_metadati.tsv"
    assert {r["normalizzazione"] for r in mappa} <= {"testo", "iso8601", "numero", "nessuna"}
    assert {r["stato"] for r in mappa} <= {"L", "C", "N", "?"}
