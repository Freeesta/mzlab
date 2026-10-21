"""The series id of an instrument-log request travels in its own field: `id` is the engine's request counter (rpc overwrites it), so a series named by `id`
reached the module as a number (wasm-bindgen reads a number as an empty string at a bad pointer) and trapped the worker; the worker passes a string (every .raw load restarted the engine)."""
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def test_natregistro_names_the_series_in_its_own_field():
    sender = (WEB / "lr-method.js").read_text(encoding="utf-8")
    worker = (WEB / "rust-worker.js").read_text(encoding="utf-8")
    assert 'op: "natregistro", name, serie:' in sender
    assert "nativo.registro(String(m.name).split(\"#\")[0], String(m.serie || \"\"))" in worker
