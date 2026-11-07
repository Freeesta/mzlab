"""tools/controlla_inglese.py: Italian is found in names, comments and paths, English words that look Italian are not."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import controlla_inglese as ci  # noqa: E402


def test_identifiers_split_and_match():
    assert ci.subwords("tabPicchiRiga") == ["tab", "picchi", "riga"]
    assert ci.italian_words(ci.subwords("chiudi_pannello")) == ["chiudi", "pannello"]
    assert ci.italian_words(ci.subwords("integrateController")) == []
    assert ci.italian_words(ci.subwords("from_le_bytes")) == []           # `le` alone is little-endian, not Italian
    assert ci.italian_words(ci.subwords("soloMode")) == []                # ambiguous word without a clear one


def test_comments():
    assert ci.italian_comment(" sposta (trascina l'intestazione), porta davanti, ridimensiona")
    assert ci.italian_comment(" questo è il fondo")
    assert not ci.italian_comment(" the tour of Dati lives in Dati; in Disegno the button starts the tour of Disegno")
    assert not ci.italian_comment(" «Da dove viene questo ione?» is read on its first use")
    assert not ci.italian_comment(" mappa.js: the 3D viewer (mappa3d.js), a modal over the page")


def test_lexers_keep_comments_strings_and_lines_apart():
    src = 'const a = /\'x/; // chiudi il pannello\nlet campoNome = "storia.fondo";\n'
    com, ids, strs = ci.lex_c_like(src, ".js")
    assert com == [(1, " chiudi il pannello")]
    assert (2, "campoNome") in ids and (2, "storia.fondo") in strs
    r = ci.scan_file("x.js", src)
    assert r["comments"] == [1] and "campoNome" in r["idents"] and "storia.fondo" in r["keys"]
    com, _, strs = ci.lex_python('"""Modulo di prova."""\nx = """non un docstring"""\n')
    assert [t for _, t in com] == ["Modulo di prova."] and strs[0][1] == "non un docstring"


def test_lang_files_read_keys_only():
    _, _, keys = ci.lex_lang('I18N.add("it", {\n  "storia.fondo": "Fondo tolto",\n  "app.title": "Analisi"\n});\n')
    assert [k for _, k in keys] == ["storia.fondo", "app.title"]


def test_paths_and_rename_map():
    assert ci.name_hits("tests_e2e/e2e_apri_uno.py") == ["apri", "uno"]
    assert ci.translate_component("lettore_finto")[0] == "fake_reader"
    assert ci.translate_component("MSBNK-UniTO-1.txt")[0] == "MSBNK-UniTO-1.txt"
    rows = ci.rename_map(["a/dati/x.py", "a/data/y.py", "b/progresso.js", "b/avanzamento2.js"])
    assert ("dir", "a/dati", "a/data", "MERGE") in rows
    assert ("file", "b/progresso.js", "b/progress.js", "") in rows
