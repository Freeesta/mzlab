"""FILESICURI (mzlab/web/file-sicuri.js): size/depth caps, prototype keys dropped, assignKnown keeps known keys of the same type."""
import json, shutil, subprocess, pathlib
import pytest

SRC = pathlib.Path(__file__).resolve().parent.parent / "mzlab/web/file-sicuri.js"
NODE = shutil.which("node")


def run(js):
    code = SRC.read_text() + "\n" + js
    r = subprocess.run([NODE, "-e", code], capture_output=True, text=True, encoding="utf-8", timeout=30)
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout)


@pytest.mark.skipif(not NODE, reason="node not available")
def test_limits_and_keys():
    out = run('''
const t = f => { try { return f(); } catch (e) { return "ERR:" + e.message; } };
const deep = "[".repeat(40) + "]".repeat(40);
const o = FILESICURI.parse('{"__proto__":{"x":1},"a":{"constructor":2,"b":3}}');
const tgt = { w: 1.2, m: "classic", n: 3 };
FILESICURI.assignKnown(tgt, JSON.parse('{"w":"bad","m":"x","n":Infinity0}'.replace("Infinity0","1e999")));
console.log(JSON.stringify({
  deep: t(() => FILESICURI.parse(deep)),
  big: t(() => FILESICURI.parse("[" + "1,".repeat(10) + "1]", { maxBytes: 5 })),
  nodes: t(() => FILESICURI.parse("[1,2,3,4]", { maxNodes: 3 })),
  strBrackets: FILESICURI.depthOf('{"a":"[[[[\\\\"[["}'),
  own: Object.keys(o), a: Object.keys(o.a), protoClean: ({}).x === undefined,
  tgt, num: [FILESICURI.num(5, 0, 10), FILESICURI.num(NaN, 0, 10), FILESICURI.num("5", 0, 10), FILESICURI.num(11, 0, 10)],
}));''')
    assert out["deep"] == "ERR:tooDeep" and out["big"] == "ERR:tooBig" and out["nodes"] == "ERR:tooBig"
    assert out["strBrackets"] == 1
    assert out["own"] == ["a"] and out["a"] == ["b"] and out["protoClean"]
    assert out["tgt"] == {"w": 1.2, "m": "x", "n": 3}
    assert out["num"] == [5, None, None, None]


@pytest.mark.skipif(not NODE, reason="node not available")
def test_read_text_cap_and_key():
    out = run('''
(async () => {
  const ok = await FILESICURI.readText(new File(["a;b"], "x.csv"));
  let err = null;
  try { await FILESICURI.readText(new File(["0123456789"], "y.csv"), 5); } catch (e) { err = { m: e.message, k: e.key }; }
  console.log(JSON.stringify({ ok, err }));
})();''')
    assert out["ok"] == "a;b" and out["err"] == {"m": "tooBig", "k": "workflow.err.tooBig"}
