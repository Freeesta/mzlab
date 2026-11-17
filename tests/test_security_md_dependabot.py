"""The Dependabot sentence of SECURITY.md names exactly the `package-ecosystem` values of .github/dependabot.yml."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def configured():
    text = (ROOT / ".github" / "dependabot.yml").read_text(encoding="utf-8")
    return {m.group(1).lower() for m in re.finditer(r"^\s*-?\s*package-ecosystem:\s*[\"']?([\w-]+)", text, re.M)}


def documented():
    m = re.search(r"Dependabot \(([^;)]+)[;)]", (ROOT / "SECURITY.md").read_text(encoding="utf-8"))
    assert m, "SECURITY.md has no «Dependabot (<ecosystems>; ...)» sentence"
    return {x.strip().lower().replace(" ", "-") for x in m.group(1).split(",")}


def test_security_md_names_the_ecosystems_of_dependabot_yml():
    assert configured() and documented() == configured(), (documented(), configured())
