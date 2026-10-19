import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import controlla_teoria as ct  # noqa: E402
import nuovo_capitolo  # noqa: E402

LONG = "uno due tre quattro cinque sei sette otto nove dieci undici dodici"


def good(n=25, verified=False):
    """A synthetic valid extended chapter: the skeleton with its placeholders replaced."""
    t = nuovo_capitolo.build(n, "Capitolo di prova", "scheda-x").replace("TODO: ", "").replace("TODO", "testo")
    return t.replace("exam-level: testo", "exam-level: base").replace("verification: draft", "verification: verified" if verified else "verification: draft")


def errs(tmp_path, text, name="25-prova.html", monkeypatch=None):
    (tmp_path / name).write_text(text, encoding="utf-8")
    return [f"{c}: {m}" for c, m in ct.public_errors(tmp_path / name, tmp_path, text)]


def has(e, frag):
    return any(frag in x for x in e)


def test_valid_chapter_passes(tmp_path):
    assert errs(tmp_path, good()) == []
    assert errs(tmp_path, good(verified=True)) == []


def test_structure(tmp_path):
    g = good()
    assert has(errs(tmp_path, g.replace("<h1>", "<h1>x</h1><h1>")), "one h1")
    assert has(errs(tmp_path, g.replace("<h2>25.1", "<h4>25.1").replace("TODO: primo argomento</h2>", "</h4>")), "jumps")
    assert has(errs(tmp_path, g.replace('class="q"', 'class="xx"')), "questions")
    assert has(errs(tmp_path, g.replace("(25.1)", "(24.1)")), "does not belong")
    assert has(errs(tmp_path, g.replace("(25.1)", "(25.2)")), "25.1 is missing")
    assert has(errs(tmp_path, g.replace("Figura 25.1", "Figura 25.3")), "figure 25.1 is missing")
    assert has(errs(tmp_path, g.replace("</figure>", "<figcaption><b>Figura 25.1</b> x</figcaption></figure>")), "appears 2 times")
    assert has(errs(tmp_path, g.replace('lang="it" ', "")), "lang")
    assert has(errs(tmp_path, g.replace("<title>Capitolo di prova · Teoria QqQ lab</title>", "<title></title>")), "title")
    assert has(errs(tmp_path, g.replace('href="21-glossario.html"', 'href="nonesiste.html"')), "missing file")
    assert has(errs(tmp_path, g + '<a href="#manca">x</a>'), "#manca")
    assert not has(errs(tmp_path, g + '<a href="#fd-25-0">x</a>'), "#fd")


def test_equation_range_is_accepted(tmp_path):
    g = good().replace("(25.1)</span></span>", "(25.1)</span></span><span class=\"eq\">y<span class=\"no\">(25.2–25.3)</span></span>")
    assert errs(tmp_path, g) == []


def test_accessibility(tmp_path):
    g = good()
    assert has(errs(tmp_path, g.replace('role="img" ', "")), "role=img")
    assert has(errs(tmp_path, g.replace("nome breve della figura (max 125 caratteri)", "x" * 130)), "125")
    assert has(errs(tmp_path, g.replace("solo ciò che si vede, al massimo 80 parole.", " ".join(["parola"] * 81))), "words")
    assert has(errs(tmp_path, g.replace(' aria-describedby="fd-25-0"', "")), "aria-describedby")
    assert has(errs(tmp_path, g + "<canvas></canvas>"), "canvas")
    assert not has(errs(tmp_path, g + '<div class="sim"><canvas></canvas></div>'), "canvas")
    assert has(errs(tmp_path, g + '<img src="x.png">'), "alt")
    assert has(errs(tmp_path, g.replace("<caption>titolo della tabella</caption>", "")), "caption")
    assert has(errs(tmp_path, g.replace("<th ", "<td ").replace("</th>", "</td>")), "header cells")
    assert has(errs(tmp_path, g.replace("</head>", "<style>.a{background:#fff}</style></head>")), "white background")
    assert has(errs(tmp_path, g + '<p style="background-color: white">x</p>'), "white background")
    assert not has(errs(tmp_path, g.replace("</head>", "<style>.a{fill:#fff}</style></head>")), "white")


def test_markers_only_in_verified_or_published(tmp_path):
    todo = good().replace("<p>testo.</p>", "<p>TODO</p>", 1)
    assert errs(tmp_path, todo) == []                                        # draft + extended: allowed
    assert has(errs(tmp_path, todo.replace("verification: draft", "verification: verified")), "marker 'TODO'")
    assert has(errs(tmp_path, good(verified=True).replace("<p>testo.</p>", "<p>da verificare ???</p>", 1)), "da verificare")
    published = todo.replace(" data-esteso", "")
    assert has(errs(tmp_path, published), "marker 'TODO'")


def test_extended_needs_noindex_and_header(tmp_path):
    g = good()
    assert has(errs(tmp_path, g.replace('<meta name="robots" content="noindex">', "")), "noindex")
    assert has(errs(tmp_path, g.replace("verification: draft", "verification: boh")), "verification")
    assert not has(errs(tmp_path, g.replace(" data-esteso", "").replace('<meta name="robots" content="noindex">', "")), "noindex")


def test_redirect_pages_are_skipped(tmp_path):
    assert errs(tmp_path, '<html><head><meta http-equiv="refresh" content="0;url=a.html"></head></html>', "03-lc.html") == []


def test_forbidden_names(tmp_path):
    terms = ["acido ossalico", "prof. rossi"]
    g = good()
    assert ct.private_errors(g, terms, None) == []
    e = ct.private_errors(g.replace("<p>testo.</p>", "<p>Si usa l'Acido  Ossalico.</p>", 1), terms, None)
    assert e and e[0][0] == "C"
    assert ct.private_errors(g.replace("<p>testo.</p>", "<p>acido ossalicoide</p>", 1), terms, None) == []


def test_copied_runs(tmp_path):
    src = "Questa è una frase della sbobina, copiata: " + LONG + " e poi altro."
    grams = {hash(tuple(w)) for w in [ct.words(src)[i:i + 10] for i in range(len(ct.words(src)) - 9)]}
    e = ct.private_errors(good().replace("<p>testo.</p>", f"<p>{LONG.upper()}.</p>", 1), None, grams)
    assert e and e[0][0] == "D"
    assert ct.private_errors(good().replace("<p>testo.</p>", "<p>uno due tre quattro cinque sei sette otto nove.</p>", 1), None, grams) == []


def test_private_checks_read_the_data_repository(tmp_path, monkeypatch):
    d = tmp_path / "dati"
    (d / "Sbobine_elaborate" / "testo").mkdir(parents=True)
    (d / "nomi_vietati.txt").write_text("# comment\nacido ossalico\n", encoding="utf-8")
    (d / "Sbobine_elaborate" / "testo" / "a.txt").write_text("x " + LONG + " y", encoding="utf-8")
    monkeypatch.setenv("MZLAB_DATI", str(d))
    pages = tmp_path / "p"
    pages.mkdir()
    (pages / "25-a.html").write_text(good().replace("<p>testo.</p>", "<p>acido ossalico</p>", 1), encoding="utf-8")
    (pages / "26-b.html").write_text(good(26).replace("<p>testo.</p>", f"<p>{LONG}</p>", 1), encoding="utf-8")
    (pages / "27-c.html").write_text(good(27), encoding="utf-8")
    new, _, warns = ct.run(pages, {})
    assert warns == [] and set(new) == {"25-a.html", "26-b.html"}
    assert new["25-a.html"][0].startswith("C:") and new["26-b.html"][0].startswith("D:")


def test_without_private_data_it_warns_and_passes(tmp_path, monkeypatch):
    monkeypatch.delenv("MZLAB_DATI", raising=False)
    (tmp_path / "25-a.html").write_text(good(), encoding="utf-8")
    new, _, warns = ct.run(tmp_path, {})
    assert new == {} and warns and "skipped" in warns[0]
    monkeypatch.setenv("MZLAB_DATI", str(tmp_path / "vuota"))                 # folder that does not exist: same
    assert ct.run(tmp_path, {})[2]
    d = tmp_path / "d"
    (d / "scratchpad").mkdir(parents=True)                                    # recognised as the data repository by its marker
    monkeypatch.setenv("MZLAB_DATI", str(d))                                  # repository without the lists: warns for both
    assert len(ct.run(tmp_path, {})[2]) == 2


def test_base_only_blocks_worsening(tmp_path):
    (tmp_path / "25-a.html").write_text(good().replace('class="q"', 'class="xx"'), encoding="utf-8")
    bf = tmp_path / "base.json"
    assert ct.main(["--path", str(tmp_path), "--base-file", str(bf), "--base"]) == 0
    assert ct.main(["--path", str(tmp_path), "--base-file", str(bf)]) == 0     # known error: tolerated
    (tmp_path / "25-a.html").write_text(good().replace('class="q"', 'class="xx"').replace('lang="it" ', ''), encoding="utf-8")
    assert ct.main(["--path", str(tmp_path), "--base-file", str(bf)]) == 1     # one more: fails
    assert ct.main(["--path", str(tmp_path), "--base-file", str(bf), "--senza-base"]) == 1
    assert "A: missing end-of-chapter questions (details.q)" in json.loads(bf.read_text(encoding="utf-8"))["25-a.html"]


def test_current_pages_pass_with_the_base(monkeypatch):
    monkeypatch.delenv("MZLAB_DATI", raising=False)                            # the public test never needs private data
    assert ct.main([]) == 0
