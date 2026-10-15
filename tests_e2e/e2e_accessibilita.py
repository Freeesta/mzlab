"""«Lettura facilitata» of Teoria and Pratica: Aa panel, fonts, WCAG spacing, backgrounds (contrast), ruler that never takes clicks, concentration mode,
reduced motion (simulations start paused), screen readers (landmarks, names, canvas labels), keyboard, high-contrast figures, persistence, Ripristina, and the Teoria inside the app (#tframe)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import json
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_accessibilita.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
LUM = "(c=>{const m=c.match(/[0-9.]+/g).map(Number),f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])})"
def cr(pg, a, b):
    return pg.evaluate(f"(()=>{{const L={LUM};const x=L({a}),y=L({b});return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)}})()")
STUB = """window.__said=[];(function(){const s=window.speechSynthesis;if(!s)return;s.speak=u=>{window.__said.push({t:u.text,r:u.rate,l:u.lang});setTimeout(()=>{u.onstart&&u.onstart();u.onend&&u.onend()},10)};s.cancel=()=>{}})();"""
r = Run(port=8970, wd="/tmp/wd_a11y")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        base = f"http://127.0.0.1:{r.port}/static/teoria/"
        def fresh(name="06-esi.html", scheme="light", motion="no-preference", prefs=None):
            q = r.b.new_page(viewport={"width": 1400, "height": 1000}, color_scheme=scheme, reduced_motion=motion)
            q.add_init_script(STUB)
            if prefs is not None: q.add_init_script("localStorage.setItem('qqq.a11y', %s)" % json.dumps(json.dumps(prefs)))
            q.goto(base + name); q.wait_for_timeout(900); return q
        def attr(q, n): return q.evaluate(f"document.documentElement.getAttribute('data-a11y-{n}')")
        def panel():
            q = fresh(); assert q.get_attribute("#a11y-btn", "aria-label") == "Impostazioni"
            assert q.is_hidden("#a11y-panel"); q.click("#a11y-btn"); assert q.is_visible("#a11y-panel")
            assert q.get_attribute("#a11y-panel", "role") == "dialog"
            q.screenshot(path=SH + "a11y_pannello.png")
            q.keyboard.press("Escape"); assert q.is_hidden("#a11y-panel")
            q.keyboard.press("Tab") ; q.focus("#a11y-btn"); q.keyboard.press("Enter"); assert q.is_visible("#a11y-panel")          # keyboard only
            txt = q.inner_text("#a11y-panel"); assert "⌘+ / Ctrl+" not in txt and "Dimensione" not in txt, txt[:200]
            assert q.evaluate("localStorage.getItem('qqq.prefs')") is None            # the old key is not touched
            q.close()
        step("Aa button, side panel, Esc, keyboard, no old text-size", panel)
        def fonts():
            q = fresh(); q.click("#a11y-btn")
            q.check("input[name=a11y-font][value=atkinson]"); attendi(q, "document.fonts.check('16px \"Atkinson Hyperlegible Next\"')", 60000)
            assert "Atkinson" in q.evaluate("getComputedStyle(document.body).fontFamily") and attr(q, "font") == "atkinson"
            assert q.evaluate("document.fonts.check('16px \"Atkinson Hyperlegible Next\"')"), "font not loaded"
            q.check("input[name=a11y-font][value=dyslexic]"); attendi(q, "document.fonts.check('16px OpenDyslexic')", 60000)
            assert "OpenDyslexic" in q.evaluate("getComputedStyle(document.body).fontFamily")
            assert q.evaluate("document.fonts.check('16px OpenDyslexic')"), "OpenDyslexic not loaded"
            q.close()
            q = fresh()                                   # fonts are downloaded only when chosen
            assert not q.evaluate("[...document.fonts].some(f=>f.status==='loaded')")
            q.close()
        step("Atkinson and OpenDyslexic apply and load; nothing downloaded otherwise", fonts)
        def space():
            q = fresh(); q.click("#a11y-btn"); q.check("input[data-k=space]")
            v = q.evaluate("(()=>{const p=document.querySelector('main p'),c=getComputedStyle(p),fs=parseFloat(c.fontSize);return {ls:parseFloat(c.letterSpacing)/fs,ws:parseFloat(c.wordSpacing)/fs,lh:parseFloat(c.lineHeight)/fs,mb:parseFloat(c.marginBottom)/fs,ta:c.textAlign,w:p.getBoundingClientRect().width,ch:fs*0.5*65}})()")
            assert abs(v["ls"] - .12) < .005 and abs(v["ws"] - .16) < .005 and 1.6 <= v["lh"] <= 1.8 and abs(v["mb"] - 2) < .05 and v["ta"] in ("left", "start"), v
            assert v["w"] <= 65 * 16 * 1.0, v                     # at most 65ch (the width of a character is about .5-.6 em)
            assert q.evaluate("getComputedStyle(document.querySelector('main svg text')||document.body).letterSpacing") in ("normal", "0px")
            q.close()
        step("spacing: 0.12em / 0.16em / line 1.7 / 2em after paragraphs / 65ch", space)
        def backgrounds():
            for bg, need in (("cream", 4.5), ("blue", 4.5), ("hc", 7)):
                for scheme in ("light", "dark"):
                    q = fresh(scheme=scheme, prefs={"bg": bg}); assert attr(q, "bg") == bg
                    ink = cr(q, "getComputedStyle(document.body).color", "getComputedStyle(document.body).backgroundColor")
                    mut = cr(q, "getComputedStyle(document.querySelector('.lead')||document.querySelector('main p')).color", "getComputedStyle(document.body).backgroundColor")
                    acc = cr(q, "getComputedStyle(document.querySelector('main a')).color", "getComputedStyle(document.body).backgroundColor")
                    assert ink >= need and acc >= need, (bg, scheme, ink, acc)
                    if bg == "cream" and scheme == "dark": assert q.evaluate("getComputedStyle(document.body).backgroundColor") == "rgb(30, 27, 22)"
                    if (bg, scheme) in (("cream", "light"), ("hc", "dark")): q.screenshot(path=SH + f"a11y_{bg}_{scheme}.png")
                    q.close()
        step("backgrounds cream / blue / high contrast, light and dark: contrast 4.5 (7)", backgrounds)
        def persist():
            q = fresh(); q.click("#a11y-btn"); q.check("input[name=a11y-bg][value=blue]"); q.check("input[data-k=nums]")
            assert json.loads(q.evaluate("localStorage.getItem('qqq.a11y')"))["bg"] == "blue"
            q.reload(); q.wait_for_timeout(700)
            assert attr(q, "bg") == "blue" and attr(q, "nums") is not None
            q.click("#a11y-btn"); assert q.is_checked("input[name=a11y-bg][value=blue]") and q.is_checked("input[data-k=nums]")
            q.close()
        step("persistence after reload (key qqq.a11y)", persist)
        def ruler():
            q = fresh(); q.click("#a11y-btn"); q.check("input[data-k=ruler]"); q.keyboard.press("Escape")
            assert q.evaluate("getComputedStyle(document.getElementById('a11y-ruler')).pointerEvents") == "none"
            assert q.evaluate("getComputedStyle(document.getElementById('a11y-ruler')).display") == "block"
            q.mouse.move(700, 400); q.wait_for_timeout(100); q.screenshot(path=SH + "a11y_righello.png")
            assert q.evaluate("document.getElementById('a11y-ruler').style.getPropertyValue('--ry')") not in ("", "40%")
            q.click("#side ol a >> nth=1"); q.wait_for_timeout(400)            # a click through the ruler still works
            assert q.evaluate("location.pathname") != ""
            q.close()
        step("reading ruler follows the mouse and never blocks clicks", ruler)
        def indice():
            q = fresh(); assert q.is_visible("#side") and q.get_attribute("#tc-indice", "aria-pressed") == "true"
            assert q.locator("#a11y-prog,[data-k=focus]").count() == 0
            q.click("#tc-indice"); assert attr(q, "noside") is not None and q.is_hidden("#side") and q.get_attribute("#tc-indice", "aria-pressed") == "false"
            assert json.loads(q.evaluate("localStorage.getItem('qqq.a11y')")).get("noside") is True
            q.reload(); q.wait_for_selector("#tc-indice"); assert q.is_hidden("#side"), "remembered"
            q.keyboard.press("f"); assert q.is_visible("#side") and attr(q, "noside") is None; q.close()
        step("index toggle (button and F): hides/shows the side index, remembered", indice)
        def motion():
            q = fresh("09-quadrupolo.html", motion="reduce"); assert attr(q, "reduce") is not None
            q.evaluate("document.querySelector('#sim-field').scrollIntoView()"); q.wait_for_timeout(600)
            assert q.inner_text("#sim-field [data-k=play]") == "Riprendi", q.inner_text("#sim-field [data-k=play]")
            a = q.evaluate("document.querySelector('#sim-field canvas').toDataURL()"); q.wait_for_timeout(500)
            assert a == q.evaluate("document.querySelector('#sim-field canvas').toDataURL()"), "still moving"
            assert q.evaluate("getComputedStyle(document.documentElement).scrollBehavior") == "auto"
            q.click("#sim-field [data-k=play]"); q.wait_for_timeout(600)
            assert a != q.evaluate("document.querySelector('#sim-field canvas').toDataURL()"), "does not start"
            q.close()
            q = fresh("10-qqq.html", motion="reduce"); q.evaluate("document.querySelector('#sim-modi,.sim canvas').scrollIntoView()") if False else None
            q.wait_for_timeout(500)
            sel = ".sim:has(canvas) button:has-text('Avvia')"
            q.evaluate("document.querySelectorAll('.sim canvas')[0].scrollIntoView()"); q.wait_for_timeout(500); assert q.locator(sel).count() >= 1
            q.close()
            q = fresh("09-quadrupolo.html", motion="no-preference")                     # automatic: nothing changes without the system request
            assert attr(q, "reduce") is None; q.close()
            q = fresh("09-quadrupolo.html", motion="no-preference", prefs={"motion": "always"}); assert attr(q, "reduce") is not None; q.close()
        step("reduced motion (system or «Sempre»): simulations start paused", motion)
        def speech():
            q = fresh()
            assert q.locator(".a11y-say").count() == 0
            q.click("#a11y-btn"); q.wait_for_selector("#a11y-panel:not([hidden])")
            assert q.locator("#a11y-tts").count() == 0
            q.close()
        step("speech synthesis removed: no ▶ and no TTS panel", speech)
        def reset():
            q = fresh(prefs={"font": "atkinson", "space": True, "bg": "cream", "ruler": True, "noside": True, "nums": True})
            q.click("#a11y-btn"); q.click("#a11y-reset"); q.wait_for_timeout(200)
            assert all(attr(q, n) is None for n in ("font", "space", "bg", "ruler", "noside", "nums")), q.evaluate("document.documentElement.outerHTML.slice(0,300)")
            assert json.loads(q.evaluate("localStorage.getItem('qqq.a11y')")) == {}
            q.close()
        step("Ripristina predefinite turns everything off", reset)
        def pratica():
            q = fresh("pratica.html"); assert q.is_visible("#a11y-btn"); q.close()
            q = fresh(); q.set_viewport_size({"width": 400, "height": 800}); q.reload(); q.wait_for_timeout(600); assert q.is_visible("#a11y-btn"); q.close()
        step("Aa is on Pratica and on a narrow screen", pratica)
        def inapp():
            pg.evaluate("localStorage.setItem('qqq.a11y', JSON.stringify({bg:'cream',space:true}))")
            pg.click("#nav button[data-v=theory]")
            attendi(pg, "(() => { const f = document.querySelector('#tframe'); try { return f.contentDocument.documentElement.getAttribute('data-a11y-bg') === 'cream' } catch (e) { return false } })()", 60000)
            f = next(x for x in pg.frames if "/teoria/" in x.url)
            assert f.evaluate("document.documentElement.getAttribute('data-a11y-bg')") == "cream" and f.evaluate("document.documentElement.hasAttribute('data-a11y-space')")
            assert f.is_visible("#a11y-btn"); f.click("#a11y-btn"); f.check("input[name=a11y-bg][value=blue]")
            assert f.evaluate("document.documentElement.getAttribute('data-a11y-bg')") == "blue"
            pg.screenshot(path=SH + "a11y_nell_app.png")
        step("the Teoria inside the app (#tframe) applies and changes the preferences", inapp)
        def pages():
            q = fresh("index.html")
            pgs = q.evaluate("CHAPTERS.map(c=>c[0])"); q.close()
            assert len(pgs) >= 30, pgs
            for name in pgs:
                q = fresh(name)
                v = q.evaluate("""(()=>{const hs=[...document.querySelectorAll('main h1,main h2,main h3,main h4')].map(h=>+(h.getAttribute('aria-level')||h.tagName[1]));let bad=0;
                  for(let i=1;i<hs.length;i++) if(hs[i]>hs[i-1]+1) bad++;
                  return {lang:document.documentElement.lang,h1:document.querySelectorAll('h1').length,main:document.querySelectorAll('main').length,bad,
                    nav:[...document.querySelectorAll('nav')].every(n=>n.getAttribute('aria-label')),skip:!!document.querySelector('a.skip'),cur:document.querySelectorAll('#side a[aria-current=page]').length}})()""")
                assert v["lang"] == "it" and v["h1"] == 1 and v["main"] == 1 and v["nav"] and v["skip"] and v["cur"] == 1, (name, v)
                assert v["bad"] == 0, (name, "heading levels skip", v)
                q.close()
        step("every page: lang, one h1, headings in order, main, named nav, skip link", pages)
        def skip():
            q = fresh("04-lc.html"); q.keyboard.press("Tab")
            assert q.evaluate("document.activeElement.className") == "skip" and q.is_visible("a.skip")
            q.keyboard.press("Enter"); q.wait_for_timeout(100)
            assert q.evaluate("document.activeElement.tagName") == "MAIN", q.evaluate("document.activeElement.tagName")
            q.close()
        step("«Salta al contenuto»: first Tab, Enter moves the focus into main", skip)
        def names():
            for name in ("index.html", "09-quadrupolo.html", "10-qqq.html", "pratica.html", "pratica-ei.html", "pratica-formula.html", "pratica-quadrupolo.html"):
                q = fresh(name); q.click("#a11y-btn")
                snap = q.locator("body").aria_snapshot()
                bad = [l.strip() for l in snap.splitlines() if __import__("re").match(r"^\s*- (button|link|combobox|slider|textbox|checkbox|radio|spinbutton)( \[[^\]]*\])*:?$", l)]
                assert not bad, (name, bad[:5])
                n = q.evaluate("document.querySelectorAll('canvas').length"); assert n == q.evaluate("document.querySelectorAll('canvas[role=img][aria-label],canvas[role=group][aria-label]').length"), (name, "canvas without role/label")
                q.close()
        step("accessible names: no unnamed button, link, field; every canvas role+label", names)
        def canvases():
            q = fresh("09-quadrupolo.html")
            labs = q.evaluate("[...document.querySelectorAll('.sim canvas')].map(c=>c.getAttribute('aria-label'))")
            assert labs and all(l.startswith("Grafico: ") for l in labs) and sum("Asse orizzontale" in l and "Asse verticale" in l for l in labs) >= 2, labs
            q.close()
            q = fresh("06-esi.html"); q.evaluate("document.querySelector('#sim-drop').scrollIntoView()"); q.wait_for_timeout(300)
            assert q.get_attribute("#sim-drop output", "aria-live") == "polite"
            assert q.get_attribute("#sim-drop .btns button >> nth=0", "aria-pressed") in ("true", "false") if q.locator("#sim-drop .btns button").count() else True
            q.close()
        step("canvases say what they show (title + axes); values in a polite live region", canvases)
        def tour(name, limit=400):
            q = fresh(name); seen = set(); last = None; n = 0; vis = 0
            for _ in range(limit):
                q.keyboard.press("Tab"); n += 1
                k = q.evaluate("""(()=>{const e=document.activeElement;if(!e||e===document.body)return null;const c=getComputedStyle(e);
                  return {id:e.tagName+'|'+(e.id||e.textContent||'').slice(0,40)+'|'+[...document.querySelectorAll(e.tagName)].indexOf(e),o:c.outlineStyle!=='none'&&parseFloat(c.outlineWidth)>0||c.boxShadow!=='none'||c.borderBottomStyle==='dotted'||e.matches('.g')}})()""")
                if k is None: break                               # the focus left the page: no trap
                if k["id"] in seen: raise AssertionError(f"{name}: keyboard trap at {k['id']}")
                seen.add(k["id"]); vis += 1 if k["o"] else 0
            assert len(seen) >= 8, (name, len(seen))
            assert vis >= len(seen) - 2, (name, "focus not visible", vis, len(seen))
            q.close()
        step("keyboard tour of a Teoria page: all reachable, visible focus, no trap", lambda: tour("09-quadrupolo.html"))
        def gioco():
            q = fresh("pratica-ei.html"); q.wait_for_timeout(500)
            assert q.locator("#game canvas[role=group]").count() == 1
            q.focus("#game canvas"); q.keyboard.press("ArrowRight"); q.wait_for_timeout(150)
            t1 = q.inner_text("#game .fb.hi"); assert "Selezionato m/z" in t1, t1
            q.keyboard.press("Shift+ArrowRight"); q.wait_for_timeout(150)
            assert "Δm" in q.inner_text("#game .fb.hi")
            assert q.get_attribute("#game .fb.hi", "aria-live") == "polite"
            assert "frecce" in q.get_attribute("#game canvas", "aria-label")
            for _ in range(40):                                               # leave the spectrum: the focus never gets stuck
                q.keyboard.press("Tab")
            assert q.evaluate("document.activeElement.tagName") != "CANVAS"
            q.close()
        step("game «Dallo spettro alla struttura»: pick peaks and Δm with the arrows, answer announced", gioco)
        def hc():
            for scheme, need in (("light", 7), ("dark", 7)):
                q = fresh("09-quadrupolo.html", scheme=scheme, prefs={"bg": "hc"})
                bg = "getComputedStyle(document.body).backgroundColor"
                for c in ("#24231f", "#9b978c", "#6b675c", "#2b5c8a", "#c2410c", "#b42318", "#0e7490", "#b45309"):
                    got = q.evaluate(f"(()=>{{const x=document.createElement('canvas').getContext('2d');x.strokeStyle='{c}';return x.strokeStyle}})()")
                    ratio = cr(q, f"'{got}'" if got.startswith("rgb") else "(()=>{const d=document.createElement('i');d.style.color='%s';document.body.appendChild(d);const v=getComputedStyle(d).color;d.remove();return v})()" % got, bg)
                    assert ratio >= 3, (scheme, c, got, ratio)
                    if c in ("#24231f", "#9b978c", "#6b675c"): assert ratio >= need, (scheme, c, got, ratio)
                for i in range(1, 7):
                    assert cr(q, "(()=>{const d=document.createElement('i');d.style.color='var(--c%d)';document.body.appendChild(d);const v=getComputedStyle(d).color;d.remove();return v})()" % i, bg) >= 3, (scheme, i)
                q.close()
        step("figures in «Alto contrasto» (light and dark): axes, text, series >= 3:1 (axes and text 7:1)", hc)
        def hc_live():
            q = fresh("09-quadrupolo.html", prefs={}); a = q.evaluate("document.querySelector('.sim canvas').toDataURL()")
            q.click("#a11y-btn"); q.check("input[name=a11y-bg][value=hc]"); q.wait_for_timeout(500)
            assert a != q.evaluate("document.querySelector('.sim canvas').toDataURL()"), "figure not redrawn"
            q.close()
        step("choosing «Alto contrasto» redraws the figures", hc_live)
        def dashes():
            q = fresh("06-esi.html"); q.evaluate("document.querySelector('#sim-rayleigh').scrollIntoView()"); q.wait_for_timeout(300)
            assert q.evaluate("TP.DASH.length") == 4 and q.evaluate("TP.DASH[0]") is None
            q.close()
        step("series of the figures are told apart by dashes too", dashes)
        def figure_texts():
            for name in ("02-fotocatalisi.html", "03-cromatografia.html", "04-lc.html", "05-gc.html", "06-esi.html", "07-ei-ci.html", "09-quadrupolo.html", "10-qqq.html",
                         "14-frammentazione-esi.html", "15-ei-metodo.html", "18-strategia.html", "index.html", "20-disegno.html"):
                q = fresh(name)
                v = q.evaluate("""[...document.querySelectorAll('main svg[role=img],main img')].map(e=>{const a=(e.getAttribute('aria-label')||e.getAttribute('alt')||'');
                  const d=e.getAttribute('aria-describedby')&&document.getElementById(e.getAttribute('aria-describedby'));
                  return {alt:a.length,d:d?d.tagName+':'+d.querySelector('summary').textContent+':'+d.innerText.replace(d.querySelector('summary').textContent,'').trim().split(/\\s+/).length:null}})""")
                assert v, name
                for x in v:
                    assert 0 < x["alt"] <= 125, (name, x)
                    if x["d"]: t, sm, n = x["d"].split(":"); assert t == "DETAILS" and sm == "Descrizione della figura" and int(n) <= 85, (name, x)
                if name != "20-disegno.html": assert any(x["d"] for x in v), (name, "scheme without long description")
                q.close()
        step("figures: short alt (<= 125 characters) and long description in <details> (<= 85 words) linked by aria-describedby", figure_texts)
        def hc_svg():
            for scheme in ("light", "dark"):
                for name in ("06-esi.html", "09-quadrupolo.html", "10-qqq.html", "14-frammentazione-esi.html", "18-strategia.html", "index.html"):
                    q = fresh(name, scheme=scheme, prefs={"bg": "hc"})
                    bad = q.evaluate("""(()=>{const L=%s,bg=getComputedStyle(document.body).backgroundColor,B=L(bg),out=[];
                      const cr=c=>{const m=c.match(/[0-9.]+/g);if(!m||(m[3]!==undefined&&+m[3]===0)||c==='none')return null;const y=L(c);return (Math.max(B,y)+.05)/(Math.min(B,y)+.05)};
                      document.querySelectorAll('main svg[role=img] text').forEach(t=>{const r=cr(getComputedStyle(t).fill);if(r!==null&&r<7)out.push('text '+t.textContent.slice(0,20)+' '+r.toFixed(1))});
                      document.querySelectorAll('main svg[role=img] :is(line,path,polyline)').forEach(e=>{const s=getComputedStyle(e).stroke,r=cr(s);if(r!==null&&r<3)out.push('line '+s+' '+r.toFixed(1))});
                      return out.slice(0,5)})()""" % LUM)
                    assert not bad, (scheme, name, bad)
                    q.close()
        step("SVG figures in «Alto contrasto» (light and dark): text >= 7:1, lines >= 3:1", hc_svg)
        def zones():
            q = fresh("09-quadrupolo.html"); q.evaluate("document.querySelector('#sim-stab,#sim-mathieu,.sim:has(canvas)').scrollIntoView()")
            src = q.evaluate("[...document.scripts].map(s=>s.src).filter(s=>s.includes('sim-quad'))[0]")
            txt = q.evaluate("fetch('%s').then(r=>r.text())" % src)
            assert "a righe /" in txt and "a righe \\\\" in txt and "bandX" in txt
            q.close()
        step("stability zones of the quadrupole are hatched (not only coloured) and the legend says so", zones)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
