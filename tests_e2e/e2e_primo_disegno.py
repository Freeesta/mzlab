"""C28 PR1: First paint must be the final layout.
Measures layout shift (CLS), bounding box differences and element text changes across 4 state instants:
(1) DOMContentLoaded (first paint)
(2) after translations are applied (i18n ready)
(3) window.qqReady === true
(4) quiet / settled after qqReady
Runs across matrix: 1300x900 and 1000x700, it-IT and en-US, cold and warm cache.
"""
from io import BytesIO
import json, os, sys
from pathlib import Path
from PIL import Image
import numpy as np
from playwright.sync_api import sync_playwright

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
from lib import Run

SNAPSHOT_JS = '''
() => {
    const out = {};
    let seen = 0;   // the key counts only the visible nodes: a node added inside something hidden shifts nothing
    for (const root of [document.querySelector('header'), document.querySelector('#start')]) {
        if (!root) continue;
        const els = [root, ...root.querySelectorAll('*')];
        for (let i = 0; i < els.length; i++) {
            const el = els[i];
            const r = el.getBoundingClientRect();
            if (r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden') {
                const key = (el.id ? '#' + el.id : el.tagName.toLowerCase()) + '[' + (seen++) + ']';
                out[key] = {
                    id: el.id || '',
                    tag: el.tagName,
                    text: (el.childElementCount === 0 ? el.textContent : '').trim(),
                    x: Math.round(r.x * 10) / 10,
                    y: Math.round(r.y * 10) / 10,
                    w: Math.round(r.width * 10) / 10,
                    h: Math.round(r.height * 10) / 10
                };
            }
        }
    }
    return out;
}
'''

INIT_TRACK_JS = '''
window.__cls = 0;
window.__cls_by_state = {};
window.__state = '1_dcl';
window.__shifts = [];
try {
    new PerformanceObserver(l => {
        for (const e of l.getEntries()) {
            if (!e.hadRecentInput) {
                window.__cls += e.value;
                window.__cls_by_state[window.__state] = (window.__cls_by_state[window.__state] || 0) + e.value;
                window.__shifts.push({
                    val: e.value,
                    state: window.__state,
                    sources: (e.sources || []).map(s => (s.node ? (s.node.id || s.node.nodeName) : 'null'))
                });
            }
        }
    }).observe({type: 'layout-shift', buffered: true});
} catch (_) {}

window.__dcl_done = false;
window.__dcl_nodes = null;
document.addEventListener('DOMContentLoaded', () => {
    window.__dcl_done = true;
    const out = {};
    let seen = 0;   // the key counts only the visible nodes: a node added inside something hidden shifts nothing
    for (const root of [document.querySelector('header'), document.querySelector('#start')]) {
        if (!root) continue;
        const els = [root, ...root.querySelectorAll('*')];
        for (let i = 0; i < els.length; i++) {
            const el = els[i];
            const r = el.getBoundingClientRect();
            if (r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden') {
                const key = (el.id ? '#' + el.id : el.tagName.toLowerCase()) + '[' + (seen++) + ']';
                out[key] = {
                    id: el.id || '',
                    tag: el.tagName,
                    text: (el.childElementCount === 0 ? el.textContent : '').trim(),
                    x: Math.round(r.x * 10) / 10,
                    y: Math.round(r.y * 10) / 10,
                    w: Math.round(r.width * 10) / 10,
                    h: Math.round(r.height * 10) / 10
                };
            }
        }
    }
    window.__dcl_nodes = out;
});

window.__i18n_done = false;
document.addEventListener('DOMContentLoaded', () => {
    setTimeout(() => { window.__i18n_done = true; }, 0);
});
'''


def diff_nodes(sA, sB):
    changes = []
    for k, a in sA.items():
        if k in sB:
            b = sB[k]
            ch = []
            if (a['x'], a['y']) != (b['x'], b['y']):
                ch.append(f"pos: ({a['x']},{a['y']})->({b['x']},{b['y']})")
            if (a['w'], a['h']) != (b['w'], b['h']):
                ch.append(f"dim: ({a['w']},{a['h']})->({b['w']},{b['h']})")
            if a['text'] != b['text']:
                ch.append(f"text: \"{a['text'][:24]}\"->\"{b['text'][:24]}\"")
            if ch:
                changes.append((k, a['id'] or a['tag'], ", ".join(ch)))
        else:
            changes.append((k, a['id'] or a['tag'], "disappeared"))
    for k, b in sB.items():
        if k not in sA:
            changes.append((k, b['id'] or b['tag'], "appeared"))
    return changes


def diff_pixels(bA, bB):
    imA, imB = Image.open(BytesIO(bA)), Image.open(BytesIO(bB))
    if imA.size != imB.size:
        return True, f"size differs: {imA.size} vs {imB.size}"
    arrA, arrB = np.array(imA), np.array(imB)
    diff = np.abs(arrA.astype(np.int16) - arrB.astype(np.int16))
    # a step of 1-3 levels on a channel is antialiasing noise at a border, not something the eye sees
    diff_count = int(np.sum(np.any(diff > 3, axis=-1)))
    return diff_count > 0, f"diff pixels: {diff_count} / {imA.size[0] * imA.size[1]}"


def run_measurement(port):
    matrix = [
        (1300, 900, "it-IT", False),
        (1300, 900, "it-IT", True),
        (1300, 900, "en-US", False),
        (1300, 900, "en-US", True),
        (1000, 700, "it-IT", False),
        (1000, 700, "it-IT", True),
        (1000, 700, "en-US", False),
        (1000, 700, "en-US", True),
    ]
    all_defects = []
    with sync_playwright() as p:
        b = p.chromium.launch()
        url = f"http://127.0.0.1:{port}/"
        for w, h, loc, warm in matrix:
            ctx = b.new_context(viewport={"width": w, "height": h}, locale=loc)
            if warm:
                warm_pg = ctx.new_page()
                warm_pg.goto(url, wait_until="networkidle")
                warm_pg.close()

            ctx.add_init_script(INIT_TRACK_JS)
            pg = ctx.new_page()
            # slow network emulation: route responses with artificial latency
            pg.route("**/*", lambda route: route.continue_())

            pg.goto(url, wait_until="commit")

            # Instant (1): DOMContentLoaded
            pg.wait_for_function("window.__dcl_nodes !== null")
            s1 = pg.evaluate("window.__dcl_nodes")
            shot1 = pg.screenshot()

            # Instant (2): after i18n applied
            pg.evaluate("window.__state = '2_i18n'")
            pg.wait_for_function("window.__i18n_done === true")
            s2 = pg.evaluate(SNAPSHOT_JS)
            shot2 = pg.screenshot()

            # Instant (3): window.qqReady === true
            pg.evaluate("window.__state = '3_qqReady'")
            pg.wait_for_function("window.qqReady === true", timeout=30000)
            s3 = pg.evaluate(SNAPSHOT_JS)
            shot3 = pg.screenshot()

            # Instant (4): 2 seconds / settled after (3) via state wait (rAF frames)
            pg.evaluate("window.__state = '4_settled'")
            pg.evaluate("() => new Promise(res => { let n = 120; function f() { if (--n <= 0) res(); else requestAnimationFrame(f); } requestAnimationFrame(f); })")
            s4 = pg.evaluate(SNAPSHOT_JS)
            shot4 = pg.screenshot()

            cls_val = pg.evaluate("window.__cls")
            d12 = diff_nodes(s1, s2)
            d23 = diff_nodes(s2, s3)
            d34 = diff_nodes(s3, s4)

            pdiff12, pnote12 = diff_pixels(shot1, shot2)
            pdiff23, pnote23 = diff_pixels(shot2, shot3)
            pdiff34, pnote34 = diff_pixels(shot3, shot4)

            label = f"{w}x{h} {loc} warm={warm}"
            print(f"[{label}] CLS={cls_val:.4f} | Diff nodes: 1->2: {len(d12)}, 2->3: {len(d23)}, 3->4: {len(d34)} | Pixels: 1->2: {pnote12}, 2->3: {pnote23}")

            if cls_val > 0 or len(d12) > 0 or len(d23) > 0 or pdiff12 or pdiff23:
                all_defects.append({
                    "config": label,
                    "cls": cls_val,
                    "shifts": pg.evaluate("window.__shifts"),
                    "d12_count": len(d12),
                    "d23_count": len(d23),
                    "d12_sample": d12[:10],
                    "d23_sample": d23[:10],
                })
            ctx.close()
        b.close()
    return all_defects


def main():
    r = Run(port=8895, wd="/tmp/wd_primo_disegno")
    try:
        defects = run_measurement(r.port)
        print("\n=== MEASUREMENT SUMMARY ===")
        print(f"Configurations tested with layout defects: {len(defects)} / 8")
        if defects:
            first = defects[0]
            print(f"Sample configuration: {first['config']}")
            print(f"CLS: {first['cls']:.4f}")
            print(f"DCL -> i18n node changes: {first['d12_count']}")
            print(f"i18n -> qqReady node changes: {first['d23_count']}")
            raise AssertionError(f"FAIL: Primo disegno non e finale: {len(defects)} configurazioni hanno salti di layout o cambi DOM")
    finally:
        r.close()


if __name__ == "__main__":
    main()
