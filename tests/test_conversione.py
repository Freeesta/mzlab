"""Progress helpers of the .raw conversion (mzlab/web/conversione.js, run by node)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "conversione.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"const c=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_phase_and_percent_come_from_the_scans():
    # the bytes read are at 100% from the start (the tail of the file is read to open it): only the scans count
    cases = [({"total": 1000, "read": 1000, "written": 0, "scans": 0, "scansTotal": 0}, "reading", None), ({"total": 1000, "read": 1000, "written": 0, "scans": 0, "scansTotal": None}, "reading", None),
             ({"total": 1000, "read": 1000, "written": 0, "scans": 0, "scansTotal": 200}, "converting", 0),
             ({"total": 1000, "read": 1000, "written": 4000, "scans": 50, "scansTotal": 200}, "converting", 25), ({"total": 1000, "read": 1000, "written": 9000, "scans": 200, "scansTotal": 200}, "converting", 100),
             ({"total": 1, "read": 1, "written": 900, "copied": 300, "size": 900}, "converting", 33)]
    got = run(f"console.log(JSON.stringify({json.dumps([c for c, _, _ in cases])}.map(p=>[c.convPhase(p),c.convPercent(p)])))")
    assert got == [[ph, pc] for _, ph, pc in cases]
    assert run("console.log(JSON.stringify([c.convPhase(null),c.convPercent(null),c.convFraction(null)]))") == ["reading", None, 0]
    assert run("console.log(JSON.stringify([c.convFraction({scans:50,scansTotal:200}),c.convFraction({copied:1,size:2})]))") == [0.25, 1]


def test_total_percent_is_weighted_by_the_size_of_the_files():
    assert run("console.log(JSON.stringify([c.convTotalPercent([100,300],0,0.5),c.convTotalPercent([100,300],1,0.5),c.convTotalPercent([100,300],1,1),c.convTotalPercent([0],0,0)]))") == [12, 62, 100, None]


def test_elapsed_and_megabytes():
    assert run("console.log(JSON.stringify([0,999,1000,65000,3599000,3600000,7325000].map(c.convElapsed)))") == ["0:00", "0:00", "0:01", "1:05", "59:59", "1:00:00", "2:02:05"]
    assert run("console.log(JSON.stringify([c.convMB(1048576*3.14), c.convMB(1048576*250)]))") == ["3.1", "250"]


def test_converted_name_keeps_the_extension_when_taken():
    got = run("""console.log(JSON.stringify([c.convName("a.raw",[]), c.convName("a.RAW",["b.mzML"]), c.convName("a.raw",["A.mzml"]), c.convName("a.raw")]))""")
    assert got == ["a.mzML", "a.mzML", "a.raw.mzML", "a.mzML"]


def test_wiff_sample_name():
    got = run("""console.log(JSON.stringify([c.wiffName("FullMass",3,false,[]), c.wiffName("FullMass",3,true,[]), c.wiffName("FullMass",3,true,["fullmass_S3.mzml"]), c.wiffName("x",1,false,["X.mzML"])]))""")
    assert got == ["FullMass.mzML", "FullMass_s3.mzML", "FullMass_s3.wiff.mzML", "x.wiff.mzML"]


def test_wiff_sample_name_from_data():
    got = run("""console.log(JSON.stringify([
        c.wiffName("FullMass", 1, true, [], "Sample_A"),
        c.wiffName("FullMass", 1, true, [], "  Sample_A  "),
        c.wiffName("FullMass", 2, true, ["sample_a.mzml"], "Sample_A"),
        c.wiffName("FullMass", 2, true, ["sample_a.mzml", "sample_a.wiff.mzml"], "Sample_A"),
        c.wiffName("FullMass", 3, true, [], ""),
        c.wiffName("FullMass", 1, false, [], "A/B\\\\:C*D?E\\\"F<G>H|I"),
        c.wiffName("FullMass", 1, false, [], "x".repeat(150)),
        c.wiffName("one", 1, false, [], "one.wiff #1"),
        c.wiffName("two", 2, true, [], "two.wiff #2"),
        c.wiffName("U", 1, false, [], "U.WIFF #1"),
        c.wiffName("x", 1, false, [], "x.wiff #3")
    ]))""")
    assert got == [
        "Sample_A.mzML",
        "Sample_A.mzML",
        "Sample_A.wiff.mzML",
        "Sample_A_2.wiff.mzML",
        "FullMass_s3.mzML",
        "A_B__C_D_E_F_G_H_I.mzML",
        "x" * 120 + ".mzML",
        "one.mzML",
        "two_s2.mzML",
        "U.mzML",
        "x.mzML",
    ]


def test_the_rows_of_the_files_being_converted_have_a_text_for_each_state():
    # CONVROWS shows a sample in the list of the files from the pairing of its .wiff and .wiff.scan: one key per state, an unknown state is "queued"
    got = run("console.log(JSON.stringify(['open','wait','run','conv','index','x'].map(c.convRowKey)))")
    assert got == ["conv.row.open", "conv.row.wait", "conv.row.run", "conv.row.conv", "conv.row.index", "conv.row.wait"]


def test_third_same_named_file_gets_a_counter_instead_of_overwriting_the_second():
    got = run("""console.log(JSON.stringify([
        c.convName("blank.raw", ["blank.mzML", "blank.raw.mzML"]),
        c.convName("blank.raw", ["blank.mzML", "blank.raw.mzML", "blank_2.raw.mzML"]),
        c.wiffName("x", 1, false, ["x.mzML", "x.wiff.mzML"]),
        c.wiffName("x", 1, true, ["x_s1.mzML", "x_s1.wiff.mzML"]),
        c.wiffName("x", 2, true, ["s.mzml", "s.wiff.mzml", "s_2.wiff.mzML"], "S")
    ]))""")
    assert got == ["blank_2.raw.mzML", "blank_3.raw.mzML", "x_2.wiff.mzML", "x_s1_2.wiff.mzML", "S_3.wiff.mzML"]


def test_bar_repaint_does_not_lose_changes_of_the_second_window():
    # r1 flushes at once, r2 waits for the end of the first window, r3 arrives in the second window: it must be repainted at the end of that one
    got = run("""const R=c.CONVROWS, log=[], rows=[{name:'a'},{name:'b'},{name:'c'}];
    R.fill=r=>log.push(r.name);
    R.queue(rows[0]);
    setTimeout(()=>R.queue(rows[1]),30);
    setTimeout(()=>R.queue(rows[2]),150);
    setTimeout(()=>{console.log(JSON.stringify(log));process.exit(0)},400);""")
    assert got == ["a", "b", "c"]


def test_wiff_same_sample_name_never_gets_an_unexplained_sN():
    # two samples (or two files) with the same DATA name: the counter of the .raw files, never "_s<index>"
    got = run("""const t=[], out=[];
      for (const [id, nm] of [[1,"NF_t0_nospik"],[2,"NF_t0_spik_80L"],[3,"NF_t0_nospik"],[4,"NF_t0_spik_80L"],[5,"NF_t0_nospik"]]) { const n=c.wiffName("batch",id,true,t,nm); t.push(n); out.push(n); }
      console.log(JSON.stringify(out))""")
    assert got == ["NF_t0_nospik.mzML", "NF_t0_spik_80L.mzML", "NF_t0_nospik.wiff.mzML", "NF_t0_spik_80L.wiff.mzML", "NF_t0_nospik_2.wiff.mzML"]
    assert not any("_s" in n.replace("_spik", "") for n in got)


def test_a_hash_in_a_file_name_becomes_an_underscore():
    # "#" separates a file from its items in the list: the name the file enters under; both files of a pair alike; another file takes a free name
    got = run("""console.log(JSON.stringify([c.hashFree("Q.mzML", []), c.hashFree("P#1.mzML", []), c.hashFree("P#1.mzML", ["p_1.mzml"]),
        c.hashFree("P#1.mzML", ["p_1.mzml", "P_1_2.mzML"]), c.hashFree("A#1.wiff", ["a_1.wiff"]), c.hashFree("A#1.WIFF.SCAN", []), c.hashFree("x#.mzML.gz", ["x_.mzml.gz"]),
        c.wiffName("S", 1, false, [], "Run #3")]))""")
    assert got == [None, "P_1.mzML", "P_1_2.mzML", "P_1_3.mzML", "A_1.wiff", "A_1.WIFF.SCAN", "x__2.mzML.gz", "Run _3.mzML"]


def test_a_download_of_a_module_sample_follows_the_compression():
    """A sample the readers module converted at the loading (`CONV.sources`): every compression, "standard" included, is written again once with `to_mzml_come`
    without the optical matrix (the file of `convert`, conv.dad false) and saved; the file of the loading (UV panel) is never the download."""
    got = run("""
      const C = c.CONV, asked = [], saved = [];
      global.window = { LETTORI_MOD: { toMzml: async (fam, mem, k, dest, on, sig, opts) => { asked.push([fam, mem.map(m => m.name), k, opts]); return { size: 5, tag: "np" }; } },
                        CONVCARD: () => ({ start: () => ({ update() {}, done() {}, fail() {} }), end() {} }) };
      global.document = { getElementById: () => ({}), querySelectorAll: () => [] };
      global.carica = async () => {};
      global.saveBlob = (b, n) => saved.push([n, b.tag]);
      C.add("a.mzML", { size: 9, tag: "std" }, { fam: "sciex", mem: [{ name: "a.wiff" }, { name: "a.wiff.scan" }], c: 1 });
      (async () => {
        C.compression = "standard"; await C.download("a.mzML");
        C.compression = "smallest"; await C.download("a.mzML"); await C.download("a.mzML");
        console.log(JSON.stringify({ asked, saved }));
      })();""")
    assert got == {"asked": [["sciex", ["a.wiff", "a.wiff.scan"], 1, {"dad": False, "compression": "standard"}],
                             ["sciex", ["a.wiff", "a.wiff.scan"], 1, {"dad": False, "compression": "smallest"}]],
                   "saved": [["a.mzML", "np"], ["a.mzML", "np"], ["a.mzML", "np"]]}
