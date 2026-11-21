"""The fields of an action of the registry (azioni/1: group, views, visible, available, shortcut{key, code, where, label}, doubleClick, download) are English; the Italian ones must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"\b(gruppo|viste|visibile|disponibile|scorciatoia|doppioOrd|scarico)\b")
# the registry files and their tests; other files use these words for other things (data groups, benchmark columns)
SCOPE = re.compile(r"mzlab/web/(azioni[\w-]*|eventi|spettro|visualizza|dda|uv-azioni|liste|libreria|arrange|workflow|metodo-incl|integra-estremi|fair-azioni|elementare|dad-mrm|composizione|area-copia|xic-spettro|sim-ui|picchi|mappa|grafici|gradiente|etichette-picchi|paletta|barra|explore|pannelli)\.js|tests/test_azioni[\w]*\.py|tests_e2e/e2e_(menu_coerenti|ms2_clic|scorciatoie|sim_wiff|fair)\.py|docs/agenti/actions\.md")


def test_old_action_fields_are_gone():
    files = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        if SCOPE.fullmatch(f) and (ROOT / f).is_file():
            for n, line in enumerate((ROOT / f).read_text(encoding="utf8").splitlines(), 1):
                if OLD.search(line):
                    hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
