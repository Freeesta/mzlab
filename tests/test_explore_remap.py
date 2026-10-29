"""explore.js: file indices of the panels across a list reopened file by file (remapFileIndex) and the provisional TIC lookup (earlyTicFor),
cut out of the source and run with node on synthetic names."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def cut(start, end):
    s = (WEB / "explore.js").read_text(encoding="utf-8")
    a = s.index(start)
    return s[a:s.index(end, a)]


def run(body):
    src = (WEB / "ordine.js").read_text(encoding="utf-8").split("if (typeof module")[0] + "const PEND = new WeakMap();\n"
    src += cut("function remapFileIndex(", "async function bootSession0") + cut("const earlyTicFor", "/** @param {string | number} k @param {string} kind")
    out = subprocess.run(["node", "-e", src + body], capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(out.stdout)


@needs_node
def test_white_and_background_file_come_back_when_files_return_one_by_one():
    got = run("""
const F = names => names.map(file => ({ file }));
const p = { k: 0, ref: 0, bk: "2", bg: "2" };
const step = (old, files) => { for (const [key, none] of [["k", 0], ["ref", 0], ["bk", ""], ["bg", ""]]) remapFileIndex(p, key, none, old, files); return { ...p }; };
const s1 = step(["a", "b", "c"], F(["a", "b"]));
const s2 = step(["a", "b"], F(["a", "b", "c"]));
console.log(JSON.stringify([s1, s2]));""")
    assert got[0]["bk"] == "" and got[0]["bg"] == ""
    assert got[1]["bk"] == "2" and got[1]["bg"] == "2"


@needs_node
def test_a_cleared_reference_stays_cleared_and_a_present_file_is_remapped():
    got = run("""
const p = { bk: "", k: 1 }; const files = [{ file: "b" }, { file: "a" }];
remapFileIndex(p, "bk", "", ["a", "b"], files); remapFileIndex(p, "k", 0, ["a", "b"], files);
console.log(JSON.stringify(p));""")
    assert got == {"bk": "", "k": 0}


@needs_node
def test_early_tic_is_found_for_a_part_of_a_file_and_only_at_level_1():
    got = run("""
const m = new Map([["x.mzML", { rt: [1] }]]), lr = n => n === "y.mzML" ? { rt: [2] } : undefined;
console.log(JSON.stringify([earlyTicFor("x.mzML#full", 1, m, lr), earlyTicFor("x.mzML#full", 2, m, lr), earlyTicFor("y.mzML#ms2", 1, m, lr), earlyTicFor("z.mzML", 1, m, lr) ?? null]));""")
    assert got == [{"rt": [1]}, None, {"rt": [2]}, None]
