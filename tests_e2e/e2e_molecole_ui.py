"""Window «Molecole» (molecole-ui.js, opened by the «Manage» link) and the search box (molecole-campo.js), on INVENTED molecules only (Esempina, Provolone B): lazy loading, open and close
with the keyboard, add / edit / delete, a catalogue molecule edited as a copy of the user, import of CSV and JSON with one error per wrong row, export CSV and JSON, the molecules
surviving a reload (IndexedDB store «molecole»), the migration of the database 3 -> 4 without losing files / kv / liste_utente / manifest, the ARIA combobox with the keyboard, axe.
Needs the built site (tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_molecole_ui.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

AXE = axe_path()
OPEN = "carica('m:molecole-ui.js').then(() => window.MOLDB.apri())"   # the window opens from the «Manage» link of a search box; here directly
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

r = Run(port=8888, wd="/tmp/wd_molecole_ui")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#drop", timeout=120000)
        pg.reload(); pg.wait_for_function('window.qqReady===true', timeout=60000)
        ROWS = "#mol-body tr"
        def names(): return pg.locator("#mol-body th").all_inner_texts()
        def fill(**kw):
            for k, v in kw.items(): pg.fill("#mol-f-" + k, v)
        def add(**kw):
            pg.click("#mol-add"); fill(**kw); pg.click("#mol-save"); pg.wait_for_selector("#mol-form", state="hidden")
        def form_error(text):
            pg.wait_for_function("document.querySelector('#mol-ferr').textContent.includes(%s)" % json.dumps(text), timeout=10000)
        def search(q):
            pg.fill("#mol-q input", q)
            pg.wait_for_function("document.querySelector('#mol-body').dataset.q === %s" % json.dumps(q.strip() if len(q.strip()) >= 2 else ""), timeout=10000)
        def axe_ok(what):
            if not AXE: return
            if not pg.evaluate("typeof axe") == "object": pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run(document.querySelector('#moldlg'), {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']}}).then(r => r.violations.map(v => v.id + ': ' + v.nodes.map(n => n.target.join(' ')).join(' ; ')))")
            assert not v, (what, v)

        def lazy():
            assert pg.locator("#mol-btn").count() == 0, "the header has no Molecules button"
            assert pg.evaluate("typeof window.MOLDB") == "undefined", "the window is lazy: not loaded before the first click"
        step("no button in the header; window not loaded at start", lazy)

        def opens():
            pg.evaluate(OPEN); pg.wait_for_selector("#moldlg .mol-box", state="visible")
            assert pg.get_attribute("#moldlg .mol-box", "role") == "dialog" and pg.get_attribute("#moldlg .mol-box", "aria-modal") == "true"
            assert pg.inner_text("#mol-h") == "Molecole"
            pg.wait_for_function("document.activeElement && document.activeElement.closest('#mol-q')", timeout=10000)
            assert pg.inner_text("#mol-cap").startswith("Le tue molecole: 0") and "Non hai ancora" in pg.inner_text("#mol-st")
            pg.keyboard.press("Escape"); pg.wait_for_selector("#moldlg", state="hidden")
        step("the window opens on the search box and Esc closes it", opens)

        def tab_trap():
            pg.evaluate(OPEN); pg.wait_for_selector("#moldlg .mol-box", state="visible")
            for _ in range(14):
                pg.keyboard.press("Tab")
                assert pg.evaluate("!!document.activeElement.closest('#moldlg')"), "Tab left the window"
            pg.keyboard.press("Shift+Tab"); assert pg.evaluate("!!document.activeElement.closest('#moldlg')")
        step("Tab and Shift+Tab stay inside the window", tab_trap)

        def adds():
            add(nome="Esempina", sinonimi="Ex-1\nEsempinum", formula="C10H15NO", smiles="CC(N)Cc1ccccc1O", note="invented")
            assert names() == ["Esempina"], names()
            row = pg.locator(ROWS).first
            assert "C10H15NO" in row.inner_text() and "165.1154" in row.inner_text() and "Tua" in row.inner_text(), row.inner_text()
            assert "Salvata: Esempina" in pg.inner_text("#mol-st")
            add(nome="Provolone B", formula="c12h17no2", carica="0", addotti="[M+H]+ [M+Na]+", raccolte="corso")
            assert names() == ["Esempina", "Provolone B"], names()
            assert "C12H17NO2" in pg.locator(ROWS).nth(1).inner_text(), "the formula is written in Hill order by the engine"
        step("add two molecules: the engine writes the formula and the mass, the level is «Tua»", adds)

        def wrong_form():
            pg.click("#mol-add"); fill(nome="Errorina", formula="Xx9"); pg.click("#mol-save")
            form_error("formula molecolare non si legge"); assert pg.is_visible("#mol-form")
            fill(formula="C2H6", carica="1.5"); pg.click("#mol-save")
            form_error("numero intero")
            fill(nome="", formula="C2H6", carica=""); pg.click("#mol-save")
            form_error("Manca il nome")
            pg.keyboard.press("Escape"); pg.wait_for_selector("#mol-form", state="hidden")
            assert pg.is_visible("#mol-list") and pg.is_visible("#moldlg"), "Esc in the form goes back to the list"
        step("a wrong form shows the error of the engine and keeps the data; Esc goes back to the list", wrong_form)

        def duplicate_key():
            add(nome="Esempina bis", formula="C10H15NO", inchikey="AAAAAAAAAAAAAA-BBBBBBBBBB-C")
            pg.click("#mol-add"); fill(nome="Copia", formula="C10H15NO", inchikey="AAAAAAAAAAAAAA-BBBBBBBBBB-C"); pg.click("#mol-save")
            form_error("stesso id")
            pg.keyboard.press("Escape")
            assert "Esempina bis" in names()
        step("the same InChIKey twice is refused with a message", duplicate_key)

        def searches():
            search("provolone"); assert names() == ["Provolone B"], names()
            search("C10H15NO"); assert "Esempina" in names(), names()
            search("207.1259"); assert "Provolone B" in names(), names()
            search(""); assert len(names()) == 3
        step("search by name, formula and mass finds the molecules of the user", searches)

        def edit_own():
            search(""); pg.locator(ROWS, has_text="Provolone B").get_by_role("button", name="Modifica Provolone B").click()
            pg.wait_for_selector("#mol-form", state="visible")
            assert pg.input_value("#mol-f-nome") == "Provolone B" and pg.input_value("#mol-f-addotti") == "[M+H]+ [M+Na]+"
            assert pg.evaluate("document.activeElement.id") == "mol-f-nome" and not pg.is_visible("#mol-fn")
            fill(nome="Provolone C", note="renamed"); pg.click("#mol-save"); pg.wait_for_selector("#mol-form", state="hidden")
            assert "Provolone C" in names() and "Provolone B" not in names()
            search("provolone"); assert names() == ["Provolone C"], "the id stays, the name changes"
        step("edit one of the user: the name changes, nothing else is lost", edit_own)

        def edit_catalogue():
            search("paracetamol"); assert names() and "Catalogo" in pg.locator(ROWS).first.inner_text()
            assert pg.locator(ROWS).first.get_by_role("button", name="Elimina", exact=False).count() == 0, "a catalogue molecule cannot be deleted"
            nm = names()[0]
            pg.locator(ROWS).first.get_by_role("button", name="Modifica " + nm).click()
            pg.wait_for_selector("#mol-form", state="visible"); assert pg.is_visible("#mol-fn")
            fill(note="my note"); pg.click("#mol-save"); pg.wait_for_selector("#mol-form", state="hidden")
            search(""); assert nm in names(), "the copy of the user is in «Le tue molecole»"
            row = pg.locator(ROWS, has_text=nm).first
            assert "Tua" in row.inner_text()
            row.get_by_role("button", name="Elimina " + nm).click()
            pg.wait_for_selector("#mol-conf", state="visible")
            assert pg.evaluate("document.activeElement.id") == "mol-conf-no", "the safe answer has the focus"
            pg.click("#mol-conf-ok"); pg.wait_for_selector("#mol-conf", state="hidden")
            assert nm not in names()
            search(nm[:6]); assert nm in names() and "Catalogo" in pg.locator(ROWS, has_text=nm).first.inner_text(), "the catalogue one is back"
            search("")
        step("a catalogue molecule is edited as a copy of the user; deleting the copy brings the catalogue one back", edit_catalogue)

        def deletes():
            pg.locator(ROWS, has_text="Esempina bis").get_by_role("button", name="Elimina Esempina bis").click()
            pg.keyboard.press("Escape"); assert pg.is_visible("#moldlg") and not pg.is_visible("#mol-conf"), "Esc cancels the question only"
            assert "Esempina bis" in names()
            pg.locator(ROWS, has_text="Esempina bis").get_by_role("button", name="Elimina Esempina bis").click()
            pg.click("#mol-conf-ok"); pg.wait_for_selector("#mol-conf", state="hidden")
            assert "Esempina bis" not in names() and "Eliminata: Esempina bis" in pg.inner_text("#mol-st")
        step("delete asks first; Esc cancels the question", deletes)

        def persists():
            pg.keyboard.press("Escape"); pg.reload(); pg.wait_for_function('window.qqReady===true', timeout=60000)
            pg.evaluate(OPEN); pg.wait_for_selector("#mol-body tr")
            assert sorted(names()) == ["Esempina", "Provolone C"], names()
            v = pg.evaluate("""async () => { const dbs = await indexedDB.databases(); const d = dbs.find(x => x.name === 'mzlab_lab');
              const db = await new Promise(ok => { const q = indexedDB.open('mzlab_lab'); q.onsuccess = () => ok(q.result); });
              const out = { v: d.version, stores: [...db.objectStoreNames].sort(), n: await new Promise(ok => { const c = db.transaction('molecole').objectStore('molecole').count(); c.onsuccess = () => ok(c.result); }) }; db.close(); return out; }""")
            assert v["v"] == 4 and "molecole" in v["stores"] and v["n"] == 2, v
        step("after a reload the molecules are still there (IndexedDB version 4, store molecole)", persists)

        def exports():
            with pg.expect_download() as d: pg.click("#mol-csv")
            path = d.value.path(); text = open(path, encoding="utf-8").read()
            assert d.value.suggested_filename == "mzlab-molecules.csv" and text.splitlines()[0].startswith("id;name;synonyms;molecular_formula") and "Esempina" in text and "Provolone C" in text, text[:300]
            with pg.expect_download() as d: pg.click("#mol-json")
            doc = json.load(open(d.value.path(), encoding="utf-8"))
            assert doc["schema"] == "molecules/1" and [m["nome"] for m in doc["molecole"]].count("Esempina") == 1 and len(doc["molecole"]) == 2, doc
            r.csv_text, r.json_text = text, json.dumps(doc)
        step("export CSV and JSON: the files of the engine (canonical)", exports)

        def import_bad():
            bad = "name;molecular_formula\nOk uno;CH4\nBrutta;Xx9\nSenza;\n"
            pg.set_input_files("#mol-file", files=[{"name": "bad.csv", "mimeType": "text/csv", "buffer": bad.encode()}])
            pg.wait_for_selector("#mol-errs", state="visible")
            txt = pg.inner_text("#mol-errs"); assert "2 righe da correggere" in txt and "Riga 3:" in txt and "Riga 4:" in txt and "Riga 2" not in txt, txt
            assert "Ok uno" not in names(), "a wrong file imports nothing"
            pg.set_input_files("#mol-file", files=[{"name": "bad.json", "mimeType": "application/json", "buffer": b"{not json"}])
            pg.wait_for_function("document.querySelector('#mol-errs').textContent.includes('non è un documento')", timeout=10000)
        step("import of a wrong file: one error per row, nothing imported", import_bad)

        def import_good():
            ok = "name;molecular_formula;synonyms;inchikey\nOk uno;CH4O;Metanolino;\nImportina;C2H6O;;AAAAAAAAAAAAAA-CCCCCCCCCC-D\n"
            put = lambda name, text, mime="text/csv": pg.set_input_files("#mol-file", files=[{"name": name, "mimeType": mime, "buffer": text.encode()}])
            def status(txt): pg.wait_for_function("document.querySelector('#mol-st').textContent.includes(%s)" % json.dumps(txt), timeout=10000)
            put("good.csv", ok); status("2 molecole importate; sostituite: 0")
            assert not pg.is_visible("#mol-errs")
            assert "Ok uno" in names() and "Importata" in pg.locator(ROWS, has_text="Ok uno").first.inner_text()
            put("good.csv", ok); status("sostituite: 2; tue, lasciate come sono: 0")
            pg.locator(ROWS, has_text="Importina").get_by_role("button", name="Modifica Importina").click()
            pg.wait_for_selector("#mol-form", state="visible"); fill(nome="Importina X"); pg.click("#mol-save"); pg.wait_for_selector("#mol-form", state="hidden")
            assert "Tua" in pg.locator(ROWS, has_text="Importina X").first.inner_text()
            put("good.csv", ok); status("sostituite: 1; tue, lasciate come sono: 1")
            assert "Importina X" in names() and "Importina" not in names(), names()
            doc = {"schema": "molecules/1", "molecole": [{"nome": "Json uno", "formula": "C2H6O"}]}
            put("uno.json", json.dumps(doc), "application/json")
            pg.wait_for_function("document.querySelector('#mol-body').textContent.includes('Json uno')", timeout=10000)
            assert "Importata" in pg.locator(ROWS, has_text="Json uno").first.inner_text()
        step("import of a CSV and of a JSON: level «Importata»; a molecule edited by the user is kept when the file comes again", import_good)

        def round_trip():
            for n in ["Ok uno", "Json uno", "Importina X", "Esempina", "Provolone C"]:
                pg.locator(ROWS, has_text=n).first.get_by_role("button", name="Elimina " + n).click(); pg.click("#mol-conf-ok"); pg.wait_for_selector("#mol-conf", state="hidden")
            assert pg.locator(ROWS).count() == 0
            pg.set_input_files("#mol-file", files=[{"name": "back.csv", "mimeType": "text/csv", "buffer": r.csv_text.encode()}])
            pg.wait_for_function("document.querySelector('#mol-body').textContent.includes('Esempina')", timeout=10000)
            with pg.expect_download() as d: pg.click("#mol-json")
            back = json.load(open(d.value.path(), encoding="utf-8")); was = json.loads(r.json_text)
            strip = lambda ms: sorted((m["id"], m["nome"], m["formula"], round(m["massa"], 4)) for m in ms)
            assert strip(back["molecole"]) == strip(was["molecole"]), (back, was)
        step("deleting all and importing the exported CSV gives back the same molecules", round_trip)

        def axe_window():
            search("")
            for theme in ("light", "dark"):
                pg.evaluate("document.documentElement.setAttribute('data-theme','%s')" % theme)
                axe_ok(theme + " list")
                pg.click("#mol-add"); axe_ok(theme + " form"); pg.keyboard.press("Escape")
            pg.evaluate("document.documentElement.setAttribute('data-theme','light')")
        step("axe on the window (list and form)" + ("" if AXE else " [SKIP: axe-core not available]"), axe_window)
        pg.keyboard.press("Escape")

        def combobox():
            pg.evaluate("""() => { const h = document.createElement('div'); h.id = 'molc-host'; h.style.cssText = 'position:fixed;left:20px;top:300px;width:420px;z-index:5000;background:var(--panel)'; document.body.appendChild(h);
              window.__picked = []; window.__box = MOLDB.campo(h, { testi: { label: 'Molecule', placeholder: 'Name, formula or mass', none: 'No molecule found', count: n => n + ' found', manage: 'Manage' }, onScegli: m => window.__picked.push(m.nome) }); }""")
            i = "#molc-host input"
            assert pg.get_attribute(i, "role") == "combobox" and pg.get_attribute(i, "aria-expanded") == "false" and pg.get_attribute(i, "aria-autocomplete") == "list"
            assert pg.inner_text("#molc-host label") == "Molecule" and pg.get_attribute(i, "placeholder") == "Name, formula or mass"
            pg.click(i); pg.keyboard.type("esemp")
            pg.wait_for_selector("#molc-host [role=listbox]", state="visible")
            assert pg.get_attribute(i, "aria-expanded") == "true" and pg.get_attribute(i, "aria-controls") == pg.get_attribute("#molc-host [role=listbox]", "id")
            assert pg.locator("#molc-host [role=option]").count() == 1 and pg.inner_text("#molc-host [role=status]") == "1 found", pg.locator("#molc-host [role=option]").all_inner_texts()
            assert pg.get_attribute("#molc-host [role=listbox]", "aria-labelledby") == pg.get_attribute("#molc-host label", "id")
            pg.keyboard.press("ArrowDown")
            act = pg.get_attribute(i, "aria-activedescendant"); assert act and pg.get_attribute("#" + act, "aria-selected") == "true"
            pg.keyboard.press("Escape"); assert pg.get_attribute(i, "aria-expanded") == "false" and pg.evaluate("document.activeElement.closest('#molc-host') !== null")
            pg.keyboard.press("ArrowDown"); pg.wait_for_selector("#molc-host [role=listbox]", state="visible")
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter")
            assert pg.evaluate("window.__picked") == ["Esempina"] and pg.input_value(i) == "Esempina" and pg.get_attribute(i, "aria-expanded") == "false"
            pg.fill(i, "zzzzqqqq"); pg.wait_for_function("document.querySelector('#molc-host [role=status]').textContent === 'No molecule found'", timeout=10000)
            assert pg.get_attribute(i, "aria-expanded") == "false"
            pg.fill(i, "165.1154"); pg.wait_for_selector("#molc-host [role=option]", timeout=10000)
            pg.locator("#molc-host [role=option]", has_text="Esempina").first.click(); assert pg.evaluate("window.__picked").count("Esempina") == 2, pg.evaluate("window.__picked")
        step("search box: ARIA combobox, texts from the caller, ArrowDown / Enter / Esc, name and mass", combobox)

        def combobox_manage():
            assert pg.inner_text("#molc-host .molc-man") == "Manage"
            pg.click("#molc-host .molc-man"); pg.wait_for_selector("#moldlg .mol-box", state="visible")
            pg.keyboard.press("Escape"); pg.wait_for_selector("#moldlg", state="hidden")
            axe_ok_box = AXE and pg.evaluate("axe.run(document.querySelector('#molc-host')).then(r => r.violations.map(v => v.id))")
            assert not axe_ok_box, axe_ok_box
        step("the «Manage» link opens the window; axe on the search box", combobox_manage)

        def migration():
            res = pg.evaluate("""async () => {
              const { upgrade, DB_VER } = await import('./static/qqq-db.js');
              const name = 'mzlab_migra_prova';
              const open = (ver, up) => new Promise((ok, no) => { const q = indexedDB.open(name, ver); q.onupgradeneeded = () => up(q.result); q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); });
              const v3 = await open(3, db => { db.createObjectStore('files'); db.createObjectStore('kv'); db.createObjectStore('liste_utente', { keyPath: 'id' }); db.createObjectStore('manifest', { keyPath: 'path' }); });
              await new Promise(ok => { const t = v3.transaction(['files', 'kv', 'liste_utente', 'manifest'], 'readwrite');
                t.objectStore('files').put('bytes', 'a.mzML'); t.objectStore('kv').put('{"session":[]}', 'notebook'); t.objectStore('liste_utente').put({ id: 'l1', name: 'My list' }); t.objectStore('manifest').put({ path: 'files/a.mzML', bytes: 5 }); t.oncomplete = ok; });
              v3.close();
              const v4 = await open(DB_VER, upgrade);
              const get = (s, k) => new Promise(ok => { const q = v4.transaction(s).objectStore(s).get(k); q.onsuccess = () => ok(q.result); });
              const out = { ver: v4.version, stores: [...v4.objectStoreNames].sort(), files: await get('files', 'a.mzML'), kv: await get('kv', 'notebook'), lst: await get('liste_utente', 'l1'), man: await get('manifest', 'files/a.mzML'),
                key: v4.transaction('molecole').objectStore('molecole').keyPath };
              v4.close(); indexedDB.deleteDatabase(name); return out; }""")
            assert res["ver"] == 4 and res["stores"] == ["files", "kv", "liste_utente", "manifest", "molecole"] and res["key"] == "id", res
            assert res["files"] == "bytes" and res["kv"] == '{"session":[]}' and res["lst"] == {"id": "l1", "name": "My list"} and res["man"] == {"path": "files/a.mzML", "bytes": 5}, res
        step("database 3 -> 4: the new store is added, files / kv / liste_utente / manifest keep their rows", migration)

        def clean():
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
            bad = [e for e in r.errs if e[0] in ("pageerror", "console.error") or e[0].startswith("http")]
            assert not bad, bad[:3]
        step("MZLAB_LR_MISS empty, no page error", clean)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:400]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s.startswith("FAIL") for _, s in steps) else 0)
