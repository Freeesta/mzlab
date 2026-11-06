"""No key is ever written where it could travel: not in the catalogs of texts, not in the schemas, not in the sample of the log of the assistant."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FORMS = re.compile(r"sk-[A-Za-z0-9_-]{16,}|AIza[0-9A-Za-z_-]{20,}|Bearer\s+[A-Za-z0-9._-]{20,}")
FIELDS = {"api_key", "apikey", "secret", "token", "authorization", "password"}


def names(node):
    """The names of the properties of a schema, or the keys of a record (not the words of the schema language)."""
    if isinstance(node, dict):
        for k, v in node.items():
            if k not in ("type", "enum", "const", "pattern", "description", "title", "minimum"):
                yield k.lower()
            yield from names(v)
    elif isinstance(node, list):
        for v in node:
            yield from names(v)


def test_nessuna_forma_di_chiave_nei_cataloghi_e_nei_dati():
    paths = list((ROOT / "mzlab/web/lang").rglob("*.js")) + list((ROOT / "mzlab/web/llm").glob("*")) + [f for f in (ROOT / "tests/data/llm").rglob("*") if f.is_file()]
    paths += [ROOT / "crates/mzlab-llm/data/fornitori-1.json"]
    for p in paths:
        assert not FORMS.search(p.read_text(encoding="utf-8")), p


def test_schemi_senza_campi_chiave():
    for p in (ROOT / "docs/schemi").glob("llm*.json"):
        assert not (FIELDS & set(names(json.loads(p.read_text(encoding="utf-8")).get("properties", {})))), p


def test_campione_del_log_senza_chiavi():
    for p in (ROOT / "tests/data/llm").glob("*.jsonl"):
        for line in p.read_text(encoding="utf-8").splitlines():
            if line.strip():
                assert not (FIELDS & set(names(json.loads(line)))), p


def test_le_esportazioni_non_nominano_il_file_delle_chiavi():
    for p in list((ROOT / "mzlab/web").glob("*.js")) + list((ROOT / "crates/mzlab-cli/src").rglob("*.rs")):
        assert "llm-chiavi" not in p.read_text(encoding="utf-8"), p


def test_la_frase_del_proprietario_resta_nel_trasporto():
    """The owner passphrase of the demo (`x-mzlab-owner`) lives only in the transport document (site) or in the keys file read by Rust (app):
    the page and the worker never carry it, and the session of the panel is the only demo value they send."""
    web = ROOT / "mzlab/web/llm"
    worker = (web / "llm-worker.js").read_text(encoding="utf-8")
    assert re.findall(r"demo_send\([^;]*\)", worker) and all(c.endswith(', "")') for c in re.findall(r"demo_send\([^;]*\)", worker)), "the worker passes an owner"
    clients = (web / "llm-clienti.js").read_text(encoding="utf-8")
    assert not re.search(r"this\.owner|owner:\s*this", clients), "a client keeps the passphrase"
    for p in list(web.glob("*.js")) + list((ROOT / "flow/lazy").glob("flow-llm*.js")):
        if p.name != "trasporto.js":
            assert "headers.set" not in p.read_text(encoding="utf-8"), p
    transport = (web / "trasporto.js").read_text(encoding="utf-8")
    assert "headers.set(st.owner, key)" in transport and 'const DB = "mzlab_llm"' in transport and "localStorage" not in transport
    # in the app the passphrase is the demo's entry of llm-chiavi.json, read by Rust only and never echoed by `llm/providers`
    app = (ROOT / "crates/mzlab-desktop/src/llm.rs").read_text(encoding="utf-8")
    assert 'Some(k) if prov.id == DEMO_PROVIDER => k.as_str()' in app and 's("owner")' not in app
    # neither the run record nor the exports name it
    for p in list((ROOT / "mzlab/web").glob("*.js")) + list((ROOT / "crates/mzlab-cli/src").rglob("*.rs")) + list((ROOT / "flow/lazy").glob("*.js")):
        assert "x-mzlab-owner" not in p.read_text(encoding="utf-8"), p
