"""Static check: every import in the e2e scripts (tests_e2e/, crates/*/tests/*.py) names the standard library, an installed package, a known third-party package or a Python file/package tracked in the repository.

Catches a script that still imports a module whose file was deleted or renamed (the e2e would die at start). Run: python3 tools/controlla_import.py
"""
from __future__ import annotations

import ast
import importlib.util
import sys
import warnings
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import indice  # noqa: E402

# Imported by the e2e but not installed in every environment (optional, or only on the CI runners).
THIRD_PARTY = {"playwright", "numpy", "pytest", "axe_playwright_python", "PIL", "openpyxl", "scipy", "pandas", "matplotlib", "sklearn"}


def local_names() -> set[str]:
    names: set[str] = set()
    for f in indice.tracked():
        if f.endswith(".py"):
            p = Path(f)
            names.add(p.stem)
            names.update(p.parent.parts)          # packages, also without __init__.py (namespace packages: TP_Mine)
    return names


def scripts() -> list[str]:
    return [f for f in indice.tracked() if f.endswith(".py") and (f.startswith("tests_e2e/") or (f.startswith("crates/") and "/tests/" in f))]


def imports(path: Path) -> list[tuple[str, int]]:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")           # invalid escapes inside the JS strings of a test are not our concern here
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    found = []
    for n in ast.walk(tree):
        if isinstance(n, ast.Import):
            found += [(a.name.split(".")[0], n.lineno) for a in n.names]
        elif isinstance(n, ast.ImportFrom) and n.level == 0 and n.module:
            found.append((n.module.split(".")[0], n.lineno))
    return found


def controlla() -> list[str]:
    local = local_names()
    bad = []
    for f in scripts():
        for top, line in imports(indice.ROOT / f):
            if top in sys.stdlib_module_names or top in local or top in THIRD_PARTY:
                continue
            try:
                if importlib.util.find_spec(top) is not None:
                    continue
            except (ImportError, ValueError):
                pass
            bad.append(f"{f}:{line}: import «{top}» matches no file of the repository nor an installed package")
    return bad


def main() -> int:
    bad = controlla()
    for b in bad:
        print(b)
    print(f"controlla_import: {len(bad)} unresolved import(s) in {len(scripts())} script(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
