"""Interface texts (English only): the engine mzlab/web/i18n.js (run with node), the catalogs and the rules of tools/controlla_i18n.py."""
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
def test_the_language_is_always_english_and_other_catalogs_or_stored_choices_are_ignored():
    r = node("""I.add('it', {only: 'solo italiano'});
    console.log(JSON.stringify([I.lang, [...I.LANGS], typeof I.set, typeof I.detect, I.t('only'), [...I.missing]]))""")
    assert r == ["en", ["en"], "undefined", "undefined", "only", ["only"]]


@needs_node
def test_a_language_stored_by_an_older_version_is_ignored_silently():
    # mzlab.lang = "it" in localStorage and ?lang=it in the address: the page still starts in English, with no error
    out = subprocess.run(["node", "-e", "globalThis.localStorage = { getItem: k => (k === 'mzlab.lang' ? 'it' : null), setItem() {} };"
                          "globalThis.location = { search: '?lang=it' }; const I = require(" + json.dumps(str(JS)) + ");"
                          "I.add('en', {k: 'Hello'}); console.log(JSON.stringify([I.lang, I.t('k')]))"], check=True, capture_output=True, text=True, encoding="utf-8").stdout
    assert json.loads(out) == ["en", "Hello"]


@needs_node
def test_messages_placeholders_numbers_and_plurals():
    r = node("""I.add('it', {a: 'Ciao {name}', n: '{n, plural, =0 {nessun file} one {# file} other {# file}}', q: '{x, number} m/z'});
    I.add('en', {a: 'Hello {name}', n: '{n, plural, =0 {no files} one {# file} other {# files}}', q: '{x, number} m/z'});
    const f = I.format;
    console.log(JSON.stringify([f('Hello {name}', {name: 'Ada'}, 'en'), f('{n, plural, one {# file} other {# files}}', {n: 1}, 'en'),
      f('{n, plural, one {# file} other {# files}}', {n: 3}, 'en'), f('{n, plural, =0 {no files} one {# file} other {# files}}', {n: 0}, 'en'),
      f('{x, number}', {x: 12345.5}, 'en'), f('{missing}', {}, 'en'),
      f('{n, plural, one {{n} of {name}} other {{n} of {name}}}', {n: 2, name: 'x'}, 'en'),
      I.t('a', {name: 'Ada'}) === 'Hello Ada', I.t('no.such.key'), I.num(3.14159, 2), I.num(5), I.num(null)]))""")
    assert r == ["Hello Ada", "1 file", "3 files", "no files", "12345.5", "{missing}", "2 of x", True, "no.such.key", "3.14", "5", ""]


@needs_node
def test_number_formats_are_built_once_per_decimals():
    # the progress rows of a loading format thousands of numbers: one Intl.NumberFormat per number of decimals, the same text as a fresh one
    r = node("""const N = Intl.NumberFormat; let made = 0; Intl.NumberFormat = function (...a) { made++; return new N(...a); };
    const out = []; for (let i = 0; i < 500; i++) out.push(I.fix(i / 7, 1), I.fix(i * 1000.5, 0), I.format('{x, number}', {x: i / 3}, 'en'));
    const ref = [1, 2].map(d => new N('en-US', {useGrouping: false, minimumFractionDigits: d, maximumFractionDigits: d}).format(1234.5678));
    console.log(JSON.stringify([made, out.slice(0, 3), out.length, I.fix(1234.5678, 1), I.fix(1234.5678, 2), ref]))""")
    assert r[0] == 3 and r[1] == ["0.0", "0", "0"] and r[2] == 1500 and r[3:5] == r[5]

@needs_node
def test_until_the_rest_of_the_catalog_runs_a_key_gives_empty_text_never_the_key():
    # the site's first part names the rest (awaitRest); a key of the rest asked before it gives "" and is noted in `late`; static elements keep
    # their text and are filled by done()
    r = node("""I.add('en', {'p.first': 'first'}); I.awaitRest('lang/en-rest.js'); const err = console.error; const said = []; console.error = m => said.push(m);
      const mk = k => { const a = {'data-i18n': k}; return { textContent: 'page text', isConnected: true, getAttribute: n => (n in a ? a[n] : null) }; };
      const els = [mk('p.first'), mk('p.later')], root = { querySelectorAll: () => els };
      const before = [I.rest, I.t('p.first'), I.t('p.later'), I.t('p.later'), [...I.late], said.length];
      I.apply(root); const held = els.map(e => e.textContent);
      I.add('en', {'p.later': 'later'}); I.done(); console.error = err;
      console.log(JSON.stringify([before, held, els.map(e => e.textContent), I.rest, I.t('p.later')]))""")
    assert r == [["lang/en-rest.js", "first", "", "", ["p.later"], 1], ["first", "page text"], ["first", "later"], "", "later"]


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
def test_numbers_always_use_the_point():
    r = node("""I.add('en', {k: '{x, number} · {n, plural, other {# file}}'});
    console.log(JSON.stringify([I.fix(1234.5, 2), I.fix(0.5, 1), I.t('k', {x: 1234567.25, n: 12345.5})]))""")
    assert r == ["1234.50", "0.5", "1234567.25 · 12345.5 file"]


@needs_node
def test_app_name_fills_in_without_parameters():
    """{app} comes from window.APP_NAME (appname.js) even when the caller passes no parameters; an explicit parameter still wins."""
    r = node("globalThis.APP_NAME='NameTest'; I.add('en', {'k.a': '{app} is free', 'k.b': '{app} and {n}'});"
             "console.log(JSON.stringify([I.t('k.a'), I.t('k.a', {}), I.t('k.b', {n: 2}), I.t('k.a', {app: 'Other'})]))")
    assert r == ["NameTest is free", "NameTest is free", "NameTest and 2", "Other is free"]
