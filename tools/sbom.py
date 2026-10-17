"""CycloneDX 1.5 SBOM of the JavaScript and WebAssembly components that ship with the site (the Rust crates have theirs from
`cargo cyclonedx`, see .github/workflows/sbom.yml). Versions are read from mzlab/web/vendor/README.md,
so the list cannot drift from what is shipped (tests/test_versione.py checks it).

    python3 tools/sbom.py [--out sbom-js.cdx.json]
Stdlib only."""
from __future__ import annotations

import argparse
import json
import re
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _find(pattern: str, text: str, what: str) -> str:
    m = re.search(pattern, text)
    if not m:
        raise SystemExit(f"sbom: {what} not found")
    return m.group(1)


def componenti(root: Path = ROOT) -> list[dict]:
    vendor = (root / "mzlab/web/vendor/README.md").read_text(encoding="utf-8")
    ketcher = _find(r"Ketcher (\d+\.\d+\.\d+)", vendor, "the version of Ketcher")
    ocl = _find(r"OpenChemLib JS (\d+\.\d+\.\d+)", vendor, "the version of OpenChemLib JS")

    def lib(name: str, version: str | None, lic: str, url: str, purl: str | None, note: str = "") -> dict:
        c: dict = {"type": "library", "bom-ref": f"{name}@{version or 'unversioned'}", "name": name,
                   "licenses": [{"license": {"id": lic}}],
                   "externalReferences": [{"type": "website", "url": url}]}
        if version:
            c["version"] = version
        if purl:
            c["purl"] = purl
        if note:
            c["description"] = note
        return c

    return [
        lib("ketcher", ketcher, "Apache-2.0", "https://github.com/epam/ketcher", f"pkg:github/epam/ketcher@v{ketcher}",
            "Chemical editor, standalone build (Indigo in WebAssembly); bundles React 18 (MIT)."),
        lib("openchemlib", ocl, "BSD-3-Clause", "https://github.com/cheminfo/openchemlib-js", f"pkg:npm/openchemlib@{ocl}",
            "Formula and exact mass of the drawn structures."),
    ]


def sbom(root: Path = ROOT) -> dict:
    return {
        "bomFormat": "CycloneDX", "specVersion": "1.5", "version": 1,
        "serialNumber": f"urn:uuid:{uuid.uuid4()}",
        "metadata": {"timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                     "component": {"type": "application", "name": "mzlab-site", "licenses": [{"license": {"id": "MIT"}}]}},
        "components": componenti(root),
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default="sbom-js.cdx.json")
    a = ap.parse_args(argv)
    Path(a.out).write_text(json.dumps(sbom(), indent=2) + "\n", encoding="utf-8")
    print("written", a.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
