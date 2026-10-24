"""Search box of the one Tabui table (TP_Mine/js/42_tp_table.js): the text of a row is searched once per burst of typing (not once per key), the shown rows are the same
as without the cache, an edit of the rows between two bursts is seen, and a chip or an order change never uses a stale text. Loads only the component in a blank page.
Run from the repo root: python3 tests_e2e/e2e_tpmine_ricerca.py"""
import sys
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

SETUP = """() => {
  const rows = Array.from({ length: 2000 }, (_, i) => ({ id: i, name: 'TP' + i, formula: 'C' + (i % 40) + 'H' + (i % 70), mz: 100 + i / 37 }));
  window.__calls = 0; window.__rows = rows;
  window.__t = window.TPTAB.mount(document.getElementById('h'), { id: 'b', esc: s => String(s), rowId: r => r.id, name: r => r.name, onOpen() {}, virtual: 300,
    cols: [{ key: 'n', label: 'Name', sort: r => r.name, cell: r => r.name }, { key: 'm', label: 'm/z', num: true, sort: r => r.mz, cell: r => r.mz.toFixed(3) }],
    chips: [{ key: 'big', label: 'Big', test: r => r.mz > 130 }],
    search: r => { window.__calls++; return [r.name, r.formula].join(' '); } });
  window.__t.setRows(rows);
}"""
TYPE = """(txt) => { const q = document.querySelector('.tpt-q'); let s = ''; for (const ch of txt) { s += ch; q.value = s; q.dispatchEvent(new Event('input')); } return window.__t.view.map(r => r.id); }"""

with sync_playwright() as p:
    b = p.chromium.launch(); pg = b.new_page(); errs = []
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.set_content('<div class="tp"><div id="h"></div></div>')
    pg.add_script_tag(path=str(ROOT / "TP_Mine" / "js" / "42_tp_table.js"))
    pg.evaluate(SETUP)

    def once_per_burst():
        pg.evaluate("() => { window.__calls = 0 }")
        ids = pg.evaluate(TYPE, "C12H3")
        calls = pg.evaluate("() => window.__calls")
        assert calls <= 2000, f"search called {calls} times for 2000 rows"
        want = pg.evaluate("() => window.__rows.filter(r => (r.name + ' ' + r.formula).toLowerCase().includes('c12h3')).map(r => r.id)")
        assert sorted(ids) == sorted(want) and ids, f"{len(ids)} shown, {len(want)} expected"
    step("one search call per row for a whole burst, same rows as a plain filter", once_per_burst)

    def edit_seen():
        pg.evaluate("() => { window.__rows[5].name = 'ZZtarget'; window.__t.setRows(window.__rows) }")
        ids = pg.evaluate(TYPE, "zztarget")
        assert ids == [5], ids
    step("an edited row is found after setRows", edit_seen)

    def chip_fresh():
        pg.evaluate(TYPE, "tp")
        pg.evaluate("() => { window.__rows[1500].name = 'QQ7'; window.__t.setChip('big', true) }")
        pg.evaluate(TYPE, "qq7")
        n = pg.evaluate("() => window.__t.view.length")
        assert n == 1, n
        pg.evaluate("() => window.__t.setChip('big', false)")
    step("a chip click drops the typing cache", chip_fresh)

    def sort_and_clear():
        pg.evaluate(TYPE, "tp1")
        pg.evaluate("() => window.__t.sortBy('m', -1)")
        v = pg.evaluate("() => window.__t.view.map(r => r.mz)")
        assert v == sorted(v, reverse=True) and len(v) > 1
        pg.evaluate("() => { const q = document.querySelector('.tpt-q'); q.value = ''; q.dispatchEvent(new Event('input')) }")
        n = pg.evaluate("() => window.__t.view.length")
        assert n == 2000, n
    step("sorting keeps the filter, an empty search shows all rows", sort_and_clear)

    def order_like_localecompare():
        names = ["éclair", "Zeta", "alpha", "Alpha", "beta", None, "ångström", "alpha", "10", "9", None, "zeta"]
        got = pg.evaluate("""(names) => {
          const rows = names.map((n, i) => ({ id: i, name: n, mz: n === null ? null : n.length }));
          document.getElementById('h').innerHTML = '';
          const t = window.TPTAB.mount(document.getElementById('h'), { id: 'o', esc: s => String(s), rowId: r => r.id, name: r => String(r.name), onOpen() {},
            cols: [{ key: 'n', label: 'Name', sort: r => r.name, cell: r => String(r.name) }, { key: 'm', label: 'Len', num: true, sort: r => r.mz, cell: r => String(r.mz) }] });
          t.setRows(rows); const out = {};
          for (const [k, d] of [['n', 1], ['n', -1], ['m', 1], ['m', -1]]) {
            t.sortBy(k, d); out[k + d] = t.view.map(r => r.id);
            const f = k === 'n' ? r => r.name : r => r.mz, ref = rows.map((r, i) => ({ r, v: f(r), i })).sort((x, y) => {
              if (x.v == null || y.v == null) return x.v == null && y.v == null ? x.i - y.i : x.v == null ? 1 : -1;
              const c = typeof x.v === 'string' ? x.v.localeCompare(y.v) : x.v - y.v; return c ? c * d : x.i - y.i; }).map(x => x.r.id);
            out[k + d + 'ref'] = ref;
          }
          return out;
        }""", names)
        for k in ("n1", "n-1", "m1", "m-1"):
            assert got[k] == got[k + "ref"], f"{k}: {got[k]} != {got[k + 'ref']}"
    step("sort order equals localeCompare: accents, case, empty last, stable ties, both directions", order_like_localecompare)
    def filter_keeps_order():
        res = pg.evaluate("""() => {
          document.getElementById('h').innerHTML = '';
          const rows = Array.from({ length: 500 }, (_, i) => ({ id: i, name: 'n' + ((i * 37) % 101), mz: (i * 53) % 17 }));
          const t = window.TPTAB.mount(document.getElementById('h'), { id: 'f', esc: s => String(s), rowId: r => r.id, name: r => r.name, onOpen() {},
            cols: [{ key: 'n', label: 'Name', sort: r => r.name, cell: r => r.name }, { key: 'm', label: 'mz', num: true, sort: r => r.mz, cell: r => String(r.mz) }],
            chips: [{ key: 'hi', label: 'Hi', test: r => r.mz > 8 }], search: r => r.name });
          t.setRows(rows); t.sortBy('m', -1);
          const q = document.querySelector('.tpt-q'), ids = () => t.view.map(r => r.id);
          const want = () => rows.filter(r => (!t.chips.includes('hi') || r.mz > 8) && r.name.includes(q.value)).map((r, i) => ({ r, i })).sort((a, b) => b.r.mz - a.r.mz || a.i - b.i).map(x => x.r.id);
          const out = [];
          t.setChip('hi', true); out.push([ids(), want()]);
          q.value = 'n1'; q.dispatchEvent(new Event('input')); out.push([ids(), want()]);
          t.setChip('hi', false); out.push([ids(), want()]);
          rows[3].mz = 99; t.setRows(rows); out.push([ids(), want()]);       // an edited sort value is read again after setRows
          t.setChip('hi', true); rows[10].mz = 98; t.setChip('hi', false); t.setChip('hi', true); out.push([ids(), want()]);   // edited in place, no setRows: the order is read again
          t.setChip('hi', false); t.sortBy('n', 1); q.value = ''; q.dispatchEvent(new Event('input'));
          return out.map(([a, b]) => JSON.stringify(a) === JSON.stringify(b)).concat([ids()[0] !== undefined]);
        }""")
        assert all(res), res
    step("a chip or the search on a sorted table equals sorting the filtered rows again", filter_keeps_order)
    b.close()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
