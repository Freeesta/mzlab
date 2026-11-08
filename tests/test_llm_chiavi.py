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


def test_no_key_shape_in_catalogs_and_data():
    paths = list((ROOT / "mzlab/web/lang").rglob("*.js")) + list((ROOT / "mzlab/web/llm").glob("*")) + [f for f in (ROOT / "tests/data/llm").rglob("*") if f.is_file()]
    paths += [ROOT / "crates/mzlab-llm/data/fornitori-1.json"]
    for p in paths:
        assert not FORMS.search(p.read_text(encoding="utf-8")), p


def test_schemas_without_key_fields():
    for p in (ROOT / "docs/schemi").glob("llm*.json"):
        assert not (FIELDS & set(names(json.loads(p.read_text(encoding="utf-8")).get("properties", {})))), p


def test_log_sample_without_keys():
    for p in (ROOT / "tests/data/llm").glob("*.jsonl"):
        for line in p.read_text(encoding="utf-8").splitlines():
            if line.strip():
                assert not (FIELDS & set(names(json.loads(line)))), p


def test_exports_do_not_name_the_keys_file():
    for p in list((ROOT / "mzlab/web").glob("*.js")) + list((ROOT / "crates/mzlab-cli/src").rglob("*.rs")):
        assert "llm-chiavi" not in p.read_text(encoding="utf-8"), p
