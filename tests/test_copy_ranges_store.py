"""Store names of a partial copy (mzlab/web/rust-nativo.js, run by node): a member copied only in part carries the copy plan's tag, so picking another sample
of the same file never finds (and reuses) a sparse copy whose other ranges are zeros; a member copied whole keeps its plain name."""
import json
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def names():
    body = """
globalThis.self = globalThis;
const m = await import(URL);
const scan = { name: "a.wiff.scan", blob: { size: 100 } }, wiff = { name: "a.wiff", blob: { size: 5 } };
const one = { key: "k1", tag: "t1", ranges: { "a.wiff.scan": [[0, 10], [50, 60]], "a.wiff": [[0, 5]] } };
const two = { key: "k1", tag: "t2", ranges: { "a.wiff.scan": [[0, 10], [70, 80]], "a.wiff": [[0, 5]] } };
console.log(JSON.stringify({ s1: m.storeOf(one, scan), s2: m.storeOf(two, scan), w1: m.storeOf(one, wiff), w2: m.storeOf(two, wiff), plain: m.storeOf({ key: "k1" }, scan), parts: m.storeParts(m.storeOf(one, scan)) }));
process.exit(0);
""".replace("URL", json.dumps((WEB / "rust-nativo.js").as_uri()))
    r = subprocess.run(["node", "--input-type=module", "-e", body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_two_choices_of_samples_of_one_file_never_share_a_partial_copy():
    got = names()
    assert got["s1"] != got["s2"], got
    assert got["s1"] != got["plain"] and got["s2"] != got["plain"], got


def test_a_member_copied_whole_keeps_its_plain_name_and_the_key_and_name_are_still_found():
    got = names()
    assert got["w1"] == got["w2"] == "nat~k1~a.wiff", got
    assert got["parts"] == {"key": "k1", "name": "a.wiff.scan"}, got
