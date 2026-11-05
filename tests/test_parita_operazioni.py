"""Every operation of the site's engine worker (mzlab/web/rust-worker.js) has a branch in the app: the desktop worker (shim/desktop-worker.js) or the
native `rpc` (crates/mzlab-desktop/src/*.rs). An operation the app does not answer gives `rpc.unknown` there, so a function would exist in the site only.
The few that are of the site by nature, or a gap not closed yet, are named in SOLO_SITO with the reason (docs/agenti/parita.md)."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# op -> why the app has no branch for it
SOLO_SITO = {
    "module": "the worker receives the compiled .wasm: the app has no .wasm",
    "policy": "the storage policy of OPFS: the app reads the disk",
    "budget": "memory budget of the wasm instance (4 GiB cap); the app has no cap",
    "down": "one memory level lower for the wasm instance; the app has no cap",
    "natsame": "samples already in OPFS with their index; the page skips it in the app (nativo.js)",
    "lr_gunzip": "a .mzXML.gz unpacked into OPFS for the readers module; the page skips it in the app (gz-flusso.js) and the app reads .gz itself",
    "lr_cen_have": "the site Flow's centroid copy in OPFS; the app's Flow runs natively (flow/run, flow/live)",
    "lr_cen_adopt": "the site Flow's centroid copy in OPFS; the app's Flow runs natively (flow/run, flow/live)",
    "lr_flow_line": "the app's page posts flow/line (flow-terminal.js)",
    "lr_flow_sched": "the app's page posts flow/live/<op> (flow-live-desktop.js)",
    "lr_cancel": "gap: the app's index build is not stopped by «Cancel» (the files are mapped, nothing is copied); the outputs do not differ",
}


def ops(text):
    return set(re.findall(r'case "([a-z_0-9]+)"', text)) | set(re.findall(r'op === "([a-z_0-9]+)"', text))


def site_ops():
    return ops((ROOT / "mzlab/web/rust-worker.js").read_text(encoding="utf-8"))


def app_ops():
    shim = ops((ROOT / "crates/mzlab-desktop/shim/desktop-worker.js").read_text(encoding="utf-8"))
    rs = "".join(p.read_text(encoding="utf-8") for p in sorted((ROOT / "crates/mzlab-desktop/src").glob("*.rs")))
    arms = set()
    for m in re.findall(r'((?:"[a-z_0-9/]+"\s*\|\s*)*"[a-z_0-9/]+")\s*=>', rs):
        arms |= set(re.findall(r'"([a-z_0-9/]+)"', m))
    arms |= set(re.findall(r'op == "([a-z_0-9]+)"', rs))
    return shim | arms


def test_every_site_operation_has_a_branch_in_the_app():
    missing = sorted(site_ops() - app_ops() - set(SOLO_SITO))
    assert not missing, f"operations of rust-worker.js the app answers with rpc.unknown: {missing} (add the branch, or name the reason in SOLO_SITO)"


def test_the_site_only_list_holds_no_stale_name():
    stale = sorted(o for o in SOLO_SITO if o not in site_ops() or o in app_ops())
    assert not stale, f"SOLO_SITO names operations the site lacks or the app answers: {stale}"
