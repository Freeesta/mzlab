from common import *
port=free_port(); s=serve(port); errs=[]
try:
  with sync_playwright() as p:
    b=p.chromium.launch(); c,pg=newpage(b,errs,port)
    A=os.environ["STRESS_WIFF_DIR"].rstrip("/")+"/"
    for f in sys.argv[1:]:
        t=time.time()
        pg.set_input_files("#pick",[A+f+".wiff",A+f+".wiff.scan"])
        try: attendi(pg,"ST.files.length>0 && ST.files.every(f=>f.kind)",120000)
        except Exception as e: print("timeout",e)
        print(f, round(time.time()-t,1), pg.evaluate("ST.files.map(f=>[f.name,f.kind,f.mixed&&f.mixed.length])"))
        try: pg.evaluate("loaded"); 
        except: pass
        print(pg.evaluate("typeof OPENNOW!=='undefined' && OPENNOW.busy()"))
    pg.wait_for_timeout(3000)
    print(pg.evaluate("[E.files.map(f=>[f.file,f.type,f.lv,f.vis,f.gone]), E.panels.map(q=>q.type)]"))
    print(pg.evaluate("Array.from(document.querySelectorAll('#flist tr')).map(t=>t.innerText.replace(/\\s+/g,' ')).slice(0,15)"))
    print(pg.evaluate("Array.from(document.querySelectorAll('button[id^=np-],#samptab,#opbtn')).map(b=>b.id+':'+b.offsetParent)"))
    print(errs, pg.evaluate("window.MZLAB_LR_MISS"))
    b.close()
finally: s.terminate()
