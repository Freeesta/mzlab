"""Writes the skeleton of a new Teoria chapter (no course text): head with the theme snippet, noindex, data-esteso, kicker, h1, objectives,
numbered sections, a numbered equation and figure, boxes, end-of-chapter questions, links to glossary and formulary.

    python3 tools/nuovo_capitolo.py 25 spe "Estrazione in fase solida (SPE)" --parte "I bis. Preparare il campione"
    python3 tools/nuovo_capitolo.py 25 spe "Estrazione in fase solida (SPE)" --out ../mzlab-dati/corso/bozze_teoria --source-card "scheda-25"

Writes mzlab/web/teoria/NN-slug.html (or <out>/NN-slug.html: private drafts are born in the private repository) and prints the lines to add by hand
to CHAPTERS in teoria.js (registering the chapter is not done here). Checked afterwards by tools/controlla_teoria.py."""
import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tema_testa  # noqa: E402

TEORIA = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "teoria"

TEMPLATE = """<!doctype html>
<html lang="it" data-esteso><head><meta charset="utf-8">
<!--
chapter: @N@ @TITLE_C@
source-card: @CARD@
verification: draft
exam-level: TODO
Skeleton from tools/nuovo_capitolo.py: no course text here. Write by hand from the private card; verification: verified only after the check.
Do not copy sentences from the sources (tools/controlla_teoria.py rejects runs of 10 identical words).
-->
<meta name="robots" content="noindex">
<title>@TITLE@ · Teoria QqQ lab</title><link rel="icon" href="../favicon-32.png"><link rel="stylesheet" href="teoria.css"><script src="tema.js"></script></head>
<body>
<div class="wrap"><nav id="side"></nav><main>
<div class="kicker">Capitolo @N@</div>
<h1>@TITLE@</h1>
<p class="lead">TODO: due righe di apertura.</p>

<h2>@N@.1 TODO: primo argomento</h2>
<p>TODO.</p>
<span class="eq">TODO<span class="no">(@N@.1)</span></span>

<h2>@N@.2 TODO: secondo argomento</h2>
<figure>
<svg viewBox="0 0 860 230" role="img" aria-label="TODO: nome breve della figura (max 125 caratteri)" aria-describedby="fd-@N@-0">
<rect x="20" y="20" width="820" height="190" rx="8" fill="none" stroke="currentColor" stroke-width="1.4"/>
</svg><details class="figdesc" id="fd-@N@-0"><summary>Descrizione della figura</summary><p>TODO: solo ciò che si vede, al massimo 80 parole.</p></details>
<figcaption><b>Figura @N@.1</b> TODO: didascalia.</figcaption>
</figure>

<div class="box key"><b>Concetto chiave</b><p>TODO.</p></div>
<div class="box warn"><b>Errore frequente</b><p>TODO.</p></div>
<div class="box lab"><b>Esempio</b><p>TODO: esempio senza il composto dell'esperienza.</p></div>

<table>
<caption>TODO: titolo della tabella</caption>
<tr><th scope="col">TODO</th><th scope="col" class="n">TODO</th></tr>
<tr><td>TODO</td><td class="n">TODO</td></tr>
</table>

<p>Termini nel <a href="21-glossario.html">glossario</a>; formule nel <a href="22-formulario.html">formulario</a>.</p>

<h2>Verifica</h2>
<details class="q"><summary>TODO: domanda 1?</summary><p>TODO: risposta.</p></details>
<details class="q"><summary>TODO: domanda 2?</summary><p>TODO: risposta.</p></details>

<h2>Per approfondire</h2>
<ul class="refs">
<li><span class="tag">libro</span>TODO.</li>
</ul>
</main></div>
<script src="../appname.js"></script><script src="teoria.js"></script>
</body></html>
"""


def build(n: int, titolo: str, card: str) -> str:
    esc = titolo.replace("&", "&amp;").replace("<", "&lt;")
    text = (TEMPLATE.replace("@N@", str(n)).replace("@TITLE_C@", titolo.replace("--", "-"))
            .replace("@TITLE@", esc).replace("@CARD@", card))
    return tema_testa.inject(text, "teoria")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("numero", type=int)
    ap.add_argument("slug")
    ap.add_argument("titolo")
    ap.add_argument("--parte", default="TODO: parte")
    ap.add_argument("--out", type=Path, default=TEORIA, help="output folder (default: mzlab/web/teoria)")
    ap.add_argument("--source-card", default="TODO: nome della scheda sorgente privata")
    ap.add_argument("--force", action="store_true", help="overwrite an existing file")
    a = ap.parse_args(argv)
    if not 1 <= a.numero <= 99:
        ap.error("numero: 1-99")
    if not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", a.slug):
        ap.error("slug: lowercase letters, digits and hyphens")
    name = f"{a.numero:02d}-{a.slug}.html"
    dest = a.out / name
    if dest.exists() and not a.force:
        print(f"{dest} exists (use --force to overwrite)", file=sys.stderr)
        return 1
    a.out.mkdir(parents=True, exist_ok=True)
    dest.write_text(build(a.numero, a.titolo, a.source_card), encoding="utf-8")
    print(f"written {dest}")
    print("add to CHAPTERS in teoria.js:")
    print(f'  ["{name}", "{a.numero}", "{a.titolo}", "{a.parte}"],')
    return 0


if __name__ == "__main__":
    sys.exit(main())
