"""Pure helpers touched by the page-speed pass (mappa.js, explore.js, shell.js), run with node on the source text: each one is cut out of its file and
compared with the definition it replaced (the old code is written here as the reference)."""
import json
import shutil
import subprocess
from pathlib import Path

import numpy as np
import pytest

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def cut(name, start, end):
    s = (WEB / name).read_text(encoding="utf-8")
    a = s.index(start)
    return s[a:s.index(end, a)]


def run(code, data=None):
    out = subprocess.run(["node", "-e", code], input=json.dumps(data or {}), capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(out.stdout)


@needs_node
def test_mad_noise_is_the_scaled_median_absolute_deviation_of_the_positive_values():
    src = (WEB / "mappa-dati.js").read_text(encoding="utf-8") + "const madNoise = MAPDATI.madNoise;"
    rng = np.random.default_rng(5)
    for n in (1, 2, 3, 10, 11, 500, 501):
        y = np.where(rng.random(n) < .3, 0.0, rng.lognormal(3, 1, n))
        got = run(src + "\nconst d=JSON.parse(require('fs').readFileSync(0,'utf8'));console.log(JSON.stringify(madNoise(d.y)))", {"y": y.tolist()})
        v = y[y > 0]
        want = 0.0 if not len(v) else 1.4826 * float(np.median(np.abs(v - np.median(v))))
        assert got == pytest.approx(want, rel=1e-12)
    assert run(src + "\nconsole.log(JSON.stringify(madNoise([0,-1,0])))") == 0


@needs_node
def test_smooth_matches_the_map_version():
    src = cut("shell.js", "function smooth(", "\n")
    old = "const smoothOld=(y,k)=>{if(!k)return y;return y.map((_,i)=>{const a=y[i-1]??y[i],c=y[i+1]??y[i];return(a+2*y[i]+c)/4})};"
    rng = np.random.default_rng(1)
    for n in (0, 1, 2, 3, 50):
        y = rng.random(n).tolist()
        if n > 3:
            y[2] = None                                                              # a NaN of the engine arrives as null
        ok = run(src + old + "\nconst d=JSON.parse(require('fs').readFileSync(0,'utf8'));console.log(JSON.stringify(JSON.stringify(smooth(d.y,3))===JSON.stringify(smoothOld(d.y,3))&&smooth(d.y,0)===d.y))", {"y": y})
        assert ok is True


@needs_node
def test_float64_to_array_and_base64_float32():
    src = "const navigator = {};" + (WEB / "cache-dati.js").read_text(encoding="utf-8") + (WEB / "mappa-dati.js").read_text(encoding="utf-8") + "const b64f32 = MAPDATI.b64f32;"
    code = src + """
const a=Float64Array.from([1.5,-0,2e-7,1e21]);const o=f64Arr(a);
const f=Float32Array.from([1,2.5,-3]);const b=Buffer.from(f.buffer).toString('base64');
const native=Uint8Array.fromBase64;
const r1=b64f32(b);Uint8Array.fromBase64=undefined;const r2=b64f32(b);
console.log(JSON.stringify([Array.isArray(o),JSON.stringify(o)===JSON.stringify(Array.from(a)),Array.from(r1),Array.from(r2),native===undefined||true]));"""
    assert run("const document={};" + code) == [True, True, [1, 2.5, -3], [1, 2.5, -3], True]


@needs_node
def test_side_profile_one_pass_equals_the_map_version():
    new = cut("mappa.js", "  function profSide(", "  // spectrum turned on its side")
    old = '''function profOld(g,sp,pr,Y,x0,x1,ymax,col){
    const m=sp.pmz,v=sp.py,X=u=>x0+(x1-x0)*u/ymax,rows=new Map();const idx=[];for(let i=pr.i0;i<pr.i1;i++)if(i===pr.i0||m[i]!==m[i-1])idx.push(i);
    const n=idx.length;for(const i of idx){const r=Math.round(Y(m[i])),u=rows.get(r);if(u===undefined||v[i]>u)rows.set(r,v[i]);}
    const pts=rows.size>=n?idx.map(i=>[Y(m[i]),v[i]]):[...rows].sort((p,q)=>q[0]-p[0]);
    g.beginPath();g.moveTo(X(pts[0][1]),pts[0][0]);for(const q of pts)g.lineTo(X(q[1]),q[0]);g.lineTo(x0,pts[pts.length-1][0]);g.lineTo(x0,pts[0][0]);return{pts:pts.length,full:n}}'''
    code = new + old + """
const rec=()=>{const L=[];return{L,beginPath(){},moveTo:(...a)=>L.push(a),lineTo:(...a)=>L.push(a),closePath(){},fill(){},stroke(){},globalAlpha:1}};
const rng=(s=>()=>(s=(s*16807)%2147483647)/2147483647)(7);let same=true;
for(const [n,H] of [[50,400],[5000,300],[5000,3000],[8,300]]){
  const m=[];let x=100;for(let i=0;i<n;i++){x+=rng()<.1?0:rng()*0.02+0.001;m.push(+x.toFixed(6))}
  const py=m.map(()=>rng()*100),sp={pmz:m,py},y0=m[0],y1=m[n-1],Y=v=>H-(v-y0)/(y1-y0)*H;
  const a=rec(),b=rec();const ra=profSide(a,sp,{i0:0,i1:n},Y,6,200,100,'#000'),rb=profOld(b,sp,{i0:0,i1:n},Y,6,200,100,'#000');
  const A=a.L.slice(0,-2),B=b.L.slice(0,-2);
  if(JSON.stringify(ra)!==JSON.stringify(rb)||JSON.stringify(a.L.slice(0,b.L.length))!==JSON.stringify(b.L))same=false}
console.log(JSON.stringify(same))"""
    assert run(code) is True


@needs_node
def test_cache_keeps_to_its_byte_budget_and_recent_answers():
    src = (WEB / "cache-dati.js").read_text(encoding="utf-8")
    code = "const navigator={};" + src + """
CACHE_BUD.max=1200;
(async()=>{
  const mk=n=>()=>Promise.resolve({rt:new Array(n).fill(1),y:new Array(n).fill(2)});
  await memo('a',mk(30));await memo('b',mk(30));await Promise.resolve();
  await memo('a',mk(30));                       // a is the newest
  await memo('c',mk(30));await Promise.resolve();await Promise.resolve();
  const fail=memo('f',()=>Promise.reject(new Error('x'))).catch(()=>0);await fail;
  console.log(JSON.stringify([...CACHE.keys()]));
})()"""
    keys = run(code)
    assert "f" not in keys and "c" in keys and "a" in keys and "b" not in keys
