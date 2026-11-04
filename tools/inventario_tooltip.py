#!/usr/bin/env python3
"""Inventory of every tooltip site (title attributes, .title=, data-tip, hover tips) in the pages.

Scans mzlab/web, TP_Mine/js and flow/lazy; resolves I18N keys against lang/ (it, en) and prints
one TSV row per site: file:line, kind, key, it, en, visible text next to it.
Used by docs/inventari/TOOLTIP*.md (see docs/inventari/TOOLTIP.md); read-only, no dependency.
"""
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SKIP = ("vendor/", "/esempi/", "elements.js", "glossario-dati.js", "ei-dati.js", "/lang/", "node_modules")
ROOTS = ["mzlab/web", "TP_Mine/js", "TP_Mine/lazy", "flow/lazy"]
KV = re.compile(r'^\s*"([^"]+)":\s*("(?:[^"\\]|\\.)*")\s*,?\s*$')


def catalogs():
    cat = {"it": {}, "en": {}}
    for lang in cat:
        files = [ROOT / f"mzlab/web/lang/{lang}.js"] + sorted((ROOT / f"mzlab/web/lang/{lang}").glob("*.js"))
        for f in files:
            for ln in f.read_text(encoding="utf-8").splitlines():
                m = KV.match(ln)
                if m:
                    try:
                        cat[lang][m.group(1)] = json.loads(m.group(2))
                    except ValueError:
                        pass
    return cat


# a tooltip site: attribute, property, setAttribute, data-i18n-title, data-tip(h), tip: field
SITE = re.compile(
    r'(?P<attr>\btitle=\\?")'
    r'|(?P<prop>\.title\s*=\s*(?P<pv>[^;]+))'
    r'|(?P<sa>setAttribute\(\s*["\']title["\']\s*,\s*(?P<sv>[^;]+?)\)\s*[;,)])'
    r'|(?P<dt>\bdata-tiph?=\\?"(?P<dv>[^"]*)\\?")'
    r'|(?P<tf>\btip:\s*(?P<tv>.+?)[,}]\s*(?:$|[a-z]+:))')
KEY = re.compile(r'''(?:I18N\.t|\bt|_t|tr)\(\s*["']([A-Za-z0-9_.\-]+)["']''')
I18N_TITLE = re.compile(r'data-i18n-title="([^"]+)"')
ARIA = re.compile(r'(?:aria-label=\\?"([^"]*)\\?"|data-i18n-aria-label="([^"]+)")')
SVGT = re.compile(r"<title>(.*?)</title>")
INNER_FULL = re.compile(r">([^<>]{1,80})</")
INNER = re.compile(r'>([^<>]{1,60})</')


def attr_value(line, start):
    """Text of an attribute value starting after the opening quote, skipping ${...} blocks."""
    i, depth, out = start, 0, []
    while i < len(line):
        c = line[i]
        if depth == 0 and c == "$" and line[i + 1:i + 2] == "{":
            depth, i = 1, i + 2
            out.append("${")
            continue
        if depth:
            depth += (c == "{") - (c == "}")
            out.append(c)
        elif c == "\\":
            i += 1
        elif c == '"':
            break
        else:
            out.append(c)
        i += 1
    return "".join(out)


def scan():
    cat = catalogs()
    rows = []
    files = []
    for r in ROOTS:
        for ext in ("*.js", "*.html"):
            files += sorted((ROOT / r).rglob(ext))
    for f in files:
        rel = f.relative_to(ROOT).as_posix()
        if any(s in "/" + rel for s in SKIP):
            continue
        for n, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            if "title" not in line and "data-tip" not in line and "tip:" not in line:
                continue
            for m in SITE.finditer(line):
                kind = next(k for k in ("attr", "prop", "sa", "dt", "tf") if m.group(k))
                if kind == "attr":
                    val = attr_value(line, m.end()).strip()
                else:
                    val = m.group({"prop": "pv", "sa": "sv", "dt": "dv", "tf": "tv"}[kind]).strip()
                if kind == "tf" and "I18N" not in val and '"' not in val:
                    continue
                keys = [k for k in re.findall(r'["`\']([A-Za-z0-9_.\-]+)["`\']', val) if k in cat["it"]]
                key = "+".join(keys[:2])
                lit = "" if key else val[:90]
                near = ARIA.search(line)
                inner = INNER.findall(line)
                tag, handle, vis = element(line, m.start(), cat) if kind in ("attr", "dt") else ("", "", "")
                rows.append(dict(file=rel, line=n, kind=kind, key=key, lit=lit, tag=tag, handle=handle, vis=vis,
                                 it=" // ".join(cat["it"][k] for k in keys[:2]),
                                 en=" // ".join(cat["en"].get(k, "") for k in keys[:2]),
                                 aria=(near.group(1) or near.group(2) or "") if near else "",
                                 inner=" | ".join(i.strip() for i in inner if i.strip())[:60],
                                 src=line.strip()[:200]))
            for m in I18N_TITLE.finditer(line):
                key = m.group(1)
                keys = [key]
                near = ARIA.search(line)
                tag, handle, vis = element(line, m.start(), cat)
                rows.append(dict(file=rel, line=n, kind="i18n", key=key, lit="", tag=tag, handle=handle, vis=vis, it=" // ".join(cat["it"][k] for k in keys[:2]),
                                 en=" // ".join(cat["en"].get(k, "") for k in keys[:2]), aria=(near.group(1) or near.group(2) or "") if near else "",
                                 inner=" | ".join(INNER.findall(line))[:60], src=line.strip()[:200]))
    for f in files:
        rel = f.relative_to(ROOT).as_posix()
        if any(sk in "/" + rel for sk in SKIP) or rel.endswith(".html"):
            continue
        for n, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            for m in SVGT.finditer(line):
                keys = [k for k in re.findall(r'["`\']([A-Za-z0-9_.\-]+)["`\']', m.group(1)) if k in cat["it"]]
                rows.append(dict(file=rel, line=n, kind="svg", key="+".join(keys[:2]), lit="" if keys else m.group(1)[:90],
                                 tag="svg", handle="", vis="", it=" // ".join(cat["it"][k] for k in keys[:2]),
                                 en=" // ".join(cat["en"].get(k, "") for k in keys[:2]), aria="", inner="", src=line.strip()[:200]))
    i18n_lines = {(r["file"], r["line"]) for r in rows if r["kind"] == "i18n"}
    rows = [r for r in rows if r["kind"] == "i18n" or (r["file"], r["line"]) not in i18n_lines]
    return rows


TAG = re.compile(r"<([a-zA-Z][a-zA-Z0-9]*)\b")


def element(line, pos, cat):
    """Tag, handle (data-a / id / class) and visible text of the element whose attributes hold `pos`."""
    st = None
    for m in TAG.finditer(line, 0, pos):
        st = m
    if not st:
        return "", "", ""
    end = line.find(">", pos)
    attrs = line[st.start():end if end > 0 else len(line)]
    h = re.search(r'(?:data-a|id|data-nav|data-o|class)=\\?"([^"\\]+)', attrs)
    inner = ""
    if end > 0:
        nxt = line.find("<", end + 1)
        inner = line[end + 1:nxt if nxt > 0 else len(line)]
    txt = []
    for frag in re.split(r"(\$\{[^}]*\})", inner):
        if frag.startswith("${"):
            ks = [k for k in re.findall(r'["`\']([A-Za-z0-9_.\-]+)["`\']', frag) if k in cat["it"]]
            txt.append(cat["it"][ks[0]] if ks else ("[icon]" if re.search(r"\bIC_|ICONS|svg|CIC|chevron|raw\(", frag) else "[dyn]"))
        else:
            txt.append(frag)
    t = re.sub(r"&times;", "×", "".join(txt)).strip()
    return st.group(1), (h.group(1) if h else ""), t[:60]


ICONISH = re.compile(r"^(?:\$\{\s*(?:IC_\w+|ICONS\.\w+|CIC\[[^}]*\]|[\w.]*icon\w*[^}]*)\}|<svg|&times;|&#\d+;|[^\w\s]{1,2}\s*$)", re.I)


def gaps():
    """Icon-only buttons with no title / data-tip / data-i18n-title (they need the tooltip with name and shortcut)."""
    out = []
    files = []
    for r in ROOTS:
        for ext in ("*.js", "*.html"):
            files += sorted((ROOT / r).rglob(ext))
    for f in files:
        rel = f.relative_to(ROOT).as_posix()
        if any(s in "/" + rel for s in SKIP):
            continue
        for n, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            for m in re.finditer(r"<button\b([^>]*)>([^<]{0,60}|<svg)", line):
                attrs, inner = m.group(1), m.group(2).strip()
                if re.search(r"\btitle=|data-tip|data-i18n-title", attrs) or not ICONISH.match(inner):
                    continue
                out.append((rel, n, inner[:20], (re.search(r'aria-label=\\?"([^"]*)', attrs) or [None, ""])[1][:50]))
    return out


def visible(r, cat):
    """Visible text next to the tooltip: inner text of tags on the line with i18n keys resolved."""
    parts = []
    for frag in INNER_FULL.findall(r["src"]):
        km = re.findall(r'["`\']([A-Za-z0-9_.\-]+)["`\']', frag)
        txt = " ".join(cat["it"][k] for k in km if k in cat["it"]) if "${" in frag else frag
        if txt.strip():
            parts.append(txt.strip())
    return " | ".join(parts)[:80]


# Classes: A useful (adds information), B redundant (repeats what is visible), C to redo, D to remove, - not a tooltip.
# Keys are I18N keys or "file:line" for sites without one (paths relative to mzlab/web unless prefixed).
NOT_TOOLTIP = {"appname.js:6", "appname.js:8", "avvio.js:4", "banco.js:63", "banco.js:67", "dda.js:118", "dda.js:299", "dda.js:308",
               "eventi.js:274", "pannelli.js:161", "pannelli.js:358", "pannelli.js:649", "pannelli.js:651", "pannelli-annulla.js:25",
               "tabs.js:206", "tabs.js:208", "index.html:8", "shell.js:137", "xic-lente.js:115", "llm/llm-clienti.js:56",
               "TP_Mine/js/20_tpmine.js:200", "rapporto.js:76", "draw.js:701", "TP_Mine/js/38_feature_export.js:27"}
OVERRIDE = {
    "etpk.tip.rt": ("B", "repeats the RT already written on the label (CROMA-CLICK-PICCO)"),
    "tip.area": ("D", "«Right-click: actions» line (CROMA-CLICK-PICCO)"),
    "tip.peakLabel": ("D", "only «Right-click: actions» (CROMA-CLICK-PICCO)"),
    "tip.peak": ("C", "spectrum hover: not the same layout as the chromatogram hover (CROMA-CLICK-PICCO)"),
    "liste-target.js:75": ("C", "English literal in the code, not in lang/"),
    "liste-target.js:145": ("C", "English literal in the code, not in lang/"),
    "draw.ann.tip": ("C", "bold text repeats the note; the gestures are the only new information"),
    "spec.dm.tip.ad": ("C", "gesture hint only (to merge with the common hover)"),
    "spec.dm.tip.ls": ("C", "gesture hint only (to merge with the common hover)"),
}
ZONES = [
    ("pannelli", "Panels, chart toolbars, bar, legends", ("pannelli", "grafici", "barra", "banco", "eventi", "scroll", "touch", "affianca", "xic-", "sottrazione", "fondo", "gradiente", "canali", "tdati", "etichette", "azioni", "visualizza", "paletta", "avanzamento")),
    ("spettri", "Spectrum, map, DDA, XIC, integration", ("spettro", "picchi", "mappa", "dda", "uv-", "metodo", "origine", "kmd", "pca", "qaqc", "profilo", "pfas", "ident", "calc", "elementare", "composizione")),
    ("liste", "Files, lists, libraries, tables, calculator", ("explore", "lista", "liste", "libreria", "tables", "tabella", "campioni", "shell", "conversione", "ruoli", "settings", "aggiorna", "predefinito", "perf", "help", "registro", "rapporto", "fair", "sito", "telefono", "index.html", "tpmine-loader", "llm", "libro")),
    ("disegno", "Draw, fragmentation graph, infusion", ("draw", "disegno", "infusione", "molecole", "msn")),
    ("tabui", "Tabui (TP_Mine) and Flow (English only)", ("TP_Mine", "flow/")),
]


def zone(rel):
    name = rel.replace("mzlab/web/", "")
    for z, _, pre in ZONES:
        if any(name.startswith(p) or ("/" + p) in "/" + name and p.endswith("/") for p in pre):
            return z
    return "liste"


def classify(r):
    rel = r["file"].replace("mzlab/web/", "")
    site = f"{rel}:{r['line']}"
    if site in NOT_TOOLTIP:
        return "-", "property of the panel / page, not a tooltip"
    for k in (r["key"], *r["key"].split("+"), site):
        if k in OVERRIDE:
            return OVERRIDE[k]
    text = r["it"] or r["lit"]
    vis = re.sub(r"\[(?:icon|dyn)\]", "", r["vis"]).strip()
    if r["kind"] == "svg":
        return "A", "hover on a point of an SVG chart"
    if not text:
        return "A", "text built at run time"
    if r["tag"] and (not vis or len(vis) <= 2):
        return "A", "icon-only: name" + (" + explanation" if len(text) > 30 else "")
    return "A", "explains what the visible text does" if len(text) > len(vis) + 12 else "short"


def md_rows():
    rows = scan()
    out = {z[0]: [] for z in ZONES}
    for r in rows:
        r["cl"], r["note"] = classify(r)
        out[zone(r["file"])].append(r)
    return out


def esc_md(x, n):
    x = re.sub(r"<[^>]+>", "", x or "").replace("|", "¦").replace("\n", " ").strip()
    return x[:n] + ("…" if len(x) > n else "")


def write_md(outdir):
    from collections import Counter
    outdir = Path(outdir)
    zones = md_rows()
    tot = Counter()
    files = []
    for zkey, ztitle, _ in ZONES:
        rs = zones[zkey]
        for r in rs:
            tot[r["cl"]] += 1
        parts, cur, size = [], [], 0
        for r in sorted(rs, key=lambda r: (r["file"], r["line"])):
            row = (f"| {r['file'].replace('mzlab/web/', '').replace('TP_Mine/', 'TP/')}:{r['line']} | {esc_md(r['handle'] or r['tag'] or r['kind'], 14)} "
                   f"| {esc_md(r['it'] or r['lit'], 90)} | {esc_md(r['en'], 60)} | {esc_md(r['vis'], 24)} | {r['cl']} | {esc_md(r['note'], 60)} |")
            if size + len(row) > 21000:
                parts.append(cur)
                cur, size = [], 0
            cur.append(row)
            size += len(row) + 1
        parts.append(cur)
        for i, part in enumerate(parts, 1):
            name = f"TOOLTIP-{zkey}{'' if len(parts) == 1 else '-' + str(i)}.md"
            head = (f"# Tooltip inventory: {ztitle}" + ("" if len(parts) == 1 else f" ({i}/{len(parts)})") +
                    "\n\nGenerated by `python3 tools/inventario_tooltip.py --md docs/inventari`; do not edit by hand. "
                    "Classes and rules: `TOOLTIP.md`.\n\n| file:line | element | IT | EN | visible next to it | class | note |\n|---|---|---|---|---|---|---|\n")
            (outdir / name).write_text(head + "\n".join(part) + "\n", encoding="utf-8")
            files.append((name, len(part)))
    return tot, files


if __name__ == "__main__":
    if "--md" in sys.argv:
        tot, files = write_md(sys.argv[sys.argv.index("--md") + 1])
        print(dict(tot), files)
        sys.exit(0)
    if "--gaps" in sys.argv:
        for g in gaps():
            print("\t".join(map(str, g)))
        sys.exit(0)
    rows = scan()
    if "--json" in sys.argv:
        print(json.dumps(rows, ensure_ascii=False, indent=1))
    else:
        for r in rows:
            print("\t".join(str(r[k]) for k in ("file", "line", "kind", "key", "it", "en", "lit", "aria", "inner")))
        print(len(rows), "sites", file=sys.stderr)
