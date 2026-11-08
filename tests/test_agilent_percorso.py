"""The Agilent refusals of the engine that the readers module may read otherwise are in the hand-off list of the site, so the site ends on the route of
the CLI (agilent_ext): a missing MSPeak.bin (the module reads MSProfile.bin) and any err.file.agilent (MRM visits, LC/MS layouts)."""
import json
import re
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"


def test_agilent_refusals_go_to_the_module():
    js = (WEB / "lettori-famiglie.js").read_text(encoding="utf-8").replace("export default LETTORI_FAM;", "").replace("window", "globalThis")
    code = js + "console.log(JSON.stringify(LETTORI_FAM.agilent.errori))"
    out = subprocess.run(["node", "-e", re.sub(r"^// @ts-check", "", code)], check=True, capture_output=True, encoding="utf-8").stdout
    assert set(json.loads(out)) >= {"err.file.agilent", "err.file.agilentNoPeaks"}
