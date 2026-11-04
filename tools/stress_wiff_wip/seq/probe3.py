from common import *
port=free_port(); s=serve(port); errs=[]
try:
  with sync_playwright() as p:
    b=p.chromium.launch(); c,pg=newpage(b,errs,port)
    A=os.environ["STRESS_WIFF_DIR"].rstrip("/")+"/"
    pg.set_input_files("#pick",[A+"MS2.wiff",A+"MS2.wiff.scan"])
    attendi(pg,"UPBUSY===0 && OPENNOW.busy()===0",120000); pg.wait_for_timeout(2000)
    for i in range(2):
      r=pg.evaluate("""()=>{const q=E.panels[%d];q.cv.focus();return [q.type,document.activeElement===q.cv,q.cv.tagName,q.cv.tabIndex,q.cv.id]}"""%i)
      pg.keyboard.press("t"); pg.wait_for_timeout(500)
      print(r, pg.evaluate("[!!document.getElementById('tdati'), (document.querySelector('[aria-live]')||{}).innerText]"))
      pg.evaluate("(()=>{const d=document.getElementById('tdati');if(d){d.close();d.remove()}})()")
    b.close()
finally: s.terminate()
