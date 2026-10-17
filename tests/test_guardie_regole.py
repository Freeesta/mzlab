"""Test guards for Federico's fixed rules in mzLab:
1. Everything in Rust (Tutto in Rust): computational logic lives in Rust (crates/),
   Python is strictly for tests/tools/parity reference, JavaScript is strictly for UI.
   No unauthorized Python scripts in the mzlab/ tree or web distribution.
2. Tabui English only (Tabui solo in inglese): TP_Mine/ is English-only, has no lang/ keys,
   no references to I18N, and the word 'formula' is never bare (always qualified,
   e.g. 'Neutral formula', '[M+H]+ formula').
"""
import csv
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MZLAB = ROOT / "mzlab"
TP_MINE = ROOT / "TP_Mine"

# Closed whitelist of allowed reference Python files in mzlab/ (parity reference models).
# No new Python file may be added to this list.
REFERENCE_PYTHON = {
    "mzlab/__init__.py", "mzlab/analog.py", "mzlab/api.py", "mzlab/api_aiuti.py",
    "mzlab/app.py", "mzlab/bigfiles.py", "mzlab/chem/__init__.py",
    "mzlab/chem/composition.py", "mzlab/chem/contaminants.py", "mzlab/chem/elements.py",
    "mzlab/chem/isoscore.py", "mzlab/chem/msntree.py", "mzlab/chem/subformulas.py",
    "mzlab/demo.py", "mzlab/differenza.py", "mzlab/explore.py", "mzlab/i18n.py",
    "mzlab/ionfamily.py", "mzlab/peaks.py", "mzlab/project.py", "mzlab/qaqc.py",
    "mzlab/reader/__init__.py", "mzlab/reader/methodinfo.py", "mzlab/reader/mzidx.py",
    "mzlab/reader/mzml.py", "mzlab/reader/ole.py", "mzlab/reader/profile.py",
    "mzlab/statistica.py",
}

# Italian words regex to detect unauthorized Italian text in Tabui UI/data
ITALIAN_WORDS = re.compile(
    r"\b(nessun[a-z]*|della|delle|degli|dello|nella|nelle|sono|questo|questa|perch[eé]|"
    r"anche|pi[uù]|pu[oò]|oppure|tabella|banco|scegli|apri|chiudi|salva|scarica|errore|avviso|"
    r"campion[ei]|file aperti|lacune|radicali|ozono|metabolismo|fase I|non (?!-)(?=[a-zàèéìòù]))\b",
    re.IGNORECASE,
)

LITERAL_STRINGS = re.compile(r'"((?:[^"\\\n]|\\.)*)"|\'((?:[^\'\\\n]|\\.)*)\'|`((?:[^`\\]|\\.)*)`', re.DOTALL)


def strip_js_comments(source: str) -> str:
    """Strip block and line comments from JavaScript source, preserving URLs."""
    clean = re.sub(r"/\*.*?\*/", "", source, flags=re.DOTALL)
    lines = []
    for line in clean.split("\n"):
        if "//" in line and not re.search(r"https?://", line):
            line = re.sub(r"(^|\s)//.*$", "", line)
        lines.append(line)
    return "\n".join(lines)


def extract_literals(path: Path):
    """Yield all string literals from a JavaScript file."""
    text = strip_js_comments(path.read_text(encoding="utf-8"))
    for match in LITERAL_STRINGS.finditer(text):
        yield next(g for g in match.groups() if g is not None)


# ==============================================================================
# 1. Regola TUTTO IN RUST
# ==============================================================================


def test_guardia_tutto_in_rust_albero_mzlab_nessun_python_non_autorizzato():
    """Tutto in Rust rule: engine computation must live in Rust (crates/).

    The Python package mzlab/ is strictly a parity reference; no unauthorized Python
    scripts may be introduced in mzlab/.
    """
    cmd = ["git", "ls-files", "mzlab"]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    py_files = sorted(f for f in out if f.endswith(".py"))
    unauthorized = [f for f in py_files if f not in REFERENCE_PYTHON]
    assert not unauthorized, (
        f"Unauthorized Python files found in mzlab/: {unauthorized}. "
        "Rule Tutto in Rust: all engine computation and algorithms must be written "
        "in Rust (crates/), compiled to native or WASM."
    )


def test_guardia_tutto_in_rust_web_senza_python():
    """Web distribution rule: mzlab/web/ must never contain any Python files."""
    cmd = ["git", "ls-files", "mzlab/web"]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    web_py = [f for f in out if f.endswith(".py") or f.endswith(".pyc")]
    assert not web_py, f"Python files found in web distribution (mzlab/web/): {web_py}"


def test_guardia_tutto_in_rust_tpmine_senza_runtime_python():
    """Tabui / TP_Mine runtime must have zero Python execution files.

    Only test files in TP_Mine/tests/ are allowed for offline data generation.
    """
    cmd = ["git", "ls-files", "TP_Mine"]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    py_runtime = [f for f in out if f.endswith(".py") and not f.startswith("TP_Mine/tests/")]
    assert not py_runtime, (
        f"Python runtime script found in TP_Mine: {py_runtime}. "
        "Tabui calculation runs solely in Rust/WASM."
    )


# ==============================================================================
# 2. Regola TABUI SOLO IN INGLESE
# ==============================================================================


def test_guardia_tabui_nessuna_chiave_lang():
    """Tabui is English only: TP_Mine/js files must not use the lang/ translation catalogs."""
    for subdir in ("js", "lazy", "files"):
        for path in sorted((TP_MINE / subdir).glob("*.js")):
            src = path.read_text(encoding="utf-8")
            assert "I18N" not in src, f"{path.name} references I18N language catalog"
            assert not re.search(r'["\']lang/[a-z]{2}\.js["\']', src), (
                f"{path.name} imports or references a lang/ catalog"
            )


def test_guardia_tabui_nessuna_chiave_interna_in_lang_catalog():
    """Main application lang catalogs (lang/it.js, lang/en.js) must not contain internal Tabui keys."""
    for lang in ("it", "en"):
        src = (ROOT / "mzlab" / "web" / "lang" / f"{lang}.js").read_text(encoding="utf-8")
        assert not re.search(r'"tpmine\.', src), f"mzlab/web/lang/{lang}.js has tpmine.* keys"


def test_guardia_tabui_testi_ui_in_inglese():
    """Tabui UI strings in TP_Mine/js, TP_Mine/lazy, TP_Mine/files must be strictly in English."""
    violations = []
    for subdir in ("js", "lazy", "files"):
        for path in sorted((TP_MINE / subdir).glob("*.js")):
            for lit in extract_literals(path):
                # HTML tags and attributes are code, not user-facing text
                text = re.sub(r"<[^>]*>", " ", lit)
                if " " in text.strip() and ITALIAN_WORDS.search(text):
                    violations.append(f"{path.relative_to(ROOT)}: {lit[:75]}")

    assert not violations, "Italian text found in Tabui files:\n" + "\n".join(violations[:25])


def test_guardia_tabui_formula_mai_da_sola():
    """Rule 'Formula mai da sola': the word 'formula' must never appear alone.

    It must always be qualified (e.g. 'Neutral formula', '[M+H]+ formula',
    'Ion formula', 'Fragment ion formula', 'Formula change').
    Applies to table headers, UI labels, CSV column headers, and exports.
    """
    # 1. Check TP_Mine curated input dataset composti.csv
    with open(TP_MINE / "data" / "composti.csv", encoding="utf-8", newline="") as f:
        reader = csv.reader(f)
        header = next(reader)
    assert "formula" not in [col.strip().lower() for col in header], (
        "Bare 'formula' header in TP_Mine/data/composti.csv. Must be 'neutral_formula'."
    )
    assert "neutral_formula" in header

    # 2. Check UI table headers in TP_Mine/js, TP_Mine/lazy, TP_Mine/files
    bare_header_regex = re.compile(r"""<th\b[^>]*>\s*formula\s*</th>""", re.IGNORECASE)
    bare_label_regex = re.compile(r"""\blabel\s*:\s*["']formula["']""", re.IGNORECASE)
    bare_th_call_regex = re.compile(r"""\bth\(\s*["'][^"']*["']\s*,\s*["']formula["']""", re.IGNORECASE)

    violations = []
    for subdir in ("js", "lazy", "files"):
        for path in sorted((TP_MINE / subdir).glob("*.js")):
            src = path.read_text(encoding="utf-8")
            if bare_header_regex.search(src):
                violations.append(f"{path.name}: <th>formula</th> table header")
            if bare_label_regex.search(src):
                violations.append(f"{path.name}: label: 'formula'")
            if bare_th_call_regex.search(src):
                violations.append(f"{path.name}: th(..., 'formula')")

    assert not violations, (
        "Bare 'formula' found in Tabui UI/table definitions:\n" + "\n".join(violations)
    )
