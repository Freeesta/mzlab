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
    out = subprocess.run(["node", "-e", f"const m=require({str(JS)!r});const {{PROGRESSO:P,progCompose:C,PROG_NET:N}}=m;{code}"], check=True, capture_output=True, encoding="utf-8").stdout
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
    out = run("""console.log(JSON.stringify([C('conversione','convert',0),C('conversione','convert',1),C('conversione','index',0),C('conversione','index',1),C('conversione','index',2),C('conversione','convert',0.5),C('x','y',0.4),N.send]))""")
    assert out == [0, 0.75, 0.75, 1, 1, 0.375, 0.4, 0.3]


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
