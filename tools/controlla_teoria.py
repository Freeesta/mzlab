"""Checker for the Teoria chapters (mzlab/web/teoria/*.html or --path FILE|FOLDER). Exit 1 if a page has an error NOT listed in tools/teoria_base.json.

    python3 tools/controlla_teoria.py                  # all pages against the base
    python3 tools/controlla_teoria.py --path bozze/    # private drafts (needs no data repository)
    python3 tools/controlla_teoria.py --base           # rewrite the base with the errors of the EXISTING pages (public checks only)

Checks: (a) structure, (b) accessibility, (e) leftover markers, (f) noindex on extended chapters: always, no private data needed.
With the private data repository (tools/dati_percorsi.py, or MZLAB_DATI=<path>) also (c) forbidden names (nomi_vietati.txt, one per line) and (d) runs of >= 10 consecutive words
copied from Sbobine_elaborate/testo/*.txt; without it these two are skipped with a warning. Errors of (c) and (d) never go in the base."""
import argparse
import json
import re
import sys
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_percorsi import radice  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
TEORIA = ROOT / "mzlab" / "web" / "teoria"
BASE = Path(__file__).resolve().parent / "teoria_base.json"
RUN = 10            # consecutive identical words that count as copying
LABEL_MAX, DESC_MAX = 125, 80
PRIVATE = ("C", "D")
VOID = {"meta", "link", "img", "br", "hr", "input", "source", "col", "area", "base", "wbr", "path", "rect", "circle", "line", "ellipse", "polyline", "polygon", "stop", "use"}


class Page(HTMLParser):
    """Flat record of a page: elements with their attributes, text per element of interest, comments."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.html_attrs, self.heads, self.ids, self.hrefs, self.comments = {}, [], set(), [], []
        self.h1 = 0
        self.metas, self.title, self.imgs, self.svgs, self.canvases = [], "", [], [], []
        self.figures, self.tables, self.styles, self.text, self.nos, self.caps, self.figdescs = [], [], [], [], [], [], []
        self.classes = Counter()
        self.stack = []

    def handle_comment(self, data):
        self.comments.append(data)

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "html":
            self.html_attrs = a
        if a.get("id"):
            self.ids.add(a["id"])
        for c in (a.get("class") or "").split():
            self.classes[c] += 1
        if a.get("href"):
            self.hrefs.append(a["href"])
        if tag in ("h1", "h2", "h3", "h4", "h5", "h6"):
            self.heads.append([int(tag[1]), ""])
            self.h1 += tag == "h1"
        elif tag == "meta":
            self.metas.append(a)
        elif tag == "img":
            self.imgs.append(a)
        elif tag == "canvas":
            self.canvases.append((a, self._in_sim()))
        elif tag == "svg" and not any(s[0] == "svg" for s in self.stack):
            self.svgs.append((a, self._in_sim()))
        elif tag == "figure":
            self.figures.append({"attrs": a, "sim": "sim" in (a.get("class") or "") or self._in_sim(), "svg": False, "canvas": False, "img": False, "caption": ""})
        elif tag == "table":
            self.tables.append({"th": 0, "caption": False})
        elif tag == "th" and self.tables:
            self.tables[-1]["th"] += 1
        elif tag == "caption" and self.tables:
            self.tables[-1]["caption"] = True
        if tag in ("svg", "canvas", "img") and self.figures and any(s[0] == "figure" for s in self.stack):
            self.figures[-1][tag] = True
        if a.get("style") is not None:
            self.styles.append(a["style"])
        if tag not in VOID:
            self.stack.append((tag, a))

    def _in_sim(self):
        return any("sim" in (s[1].get("class") or "").split() for s in self.stack)

    def handle_endtag(self, tag):
        for i in range(len(self.stack) - 1, -1, -1):
            if self.stack[i][0] == tag:
                del self.stack[i:]
                break

    def handle_data(self, data):
        tags = [s[0] for s in self.stack]
        if "style" in tags:
            self.styles.append(data)
            return
        if "script" in tags:
            return
        self.text.append(data)
        if "title" in tags:
            self.title += data
        if self.heads and tags and tags[-1] in ("h1", "h2", "h3", "h4", "h5", "h6") or (self.heads and any(t[0] == "h" and len(t) == 2 for t in tags)):
            self.heads[-1][1] += data
        cls = [(s[1].get("class") or "").split() for s in self.stack]
        if any("no" in c for c in cls) and tags and "span" in tags:
            self.nos.append(data.strip())
        if "figcaption" in tags:
            self.caps.append(data)
        if any("figdesc" in c for c in cls) and "p" in tags:
            self.figdescs.append(data)


def parse(text):
    p = Page()
    p.feed(text)
    return p


def words(s):
    return re.sub(r"[\W_]+", " ", s.lower()).split()


def chapter_number(name):
    m = re.match(r"(\d+)-", name)
    return int(m.group(1)) if m else None


def meta_info(page):
    """Fields of the header comment (source-card, verification, exam-level)."""
    out = {}
    for c in page.comments:
        for k in ("source-card", "verification", "exam-level"):
            m = re.search(rf"^\s*{k}:\s*(.*)$", c, re.M)
            if m:
                out[k] = m.group(1).strip()
    return out


def public_errors(path, folder, text):
    """Errors of checks (a), (b), (e), (f) as (code, message) tuples; messages carry no line numbers (stable for the base)."""
    E = []
    p = parse(text)
    if any(m.get("http-equiv", "").lower() == "refresh" for m in p.metas):
        return E                                               # old chapter name: redirect only
    esteso = "data-esteso" in p.html_attrs
    info = meta_info(p)
    # (a) structure
    if not p.html_attrs.get("lang"):
        E.append(("A", "html without lang"))
    if not p.title.strip():
        E.append(("A", "empty or missing title"))
    if p.h1 != 1:
        E.append(("A", f"expected one h1, found {p.h1}"))
    prev = 0
    for lv, t in p.heads:
        if lv > prev + 1:
            E.append(("A", f"heading level jumps h{prev} -> h{lv}: {t.strip()[:40]}"))
        prev = lv
    chapter = chapter_number(path.name)
    if chapter is not None:
        if not p.classes["q"]:
            E.append(("A", "missing end-of-chapter questions (details.q)"))
        eqs, bad = [], []
        for n in filter(None, p.nos):
            m = re.fullmatch(r"\((\d+)\.(\d+)(?:\s*[–-]\s*(?:(\d+)\.)?(\d+))?\)", n)     # «(9.1)» or a range «(9.4–9.5)»
            if m:
                eqs += [(int(m[1]), k) for k in range(int(m[2]), int(m[4] or m[2]) + 1)]
            else:
                bad.append(n)
        E += numbering("equation", chapter, eqs, bad)
        figs = []
        for c in p.caps:
            m = re.match(r"\s*(?:Figura|Fig\.)\s*(\d+)\.(\d+)", c)
            if m:
                figs.append((int(m[1]), int(m[2])))
        E += numbering("figure", chapter, figs, [])
    for h in p.hrefs:
        e = link_error(h, path, folder, p)
        if e:
            E.append(("A", e))
    # (b) accessibility
    for a, sim in p.svgs:
        if a.get("aria-hidden") == "true" or sim:
            continue
        label = (a.get("aria-label") or "").strip()
        if a.get("role") != "img" or not label:
            E.append(("B", "svg without role=img and aria-label"))
        elif len(label) > LABEL_MAX:
            E.append(("B", f"aria-label longer than {LABEL_MAX} characters: {label[:30]}..."))
    for a, sim in p.canvases:
        if not sim and not (a.get("aria-label") or "").strip():
            E.append(("B", "canvas without aria-label"))
    for f in p.figures:
        if f["sim"]:
            continue
        if f["svg"]:
            ids = {a.get("aria-describedby") for a, _ in p.svgs}
            if not (ids - {None}):
                E.append(("B", "figure with svg without aria-describedby long description"))
        if not (f["svg"] or f["canvas"] or f["img"]):
            E.append(("B", "figure without svg, canvas or img"))
    for a, _ in p.svgs:
        d = a.get("aria-describedby")
        if d and d not in p.ids:
            E.append(("B", f"aria-describedby points to missing id {d}"))
    for d in p.figdescs:
        n = len(d.split())
        if n > DESC_MAX:
            E.append(("B", f"long description of {n} words (max {DESC_MAX}): {d.strip()[:30]}..."))
    for im in p.imgs:
        if "alt" not in im:
            E.append(("B", f"img without alt: {im.get('src', '')}"))
    for t in p.tables:
        if not t["th"]:
            E.append(("B", "table without header cells (th)"))
        if not t["caption"]:
            E.append(("B", "table without caption"))
    css = " ".join(p.styles)
    if re.search(r"background(?:-color)?\s*:\s*(?:#fff(?:fff)?\b|white\b|rgb\(\s*255\s*,\s*255\s*,\s*255\s*\))", css, re.I):
        E.append(("B", "fixed white background in inline style"))
    # (e) markers
    verified = info.get("verification") == "verified"
    if verified or not esteso:
        for m in sorted(set(re.findall(r"TODO|\?\?\?", text)) | {m.lower() for m in re.findall(r"da verificare", text, re.I)}):
            E.append(("E", f"marker '{m}' in a {'verified' if verified else 'published'} chapter"))
    # (f) extended chapters
    if esteso:
        if not any(m.get("name") == "robots" and "noindex" in m.get("content", "") for m in p.metas):
            E.append(("F", "extended chapter without <meta name=robots content=noindex>"))
        if info.get("verification") not in ("draft", "verified"):
            E.append(("F", "extended chapter: header verification must be draft or verified"))
    return E


def numbering(kind, chapter, found, bad):
    E = [("A", f"{kind} number not in the form N.k: {b}") for b in bad]
    ks = []
    for n, k in [f for f in found if f]:
        if n != chapter:
            E.append(("A", f"{kind} {n}.{k} does not belong to chapter {chapter}"))
        else:
            ks.append(k)
    for k, c in Counter(ks).items():
        if c > 1:
            E.append(("A", f"{kind} {chapter}.{k} appears {c} times"))
    for k in range(1, max(ks, default=0) + 1):
        if k not in ks:
            E.append(("A", f"{kind} {chapter}.{k} is missing (gap)"))
    return E


def link_error(h, path, folder, page):
    if re.match(r"(?:[a-z][a-z0-9+.-]*:|//)", h, re.I):
        return None
    f, _, frag = h.partition("#")
    f = f.split("?")[0]
    if not f:
        if frag and frag not in page.ids:
            return f"anchor #{frag} not found in the page"
        return None
    target = (path.parent / f)
    if not target.exists():
        # a draft outside the site may point at site pages: look in the Teoria folder too
        target = TEORIA / f
        if not target.exists():
            return f"link to missing file {f}"
    if frag and target.suffix == ".html":
        ids = set(re.findall(r'\bid="([^"]+)"', target.read_text(encoding="utf-8", errors="replace")))
        if frag not in ids:
            return f"anchor {f}#{frag} not found"
    return None


def visible_text(text):
    text = re.sub(r"<!--.*?-->|<(script|style)\b.*?</\1>", " ", text, flags=re.S)
    return re.sub(r"<[^>]+>", " ", text)


def load_private():
    """(forbidden terms, set of hashed 10-grams, warnings)."""
    d = radice()
    warn = []
    if d is None:
        return None, None, ["private data repository not found: checks (c) forbidden names and (d) copying from the sources skipped"]
    terms = None
    f = d / "nomi_vietati.txt"
    if f.exists():
        terms = [t.strip().lower() for t in f.read_text(encoding="utf-8").splitlines() if t.strip() and not t.startswith("#")]
    else:
        warn.append("nomi_vietati.txt not found in the data repository: check (c) skipped")
    grams = None
    src = sorted((d / "Sbobine_elaborate" / "testo").glob("*.txt"))
    if src:
        grams = set()
        for s in src:
            w = words(s.read_text(encoding="utf-8", errors="replace"))
            grams.update(hash(tuple(w[i:i + RUN])) for i in range(len(w) - RUN + 1))
    else:
        warn.append("Sbobine_elaborate/testo/*.txt not found in the data repository: check (d) skipped")
    return terms, grams, warn


def private_errors(text, terms, grams):
    E = []
    plain = " ".join(words(visible_text(text)))
    for t in terms or []:
        if re.search(rf"(?<!\w){re.escape(' '.join(words(t)))}(?!\w)", plain):
            E.append(("C", f"forbidden name present: {t}"))
    if grams:
        w = plain.split()
        for i in range(len(w) - RUN + 1):
            if hash(tuple(w[i:i + RUN])) in grams:
                E.append(("D", f"{RUN}+ consecutive words identical to a source: «{' '.join(w[i:i + RUN])}…»"))
                break
    return E


def pages(path):
    path = Path(path)
    return [path] if path.is_file() else sorted(path.glob("*.html"))


def run(path, base, use_private=True):
    """Returns (new errors {file: [msg]}, all errors {file: [(code,msg)]}, warnings)."""
    terms = grams = None
    warns = []
    if use_private:
        terms, grams, warns = load_private()
    allerr = {}
    for p in pages(path):
        text = p.read_text(encoding="utf-8")
        e = public_errors(p, p.parent, text)
        if terms or grams:
            e += private_errors(text, terms, grams)
        allerr[p.name] = e
    new = {}
    for name, errs in allerr.items():
        known = Counter(base.get(name, []))
        for code, msg in errs:
            if code not in PRIVATE and known[f"{code}: {msg}"] > 0:
                known[f"{code}: {msg}"] -= 1
            else:
                new.setdefault(name, []).append(f"{code}: {msg}")
    return new, allerr, warns


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--path", default=str(TEORIA), help="page or folder (default: mzlab/web/teoria)")
    ap.add_argument("--base", action="store_true", help="rewrite tools/teoria_base.json with the current errors (public checks only)")
    ap.add_argument("--base-file", default=str(BASE))
    ap.add_argument("--senza-base", action="store_true", help="ignore the base: every error counts")
    a = ap.parse_args(argv)
    bf = Path(a.base_file)
    if a.base:
        _, allerr, _ = run(a.path, {}, use_private=False)
        data = {n: sorted(f"{c}: {m}" for c, m in e) for n, e in allerr.items() if e}
        bf.write_text(json.dumps(data, indent=1, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        print(f"base written: {sum(map(len, data.values()))} errors in {len(data)} pages")
        return 0
    base = {} if a.senza_base or not bf.exists() else json.loads(bf.read_text(encoding="utf-8"))
    new, allerr, warns = run(a.path, base)
    for w in warns:
        print("warning:", w, file=sys.stderr)
    for n, errs in new.items():
        for e in errs:
            print(f"{n}: {e}")
    known = sum(len(e) for e in allerr.values()) - sum(map(len, new.values()))
    print(f"controlla_teoria: {len(allerr)} pages, {sum(map(len, new.values()))} new errors, {max(known, 0)} known (base)")
    return 1 if new else 0


if __name__ == "__main__":
    sys.exit(main())
