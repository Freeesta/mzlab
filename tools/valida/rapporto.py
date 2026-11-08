"""Report of the validation: JSON (deterministic, sorted keys) and a Markdown summary."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np


def _clean(x):
    if isinstance(x, dict):
        return {str(k): _clean(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [_clean(v) for v in x]
    if isinstance(x, (np.floating, float)):
        return None if not np.isfinite(x) else float(f"{float(x):.6g}")
    if isinstance(x, np.integer):
        return int(x)
    if isinstance(x, np.bool_):
        return bool(x)
    return x


def without_performance(rapporto: dict) -> dict:
    return {k: v for k, v in rapporto.items() if k != "prestazioni"}


def a_json(rapporto: dict, performance: bool = True) -> str:
    r = rapporto if performance else without_performance(rapporto)
    return json.dumps(_clean(r), indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def _pct(x):
    return "n/a" if x is None else f"{100 * x:.1f}%"


def a_markdown(r: dict) -> str:
    L = [f"# Validation report: {r['serie']['nome']}", "",
         f"Files: {len(r['serie']['file'])} ({', '.join(f['nome'] for f in r['serie']['file'])}); tolerance ±{r['parametri']['ppm']} ppm, ±{r['parametri']['rt_tol']} min.", ""]
    L += ["## Rows", "", "| table | rows | median per file |", "|---|---|---|"]
    for nome, t in r["tabelle"].items():
        L.append(f"| {nome} | {t['righe']['righe']} | {t['righe']['mediana_per_file']:.0f} |")
    L += ["", "## Isotopologue completeness (detectable reference peaks found)", "", "| table | M | M+1 | M+2 | M+1/M error (median, n) | reference traces |", "|---|---|---|---|---|---|"]
    for nome, t in r["tabelle"].items():
        c = t.get("completezza")
        if not c:
            continue
        ri = t["rapporto_isotopico"]
        L.append(f"| {nome} | {c['M']['trovati']}/{c['M']['rilevabili']} | {c['M+1']['trovati']}/{c['M+1']['rilevabili']} | {c['M+2']['trovati']}/{c['M+2']['rilevabili']} | "
                 f"{ri['tabella']['mediana']} ({ri['tabella']['n']}) | {ri['riferimento']['mediana']} ({ri['riferimento']['n']}) |")
    for nome, t in r["tabelle"].items():
        tp = t.get("tp_veri")
        if not tp:
            continue
        s = tp["riassunto"]
        L += ["", f"## True products: {nome}", "", f"Found {s['trovati']}/{s['valutabili']}"
              + (f", {s['nei_candidati']} among the {s['candidati_totali']} candidates, median rank among candidates {s['rango_candidati_mediano']}" if "nei_candidati" in s else "") + ".", "",
              "| id | m/z | RT | found | ppm | rank (all rows) | candidate | rank (candidates) |", "|---|---|---|---|---|---|---|---|"]
        for d in tp["dettaglio"]:
            L.append(f"| {d['id']} | {d['mz']:.4f} | {d['rt']} | {'yes' if d['trovato'] else 'no'} | {d.get('ppm', '')} | {d.get('rango_tutte', '')} | {d.get('candidato', '')} | {d.get('rango_candidati') or ''} |")
    if r.get("bianco"):
        L += ["", "## Rows in the blank", ""] + [f"- {n}: {b['righe_nel_bianco']} of {b['righe_con_segnale']} ({_pct(b['frazione'])})" for n, b in r["bianco"].items() if b]
    if r.get("jaccard"):
        j = r["jaccard"]
        L += ["", "## Jaccard mzFinder vs mzmine (aligned tables)", "", "| class | common | only mzFinder | only mzmine | Jaccard |", "|---|---|---|---|---|",
              f"| all | {j['totale']['comuni']} | {j['totale']['solo_a']} | {j['totale']['solo_b']} | {j['totale']['jaccard']} |"]
        L += [f"| {c} | {v['comuni']} | {v['solo_a']} | {v['solo_b']} | {v['jaccard']} |" for c, v in j["classi"].items()]
    if r.get("sensibilita"):
        L += ["", "## Sensitivity (floor and min height scaled)", "", "| factor | aligned rows | candidates | true products found | in candidates |", "|---|---|---|---|---|"]
        for f, s in r["sensibilita"].items():
            L.append(f"| x{f} | {s['righe']} | {s['candidati']} | {s['tp_trovati']} | {s['tp_nei_candidati']} |")
    if r.get("funnel"):
        L += ["", "## Filter funnel", ""] + [f"- {p['text']}: {p['n']}" for p in r["funnel"]]
    if r.get("prestazioni"):
        p = r["prestazioni"]
        L += ["", "## Time and memory", "", f"- total {p['totale_s']:.0f} s; " + "; ".join(f"{k} {v:.0f} s" for k, v in p["fasi_s"].items()) + f"; peak memory {p['memoria_massima_mb']:.0f} MB"]
    if r.get("note"):
        L += ["", "## Notes", ""] + [f"- {n}" for n in r["note"]]
    return "\n".join(L) + "\n"


def scrivi(folder, rapporto: dict, performance: bool = True) -> tuple[Path, Path]:
    d = Path(folder)
    d.mkdir(parents=True, exist_ok=True)
    j, m = d / "rapporto.json", d / "rapporto.md"
    j.write_text(a_json(rapporto, performance), encoding="utf-8")
    m.write_text(a_markdown(rapporto if performance else without_performance(rapporto)), encoding="utf-8")
    return j, m


# ---------------------------------------------------------------------------------------------------------------- the gate
ESITO = {True: "PASS", False: "FAIL", None: "not measured"}


def cancello_markdown(c: dict, det: dict) -> str:
    """Summary of the gate: the table of the criteria with PASS / FAIL, the result and the details of the measures."""
    L = [f"# Gate of quality: crate {c['crate_version']}, {c['data']}", "",
         f"Table view: **{'open' if c.get('vista_tabella') else 'closed'}**. Candidate view: **{'open' if c['vista_candidati'] else 'closed'}**. Agreement with mzmine: {c['concordanza_mzmine']}.", "",
         "| criterion | value | threshold | outcome |", "|---|---|---|---|"]
    L += [f"| {k['id']} | {'' if k['valore'] is None else k['valore']} | {k['soglia']} | {ESITO[k['ok']]} |" for k in c["criteri"]]
    if det.get("timololo"):
        L += ["", "## Real series", "", "| measure | value |", "|---|---|"] + [f"| {k} | {v} |" for k, v in det["timololo"].items()]
    if det.get("mzmine"):
        L += ["", "## Agreement with mzmine", "", "| measure | value |", "|---|---|"] + [f"| {k} | {v:.4g} |" for k, v in det["mzmine"].items()]
    s = det.get("sintetici")
    if s:
        L += ["", f"## Synthetic series ({s['profilo']}, seed {s['seme']})", "", f"Rows {s.get('righe')}, candidates {s.get('candidati')}.", "", "| measure | value | n |", "|---|---|---|"]
        L += [f"| {k} | {'' if m['valore'] is None else round(m['valore'], 4)} | {m['n']} |" for k, m in s.get("misure", {}).items()]
    return "\n".join(L) + "\n"


def scrivi_cancello(folder, c: dict, det: dict, gate_json: str) -> list[Path]:
    """Writes `cancello.json` (where the panel reads it: `gate_json`) and, in `folder`, the report with all the numbers (`cancello_rapporto.json` / `.md`)."""
    d = Path(folder)
    d.mkdir(parents=True, exist_ok=True)
    text = json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
    out = [Path(gate_json), d / "cancello_rapporto.json", d / "cancello_rapporto.md"]
    out[0].parent.mkdir(parents=True, exist_ok=True)
    out[0].write_text(text, encoding="utf-8")
    out[1].write_text(json.dumps(_clean({"cancello": c, "dettagli": det}), indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    out[2].write_text(cancello_markdown(c, det), encoding="utf-8")
    return out


def gate_tp_markdown(c: dict, det: dict) -> str:
    """Summary of the gate of the TP view: the criteria with PASS / FAIL / not measured and the numbers of the raw view."""
    L = [f"# Gate of the TP view: crate {c['crate_version']}, {c['dataset']}, {c['data']}", "", f"TP view: **{'open' if c['vista_tp'] else 'closed'}**.", "",
         "| criterion | value | threshold | outcome |", "|---|---|---|---|"]
    L += [f"| {k['id']} | {'' if k['valore'] is None else k['valore']} | {'' if k['soglia'] is None else k['soglia']} | {ESITO[k['ok']]} |" for k in c["criteri"]]
    v = det.get("vista_grezza")
    if v:
        L += ["", "## Raw view (candidates of the pipeline)", "", "| measure | value |", "|---|---|"] + [f"| {k} | {x} |" for k, x in v.items()]
    return "\n".join(L) + "\n"


def scrivi_cancello_tp(folder, c: dict, det: dict, gate_tp_json: str) -> list[Path]:
    """Writes `cancello_tp.json` (beside `cancello.json`) and, in `folder`, `cancello_tp_rapporto.json` / `.md`."""
    d = Path(folder)
    d.mkdir(parents=True, exist_ok=True)
    out = [Path(gate_tp_json), d / "cancello_tp_rapporto.json", d / "cancello_tp_rapporto.md"]
    out[0].parent.mkdir(parents=True, exist_ok=True)
    out[0].write_text(json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    out[1].write_text(json.dumps(_clean({"cancello_tp": c, "dettagli": det}), indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    out[2].write_text(gate_tp_markdown(c, det), encoding="utf-8")
    return out
