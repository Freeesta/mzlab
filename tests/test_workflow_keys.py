"""The workflow loader reads exactly the parameter keys the exporter writes (a renamed key left in one half silently restores nothing)."""
import re
from pathlib import Path

SRC = (Path(__file__).resolve().parents[1] / "mzlab" / "web" / "workflow.js").read_text(encoding="utf-8")


def test_loader_reads_only_keys_the_exporter_writes():
    written = set(re.findall(r"^\s{6}(\w+):", SRC.split("const parameters = {")[1].split("};")[0], re.M))
    loader = SRC.split("const p = wf.parameters || {};")[1]
    read = set(re.findall(r"\bp\.(\w+)", loader.split("QAQC.importState", 1)[0]))
    assert {"theme", "palette", "ppm", "dec"} <= written, written
    assert read and read <= written, sorted(read - written)
    # the theme is assigned from the same key it is tested with
    assert re.search(r"includes\(p\.theme\)\) UIP\.theme = p\.theme;", SRC)
