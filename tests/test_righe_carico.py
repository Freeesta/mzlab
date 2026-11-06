"""Progress rows of the side list (mzlab/web/upload-rows.js, run by node over a small fake DOM): the box is written again only when its shape changes (a row comes
or goes, a file fails, the count line appears); a new percent, a new phase or a new count only change the rows that are there, and the box is never read back."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

# innerHTML = "…" makes one fake row per data-up (its text, title, bar and label as one() wrote them) and the count line if there is one; reading it back is counted
SETUP = r"""
const log = { writes: 0, reads: 0 };
const el = () => { const n = { attrs: {}, style: {}, dataset: {}, textContent: "", title: "", getAttribute: k => n.attrs[k] ?? null, setAttribute: (k, v) => { n.attrs[k] = String(v); },
  querySelector: () => null, querySelectorAll: () => [] }; return n; };
const unesc = s => s.replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
const box = el(); box.id = "fl-up"; let html = "", kids = [], cnt = null;
Object.defineProperty(box, "innerHTML", { get() { log.reads++; return html; }, set(v) { log.writes++; html = v; kids = []; cnt = null;
  for (const seg of v.split('<div class="fl pend up').slice(1)) {
    const m = /data-up="([^"]*)" title="([^"]*)"[\s\S]*?data-uptxt>([^<]*)</.exec(seg), p = /aria-valuenow="(\d+)" aria-label="([^"]*)"/.exec(seg); if (p) m.push(p[1], p[2]);
    const row = el(), t = el(), bar = el(), fill = el(); row.dataset.up = unesc(m[1]); row.title = unesc(m[2]); t.textContent = unesc(m[3]); row.parentElement = box;
    if (m[4]) { bar.attrs["aria-valuenow"] = m[4]; bar.attrs["aria-label"] = unesc(m[5]); fill.style.width = m[4] + "%"; bar.querySelector = () => fill; }
    row.querySelector = s => s === "[data-uptxt]" ? t : s === ".prog" && m[4] ? bar : null; row.t = t; row.bar = bar; row.fill = fill; kids.push(row); }
  const c = /data-cvhead>([^<]*)</.exec(v); if (c) { cnt = el(); cnt.textContent = unesc(c[1]); } } });
Object.defineProperty(box, "children", { get: () => kids });
box.querySelector = s => s === "[data-cvhead]" ? cnt : null;
const flst = el(); flst.appendChild = () => {};
globalThis.window = globalThis; globalThis.CSS = { escape: s => s };
globalThis.document = { getElementById: id => id === "fl-up" ? box : id === "flst" ? flst : null,
  querySelector: s => { const m = /^#fl-up \[data-up="(.*)"\]$/.exec(s); return m ? kids.find(k => k.dataset.up === m[1]) || null : null; } };
globalThis.I18N = { t: k => k }; globalThis.EH = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
globalThis.ST = { files: [] }; globalThis.GORD = {}; globalThis.OPENNOW = { isOpen: () => false, busy: () => 1 };
const a = { name: "a.mzML", state: "index", pc: 10 }, b = { name: "b.mzML", state: "read", pc: 0 };
let done = 0;
globalThis.CONVROWS = { rows: [a], rank: n => n === "a.mzML" ? 0 : 1, short: x => ({ txt: x.state, num: x.pc + "%" }), label: x => x.state + " " + x.pc,
  running() { return this.rows.filter(r => r.state !== "fail"); }, ready: () => done + " ready", onCancel: null, find: n => CONVROWS.rows.find(r => r.name === n), fill() {}, paint() {} };
const row = n => kids.find(k => k.dataset.up === n);
"""


def run(code):
    up = (WEB / "upload-rows.js").read_text(encoding="utf-8")
    js = SETUP + f"""require("vm").runInThisContext({json.dumps(up)});
{code}
console.log(JSON.stringify(log));"""
    return json.loads(subprocess.run(["node", "-"], input=js, check=True, capture_output=True, encoding="utf-8").stdout)


@needs_node
def test_a_repaint_with_nothing_new_writes_nothing_and_reads_nothing():
    assert run("UPROWS.paint(); UPROWS.paint(); UPROWS.paint();") == {"writes": 1, "reads": 0}


@needs_node
def test_a_new_percent_and_phase_change_the_row_in_place():
    out = run("""UPROWS.paint(); a.pc = 40; a.state = "nscan"; UPROWS.paint(); const r = row("a.mzML");
log.t = r.t.textContent; log.title = r.title; log.now = r.bar.attrs["aria-valuenow"]; log.w = r.fill.style.width; log.lab = r.bar.attrs["aria-label"];""")
    assert out == {"writes": 1, "reads": 0, "t": "nscan 40%", "title": "nscan 40", "now": "40", "w": "40%", "lab": "a.mzML: nscan 40"}


@needs_node
def test_the_count_line_changes_in_place_and_its_coming_writes_the_box():
    out = run("""UPROWS.paint(); CONVROWS.onCancel = () => {}; UPROWS.paint(); const w = log.writes; done = 1; UPROWS.paint(); log.w = w; log.cnt = box.querySelector("[data-cvhead]").textContent;""")
    assert out == {"writes": 2, "reads": 0, "w": 2, "cnt": "1 ready"}


@needs_node
def test_a_new_row_or_a_failure_writes_the_box_again():
    assert run("""UPROWS.paint(); CONVROWS.rows.push(b); UPROWS.paint(); UPROWS.paint(); b.state = "fail"; UPROWS.paint(); log.n = kids.length;""") == {"writes": 3, "reads": 0, "n": 2}


@needs_node
def test_a_new_percent_from_the_loading_fills_its_row_only():
    out = run("""UPROWS.paint(); a.pc = 70; CONVROWS.fill(a); log.now = row("a.mzML").bar.attrs["aria-valuenow"]; UPROWS.paint();""")
    assert out == {"writes": 1, "reads": 0, "now": "70"}
