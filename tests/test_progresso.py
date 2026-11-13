"""The model of progress of every loading (mzlab/web/progresso.js, run by node): for each kind of reader the bar goes forward only and is not 100 % before the end."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "progresso.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

# what each reader tells the page (event mzlab-index {name, frac, phase}, frac = 0..1 of the whole file); the sequences go back and repeat on purpose: the builders read a file more than once
READERS = {
    "mzml": [(None, f) for f in (0, 0.05, 0.3, 0.6, 0.45, 0.9, 0.9, 1.0)],
    "raw": [("read", 0.0), ("read", 0.12), ("read", 0.25), ("scans", 0.3), ("scans", 0.6), ("scans", 0.35), ("chrom", 0.7), ("chrom", 0.95), ("chrom", 1.0)],
    "wiff": [("read", 0.25), ("scans", 0.4), ("scans", 0.6), ("chrom", 0.7), ("scans", 0.5), ("chrom", 1.0)],
    "mzidx": [],                                                                    # the index is already there: nothing to tell, the file just ends
    "d_folder": [("convert", 0.0), ("convert", 0.5), ("convert", 1.0), ("index", 0.0), ("index", 0.7), ("index", 0.2), ("index", 1.0)],
}


def run(code):
    out = subprocess.run(["node", "-e", f"const m=require({str(JS)!r});const {{PROGRESSO:P,progCompose:C,rowPc:R}}=m;{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def series(kind, n_files=1):
    """pc after every step of `n_files` files of the same reader, interleaved; and the steps at which a file ended"""
    return run(f"""
const seq={json.dumps(READERS[kind])},prof={json.dumps('conversione' if kind == 'd_folder' else '')},out=[];
P.reset();for(let i=0;i<{n_files};i++)P.begin('f'+i,(i+1)*1000);out.push(['begin',P.state().pc]);
const L=Math.max(seq.length,1);
for(let s=0;s<L;s++)for(let i=0;i<{n_files};i++){{ if(seq.length){{const [ph,fr]=seq[s];P.set('f'+i,prof==='conversione'?C(prof,ph,fr):fr,ph||undefined);out.push(['set',P.state().pc,P.state().over]);}} }}
for(let i=0;i<{n_files};i++){{P.end('f'+i);const st=P.state();out.push(['end',st.pc,st.over]);}}
console.log(JSON.stringify(out));""")


@pytest.mark.parametrize("kind", list(READERS))
@pytest.mark.parametrize("n", [1, 3])
def test_forward_only_and_not_complete_before_the_end(kind, n):
    out = series(kind, n)
    pcs = [o[1] for o in out]
    assert pcs == sorted(pcs), (kind, pcs)                                          # never backwards
    assert all(o[1] <= 99 and not o[2] for o in out if o[0] == "set"), (kind, out)      # not 100 % while a file still runs
    ends = [o for o in out if o[0] == "end"]
    assert [o[1] for o in ends[:-1]] == sorted(o[1] for o in ends[:-1]) and all(o[1] <= 99 for o in ends[:-1]), (kind, ends)      # nor after the first ones end
    assert ends[-1][1] == 100 and ends[-1][2], (kind, ends)                        # the last end gives 100


def test_a_file_added_in_the_middle_does_not_pull_the_bar_back():
    out = run("""
P.reset();const o=[];P.begin('a',1000);P.set('a',0.9,'scans');o.push(P.state().pc);
P.begin('b',9000);o.push(P.state().pc);P.set('b',0.1,'read');o.push(P.state().pc);P.end('a');o.push(P.state().pc);P.set('b',0.95);o.push(P.state().pc);P.end('b');o.push(P.state().pc);
console.log(JSON.stringify(o));""")
    assert out == sorted(out) and out[-1] == 100 and max(out[:-1]) <= 99, out


def test_a_failed_file_ends_too_and_the_batch_closes_at_100():
    out = run("""P.reset();P.begin('a',10);P.begin('b',10);P.set('a',0.5);P.end('a');P.end('b');console.log(JSON.stringify([P.state().pc,P.state().over,P.busy()]))""")
    assert out == [100, True, False]


def test_a_new_batch_starts_at_zero_after_the_last_one_ended():
    out = run("""P.reset();P.begin('a',10);P.end('a');const x=P.state().pc;P.begin('b',10);console.log(JSON.stringify([x,P.state().pc,P.busy()]))""")
    assert out == [100, 0, True]


def test_the_phase_shown_is_the_one_of_the_slowest_file():
    out = run("""P.reset();P.begin('a',10);P.begin('b',10);P.set('a',0.8,'chrom');P.set('b',0.2,'read');console.log(JSON.stringify(P.state().phase))""")
    assert out == "read"


def test_compose_uses_the_weights_and_clamps():
    out = run("""console.log(JSON.stringify([C('conversione','convert',0),C('conversione','convert',1),C('conversione','index',0),C('conversione','index',1),C('conversione','index',2),C('conversione','convert',0.5),C('x','y',0.4)]))""")
    assert out == [0, 0.75, 0.75, 1, 1, 0.375, 0.4]


def test_fuzz_never_back_never_complete():
    out = run("""
let seed=7;const rnd=()=>(seed=(seed*1103515245+12345)%2147483648)/2147483648;
P.reset();const names=[];let last=0,bad=[];
for(let step=0;step<400;step++){
  const r=rnd();
  if(r<0.15||!names.length){const n='f'+step;names.push(n);P.begin(n,1+Math.floor(rnd()*1e6));}
  else if(r<0.2){const n=names[Math.floor(rnd()*names.length)];if(names.filter(x=>P.file(x)<1).length>1)P.end(n);}
  else{const n=names[Math.floor(rnd()*names.length)];P.set(n,rnd()*1.3-0.1,['read','scans','chrom','index',undefined][Math.floor(rnd()*5)]);}
  const s=P.state();if(s.pc<last)bad.push(['back',step,last,s.pc]);if(s.pc===100&&!s.over)bad.push(['early',step]);last=s.pc;
}
console.log(JSON.stringify(bad));""")
    assert out == []


def test_the_bar_of_a_row_has_fixed_shares_for_a_makers_file_and_the_model_for_an_mzml():
    # a maker's file: load 0-40 %, read 40-85 %, index 85-100 % by the model's 0.3 and 0.75; an mzML: the model's fraction as is (mzmlFrac weighs its stages); forward only in both
    got = run("console.log(JSON.stringify([[0,.3,.525,.75,1].map(f=>R(true,f)),[0,.25,.5,1].map(f=>R(false,f)),R(true,-1),R(false,5),R(true,null)]))")
    assert got[0] == [0, 40, 62.5, 85, 100] and got[1] == [0, 25, 50, 100] and got[2:] == [0, 100, 0]
    for native in ("true", "false"):
        v = run(f"console.log(JSON.stringify(Array.from({{length:101}},(_,i)=>R({native},i/100))))")
        assert v == sorted(v) and v[0] == 0 and v[-1] == 100


def mzml(code):
    return json.loads(subprocess.run(["node", "-e", f"const {{mzmlFrac:F,MZML_STAGES:S}}=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout)


def test_the_stages_of_an_mzml_weigh_their_cost_and_only_go_forward():
    # every stage starts where the one before ends, and the whole runs forward from 0 to 1, for a small and a large file
    for size in (160e3, 5e6, 76e6, 2e9):
        v = mzml(f"const o=[];for(const k of S)for(let i=0;i<=10;i++)o.push(F({size},k,i/10));console.log(JSON.stringify(o))")
        assert v == sorted(v) and v[0] == 0 and abs(v[-1] - 1) < 1e-9, size
    # a small file: fixed costs (the start of the worker, the listing) weigh most, no rounds; a large one: the copy and the builder (reading, then its rounds) weigh most
    small = mzml("console.log(JSON.stringify(S.map(k=>F(160e3,k,1)-F(160e3,k,0))))")
    big = mzml("console.log(JSON.stringify(S.map(k=>F(76e6,k,1)-F(76e6,k,0))))")
    st = dict(zip(["load", "start", "scan", "read", "index", "list"], small)); bg = dict(zip(st, big))
    assert st["index"] == 0 and st["start"] > st["load"] and st["list"] > 0.1
    assert bg["index"] > 0.3 and bg["read"] > 0.3 and bg["load"] > 0.15 and bg["start"] < 0.02
    # an unknown stage (a queued notice) counts as nothing, a local fraction out of 0..1 is clamped
    assert mzml("console.log(JSON.stringify([F(1e6,'?',1),F(1e6,undefined,1),F(1e6,'load',-1),F(1e6,'list',9)]))") == [0, 0, 0, 1]
