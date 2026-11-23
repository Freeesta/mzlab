#!/usr/bin/env python3
"""Complete inventory of the user-visible tooltip texts, rendered as Markdown and CSV in memory.

Sources: tooltip sites in the pages (tools/inventory_tooltip.scan), catalog keys ending .title/.tip/.tt/.hint
that no site reaches by name, and the Flow step/parameter help (docs/schemi/steps-*.json). Every entry gets
automatic rule-20/21 flags. Read-only: nothing is written; `--md` / `--csv` print to stdout. No dependency.
"""
import csv, io, json, re, sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import inventory_tooltip as INV  # noqa: E402

ROOT = INV.ROOT
HINT_KEY = re.compile(r"(\.title|\.tip|\.tt|\.hint|Title)$|^tip\.")
AREAS = {"panels": "Panels, chart toolbars, bar, legends", "spectra": "Spectrum, map, DDA, XIC, integration",
         "lists": "Files, lists, libraries, tables, calculator", "drawing": "Draw, fragmentation graph, infusion",
         "tabui": "Tabui (TP_Mine, English only)", "flow-ui": "Flow buttons and menus (English only)",
         "flow-nodi": "Flow steps and parameters (English only)", "catalogo": "Catalog keys not reached by a site"}
FLAG_RULES = [  # (reason, regex)
    ("non", r"\b(not|never|no longer|doesn.t|does not|isn.t)\b"),
    ("invece", r"\binstead\b"),
    ("puoi", r"\b(you can|you may|can be)\b"),
    ("consigliamo", r"recommend|suggest"),
    ("riaperto", r"reopen"),
    ("privacy", r"privacy|\bsent\b|\bsends?\b|upload|server|cloud|your computer|stays in|never\b"),
]
DYN = re.compile(r"\$\{|\{[A-Za-z_]\w*\}|%[sd]")
SENT = re.compile(r"[.!?;:](?:\s+|$)")


def sentences(t):
    """Number of sentences: terminators followed by a capital or digit, not abbreviations or decimals."""
    t = re.sub(r"\b(ca|es|vs|cfr|e\.g|i\.e|Fig|n|nr)\.", r"\1", t)
    return 1 + len(re.findall(r"[.!?]\s+[A-ZÀ-ÖØ-Þ0-9«\"“]", t))


def norm(x):
    return re.sub(r"[^a-z0-9]+", " ", re.sub(r"<[^>]+>", "", x or "").lower()).strip()


def flags(en, label, dynamic):
    out = []
    for reason, ren in FLAG_RULES:
        if en and re.search(ren, en, re.I):
            out.append(reason)
    if dynamic or DYN.search(en or ""):
        out.append("dinamico")
    if len(en or "") > 90:
        out.append(">90")
    n = norm(label)
    if n and len(n) > 2 and en and (norm(en) == n or (len(n) > 6 and norm(en).startswith(n + " "))):
        out.append("ripete-etichetta")
    if sentences(en or "") > 1:
        out.append("due-frasi")
    return out


def key_lines():
    """key -> (relative file, line) of its definition in the catalog."""
    loc = {}
    for f in sorted((ROOT / "mzlab/web/lang/en").glob("*.js")):
        for n, ln in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            m = INV.KV.match(ln)
            if m:
                loc[m.group(1)] = (f.relative_to(ROOT).as_posix(), n)
    return loc


def area_of(rel):
    if rel.startswith("TP_Mine/"):
        return "tabui"
    if rel.startswith("flow/"):
        return "flow-ui"
    return INV.zone(rel)


def entries():
    cat, loc = INV.catalogs(), key_lines()
    rows = INV.scan()
    ents, skipped, used = [], [], set()
    for r in rows:
        rel = r["file"]
        site = f"{rel.replace('mzlab/web/', '')}:{r['line']}"
        if site in INV.NOT_TOOLTIP:
            skipped.append((f"{rel}:{r['line']}", "property of the panel or page, not a tooltip"))
            continue
        keys = [k for k in r["key"].split("+") if k]
        used.update(keys)
        en = r["en"]
        dyn = False
        if not keys:
            en = r["lit"]
            dyn = "${" in en or "+" in en or "`" in en
        label = re.sub(r"\[(?:icon|dyn)\]", "", r["vis"]).strip() or r["aria"]
        src = "svg" if r["kind"] == "svg" else ("i18n-title" if r["kind"] == "i18n" else "site")
        ents.append(dict(area=area_of(rel), label=label, key="+".join(keys), where=f"{rel}:{r['line']}", en=en,
                         source=src, kind=r["kind"], dynamic=dyn or (not keys and not r["lit"])))
    for k, v in cat["en"].items():
        if k in used or not HINT_KEY.search(k) or re.match(r"^(help\.|tour\.|og\.s\d\.|.*\.dialog\.title$)", k):
            continue
        f, n = loc[k]
        ents.append(dict(area="catalogo", label="", key=k, where=f"{f}:{n}", en=v,
                         source="catalog-only", kind="key", dynamic=False))
    for f in ("steps-1", "steps-attributio-1", "steps-cli-1", "steps-tabui-1"):
        p = f"docs/schemi/{f}.json"
        for s in json.loads((ROOT / p).read_text(encoding="utf-8")):
            if s.get("doc"):
                ents.append(dict(area="flow-nodi", label=s["title"], key=s["id"], where=p, en=s["doc"],
                                 source="flow-step", kind="step.doc", dynamic=False))
            for name, q in ((s.get("params") or {}).get("properties") or {}).items():
                t = q.get("x-tip") or q.get("description")
                if t:
                    ents.append(dict(area="flow-nodi", label=q.get("x-label") or q.get("title") or name,
                                     key=f"{s['id']}.{name}", where=p, en=t, source="flow-param", kind="param.tip", dynamic=False))
    for i, e in enumerate(ents):
        e["chars"] = len(e["en"] or "")
        e["flags"] = flags(e["en"], e["label"], e["dynamic"] or not e["en"])
        if not e["en"]:
            e["en"] = "(built at run time)"
            e["chars"] = 0
    ents.sort(key=lambda e: (list(AREAS).index(e["area"]), not e["flags"], e["where"], e["key"]))
    for i, e in enumerate(ents, 1):
        e["id"] = i
    return ents, skipped, INV.gaps(), len(rows)


def cell(x, n=400):
    x = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", "", x or "")).replace("|", "¦").strip()
    return x[:n] + ("…" if len(x) > n else "")


def render():
    ents, skipped, gaps, nscan = entries()
    reasons = Counter(r for e in ents for r in e["flags"])
    flagged = sum(1 for e in ents if e["flags"])
    srcs = Counter(e["source"] for e in ents)
    L = ["# Tooltip list (all user-visible tooltip texts)", "",
         "Generated by `python3 tools/list_tooltip.py --md`; start from the flagged rows.", "",
         f"## Summary\n\n{len(ents)} tooltips, {flagged} flagged ({len(ents) - flagged} clean).\n", "| flag | entries |", "|---|---|"]
    L += [f"| {r} | {reasons[r]} |" for r, _ in FLAG_RULES] + [f"| dinamico | {reasons['dinamico']} |", f"| >90 | {reasons['>90']} |",
                                                                 f"| ripete-etichetta | {reasons['ripete-etichetta']} |", f"| due-frasi | {reasons['due-frasi']} |", ""]
    L += ["Flags are automatic suspects, not verdicts: `non`/`invece`/`puoi`/`consigliamo`/`riaperto`/`privacy` = those words appear; "
          "`dinamico` = the text carries a value built at run time; `>90` = longer than 90 characters; `ripete-etichetta` = the text restates the visible label; "
          "`due-frasi` = more than one sentence.", ""]
    L += ["## Method and completeness", "",
          "- **Sites** (`tools/inventory_tooltip.py`): `title=`, `.title =`, `setAttribute('title')`, `data-tip`/`data-tiph`, `tip:` fields, `data-i18n-title` and SVG `<title>` in `mzlab/web`, `TP_Mine/js`, `TP_Mine/lazy`, `flow/lazy`; keys resolved in `lang/en`.",
          "- **Catalog keys** ending `.title`, `.tip`, `.tt`, `.hint` or `Title` (and `tip.*`) that no site names are added as `catalog-only` (headings, tour, help and dialog titles are not tooltips and are left out).",
          "- **Flow**: the hover help of each step (`doc`) and parameter (`x-tip`, else `description`) from `docs/schemi/steps-*.json` (English only).",
          "- **aria-label**: the program's tooltip reads only `title`, `data-tip` and `data-tiph` (`tooltip.js`), so an `aria-label` is hover text only where code copies it into a `title` (those `.title =` sites are in the list).",
          "- **Desktop app**: it serves the same pages; `crates/mzlab-desktop/shim` and `src` set no tooltip text (the window title is the page title).",
          "", "| source | entries |", "|---|---|"]
    L += [f"| {s} | {srcs[s]} |" for s in ("site", "i18n-title", "svg", "catalog-only", "flow-step", "flow-param")]
    L += [f"| **total** | **{len(ents)}** |", "",
          f"Check: scanner sites {nscan} = {len(ents) - srcs['catalog-only'] - srcs['flow-step'] - srcs['flow-param']} listed + {len(skipped)} excluded (appendix A); nothing is skipped silently.", ""]
    for a, title in AREAS.items():
        part = [e for e in ents if e["area"] == a]
        if not part:
            continue
        L += [f"## {title} ({len(part)}; {sum(1 for e in part if e['flags'])} flagged)", "",
              "| # | label | key | file:line | text | chars | flags |", "|---|---|---|---|---|---|---|"]
        for e in part:
            L.append(f"| {e['id']} | {cell(e['label'], 30)} | {e['key']} | {e['where']} | {cell(e['en'])} | {e['chars']} | {', '.join(e['flags'])} |")
        L.append("")
    L += ["## Appendix A: scanner sites that are not tooltips", "", "| file:line | why |", "|---|---|"] + [f"| {w} | {y} |" for w, y in skipped]
    L += ["", "## Appendix B: icon-only buttons with no tooltip", "", "| file:line | icon | aria-label |", "|---|---|---|"] + [f"| {f}:{n} | {cell(i, 20)} | {cell(a, 50)} |" for f, n, i, a in gaps]
    buf = io.StringIO()
    w = csv.writer(buf, lineterminator="\n")
    w.writerow(["id", "area", "label", "key", "file_line", "text", "chars", "flags", "source"])
    for e in ents:
        w.writerow([e["id"], e["area"], e["label"], e["key"], e["where"], e["en"], e["chars"], " ".join(e["flags"]), e["source"]])
    return "\n".join(L) + "\n", buf.getvalue(), dict(total=len(ents), flagged=flagged, reasons=dict(reasons), sources=dict(srcs), skipped=len(skipped), gaps=len(gaps))


if __name__ == "__main__":
    md, cs, stats = render()
    if "--md" in sys.argv:
        print(md, end="")
    elif "--csv" in sys.argv:
        print(cs, end="")
    else:
        print(json.dumps(stats, ensure_ascii=False))
