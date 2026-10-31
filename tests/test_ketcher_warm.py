"""first-paint.js: Ketcher is warmed by ONE policy (low-priority prefetch links, only after the page and any file load are quiet), never read into memory."""
import json, pathlib, shutil, subprocess
import pytest

WEB = pathlib.Path(__file__).resolve().parent.parent / "mzlab/web"
NODE = shutil.which("node")

HARNESS = r'''
const src = require("fs").readFileSync(process.argv[1], "utf8");
function make(o) {
  const log = { fetches: [], links: [], idle: 0 }, tm = [], ev = {}; let now = 0;
  const el = () => ({ attrs: {}, setAttribute(k, v) { this.attrs[k] = v; }, getAttribute(k) { return this.attrs[k] ?? null; } });
  const win = { QQQ_PHONE: !!o.phone, requestIdleCallback: f => { log.idle++; tm.push([now, f]); } };
  const doc = { getElementById: () => Object.assign(el(), { removeAttribute() {}, contentWindow: {} }), write() {}, createElement: () => el(), head: { appendChild: l => log.links.push(l) },
    };
  const ctx = { window: win, document: doc, location: { hash: o.hash || "", href: "http://x/" }, URL, DOMParser: class { parseFromString() {
      return { querySelectorAll: () => [Object.assign(el(), { attrs: { src: "./assets/a.js", crossorigin: "" } })] }; } },
    navigator: { onLine: o.online !== false, connection: o.saveData ? { saveData: true } : undefined },
    fetch: (u, i) => { log.fetches.push([u, i && i.priority]); return Promise.resolve({ text: () => Promise.resolve("<html>"), arrayBuffer: () => { throw new Error("body read"); } }); },
    addEventListener: (n, f) => { (ev[n] = ev[n] || []).push(f); }, setTimeout: (f, d) => tm.push([now + d, f]), Date: { now: () => now } };
  ctx.window.addEventListener = ctx.addEventListener;
  require("vm").runInNewContext(src, Object.assign(ctx, win));
  return { ctx, log, fire: n => (ev[n] || []).forEach(f => f({})),
    at: ms => { while (true) { tm.sort((a, b) => a[0] - b[0]); if (!tm.length || tm[0][0] > ms) break; const [t, f] = tm.shift(); now = Math.max(now, t); f(); } now = ms; } };
}
'''


def run(js):
    r = subprocess.run([NODE, "-e", HARNESS + js, str(WEB / "first-paint.js")], capture_output=True, text=True, encoding="utf-8", timeout=30)
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout)


@pytest.mark.skipif(not NODE, reason="node not available")
def test_warms_once_after_load_plus_quiet_with_low_priority_links():
    out = run('''
(async () => {
const h = make({}); const seen = [];
h.at(1000); h.fire("load"); h.at(2999); seen.push(h.log.fetches.length);
h.at(3999); seen.push(h.log.fetches.length);
h.at(5000); await new Promise(r => setImmediate(r)); const l = h.log.links;
console.log(JSON.stringify({ seen, fetches: h.log.fetches, n: l.length, rel: l[0] && l[0].rel, pr: l[0] && l[0].attrs.fetchpriority }));
})();
''')
    assert out["seen"] == [0, 0] and out["fetches"] == [["static/vendor/ketcher/index.html", "low"]]
    assert out["n"] == 1 and out["rel"] == "prefetch" and out["pr"] == "low"


@pytest.mark.skipif(not NODE, reason="node not available")
def test_a_file_load_in_progress_delays_the_warming():
    out = run('''
const h = make({}); h.at(0); h.fire("load");
h.at(2500); h.fire("mzlab-index"); h.at(5000); const mid = h.log.fetches.length;
h.at(5400); h.fire("mzlab-index"); h.at(8000); const mid2 = h.log.fetches.length;
h.at(8500);
console.log(JSON.stringify({ mid, mid2, end: h.log.fetches.length }));
''')
    assert out == {"mid": 0, "mid2": 0, "end": 1}


@pytest.mark.skipif(not NODE, reason="node not available")
def test_skips_on_save_data_offline_phone_and_direct_draw_but_hover_works_otherwise():
    out = run('''
const r = {};
for (const [k, o] of Object.entries({ save: { saveData: true }, off: { online: false }, phone: { phone: true }, draw: { hash: "#/draw" } })) {
  const h = make(o); h.fire("load"); h.at(60000); r[k] = [h.log.fetches.length, typeof h.ctx.window.MZLAB_WARM_KETCHER];
}
const s = make({ saveData: true }); s.ctx.window.MZLAB_WARM_KETCHER(); s.ctx.window.MZLAB_WARM_KETCHER();
r.hover = s.log.fetches.length;
console.log(JSON.stringify(r));
''')
    assert out["save"][0] == 0 and out["off"] == [0, "function"] and out["phone"] == [0, "undefined"] and out["draw"][0] == 0
    assert out["hover"] == 1


def test_shell_has_no_second_ketcher_policy():
    s = (WEB / "shell.js").read_text(encoding="utf-8")
    assert "downlink" not in s and "prefetchKetcher" not in s and "arrayBuffer" not in s.split("function prefetchIdle")[0][-600:]
    assert "MZLAB_WARM_KETCHER" in s
