"""Interface languages: the engine mzlab/web/i18n.js (run with node), the catalogs and the rules of tools/controlla_i18n.py."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
JS = ROOT / "mzlab" / "web" / "i18n.js"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def node(code: str):
    out = subprocess.run(["node", "-e", f"const I=require({str(JS)!r});" + code], check=True, capture_output=True, text=True, encoding="utf-8").stdout
    return json.loads(out)


@needs_node
def test_default_language_follows_the_browser():
    r = node("console.log(JSON.stringify([I.detect('', null, 'it-IT'), I.detect('', null, 'IT'), I.detect('', null, 'en-US'), I.detect('', null, 'fr-FR'),"
             "I.detect('', null, 'de'), I.detect('', null, undefined), I.detect('', null, 'es-ES')]))")
    assert r == ["it", "it", "en", "en", "en", "en", "en"]


@needs_node
def test_explicit_choice_wins_over_the_browser_and_the_address_over_both():
    r = node("console.log(JSON.stringify([I.detect('', 'it', 'en-US'), I.detect('', 'en', 'it-IT'), I.detect('?lang=en', 'it', 'it-IT'),"
             "I.detect('?x=1&lang=it', 'en', 'en-US'), I.detect('?lang=xx', null, 'it-IT'), I.detect('', 'garbage', 'it-IT')]))")
    assert r == ["it", "en", "en", "it", "it", "it"]


@needs_node
def test_messages_placeholders_numbers_and_plurals():
    r = node("""I.add('it', {a: 'Ciao {name}', n: '{n, plural, =0 {nessun file} one {# file} other {# file}}', q: '{x, number} m/z'});
    I.add('en', {a: 'Hello {name}', n: '{n, plural, =0 {no files} one {# file} other {# files}}', q: '{x, number} m/z'});
    const f = I.format;
    console.log(JSON.stringify([f('Hello {name}', {name: 'Ada'}, 'en'), f('{n, plural, one {# file} other {# files}}', {n: 1}, 'en'),
      f('{n, plural, one {# file} other {# files}}', {n: 3}, 'en'), f('{n, plural, =0 {no files} one {# file} other {# files}}', {n: 0}, 'en'),
      f('{x, number}', {x: 12345.5}, 'en'), f('{x, number}', {x: 12345.5}, 'it'), f('{missing}', {}, 'en'),
      f('{n, plural, one {{n} of {name}} other {{n} of {name}}}', {n: 2, name: 'x'}, 'en'),
      I.t('a', {name: 'Ada'}) === (I.lang === 'it' ? 'Ciao Ada' : 'Hello Ada'), I.t('no.such.key'), I.num(3.14159, 2), I.num(5), I.num(null)]))""")
    assert r == ["Hello Ada", "1 file", "3 files", "no files", "12345.5", "12345.5", "{missing}", "2 of x", True, "no.such.key", "3.14", "5", ""]


@needs_node
def test_a_key_missing_in_english_falls_back_to_italian():
    r = node("""I.add('it', {only: 'solo italiano'}); console.log(JSON.stringify([I.t('only'), [...I.missing]]))""")
    assert r == ["solo italiano", []]


def test_the_checker_passes_on_the_repository():
    p = subprocess.run([sys.executable, str(ROOT / "tools" / "controlla_i18n.py")], capture_output=True, text=True, encoding="utf-8")
    assert p.returncode == 0, p.stdout + p.stderr


def test_the_checker_catches_what_it_promises():
    sys.path.insert(0, str(ROOT / "tools"))
    import controlla_i18n as c
    assert c.icu_signature("{n, plural, one {# file} other {# files}} da {name}")[0] == [("n", "plural"), ("name", "plain")]
    assert c.icu_signature("{n, plural, one {a} other {b}}")[1] == ["one", "other"]
    with pytest.raises(ValueError):
        c.icu_signature("{n")
    assert c.italian_in("Carica i file") and not c.italian_in("Load the files")
    assert c.has_form("The Precursor ion", ["precursor ion"]) and c.has_form("precursors", ["precursor"]) and not c.has_form("precursory", ["precursor"])
    strings = list(c.js_strings('const a = "x"; // "no"\n/* "no" */ const b = `y`;'))
    assert [s for _, s in strings] == ["x", "y"]


def test_user_error_to_json():
    sys.path.insert(0, str(ROOT))
    from mzlab.i18n import UserError
    e = UserError("err.file.notMzml", {"name": "a.raw"}, "not an mzML file")
    assert e.to_json() == {"error": "not an mzML file", "error_key": "err.file.notMzml", "params": {"name": "a.raw"}}


def test_api_answers_with_keys():
    """A UserError reaches the page as error_key + params; the old HR_MIX marker and the Italian error texts are gone."""
    sys.path.insert(0, str(ROOT))
    import json
    from mzlab.api import dispatch
    from mzlab.app import App
    app = App(None)
    code, _, body, _ = dispatch(app, "GET", "/api/chrom", {"k": "0"})
    assert code == 500 or code == 400
    j = json.loads(body)
    assert j["error_key"] == "err.session.noSuchFile" and "params" in j


@needs_node
def test_numbers_always_use_the_point_in_both_languages():
    r = node("""I.add('it', {k: '{x, number} · {n, plural, other {# file}}'}); I.set && I.set('it');
    console.log(JSON.stringify([I.fix(1234.5, 2), I.fix(0.5, 1), I.t('k', {x: 1234567.25, n: 12345.5})]))""")
    assert r == ["1234.50", "0.5", "1234567.25 · 12345.5 file"]
