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

import sys
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import lang_catalogo  # noqa: E402
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
# 1. Rule ALL LOGIC IN RUST
# ==============================================================================


def test_guard_all_in_rust_mzlab_tree_no_unauthorized_python():
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


def test_guard_all_in_rust_web_without_python():
    """Web distribution rule: mzlab/web/ must never contain any Python files."""
    cmd = ["git", "ls-files", "mzlab/web"]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    web_py = [f for f in out if f.endswith(".py") or f.endswith(".pyc")]
    assert not web_py, f"Python files found in web distribution (mzlab/web/): {web_py}"


def test_guard_all_in_rust_tpmine_without_python_runtime():
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
# 2. Rule TABUI ENGLISH ONLY
# ==============================================================================


def test_guard_tabui_no_lang_key():
    """Tabui is English only: TP_Mine/js files must not use the lang/ translation catalogs."""
    for subdir in ("js", "lazy", "files"):
        for path in sorted((TP_MINE / subdir).glob("*.js")):
            src = path.read_text(encoding="utf-8")
            assert "I18N" not in src, f"{path.name} references I18N language catalog"
            assert not re.search(r'["\']lang/[a-z]{2}\.js["\']', src), (
                f"{path.name} imports or references a lang/ catalog"
            )


def test_guard_tabui_no_internal_key_in_lang_catalog():
    """Main application lang catalogs (lang/it.js, lang/en.js) must not contain internal Tabui keys."""
    for lang in ("it", "en"):
        src = lang_catalogo.testo(lang)
        assert not re.search(r'"tpmine\.', src), f"mzlab/web/lang/{lang}.js has tpmine.* keys"


def test_guard_tabui_ui_texts_in_english():
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


def test_guard_tabui_formula_never_alone():
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


def test_guard_no_relaxed_simd_in_build_files():
    """Relaxed SIMD gives CPU-dependent results: the site and the app must stay byte for byte equal, so it never enters a build file."""
    files = [ROOT / ".cargo/config.toml", ROOT / "Cargo.toml", ROOT / "rust-toolchain.toml"]
    files += sorted((ROOT / "tools").glob("*.sh")) + sorted((ROOT / ".github/workflows").glob("*.yml"))
    files += sorted(ROOT.glob("crates/*/Cargo.toml")) + sorted(ROOT.glob("crates/*/.cargo/config.toml"))
    found = []
    for f in files:
        if f.exists() and re.search(r"relaxed[-_]simd", f.read_text(encoding="utf-8"), re.IGNORECASE):
            found.append(str(f.relative_to(ROOT)))
    assert not found, f"relaxed SIMD in build files: {found}"


# Std sorts cost 2-10 KB of .wasm per instance: the wasm crates use `mzlab_picchi::sortc` (docs/agenti/wasm-tetti.md, «Ordinamenti»).
SORT_STD = re.compile(r"\.sort(_unstable)?(_by(_key)?|_by_cached_key)?\(")
# folders that keep std sorts (raw-data detection, legacy reference)
SORT_STD_ALLOWED = ("crates/mzlab-feat/src/untargeted/", "crates/mzlab-feat/src/legacy/")


def test_guard_std_sorts_only_sortc_in_wasm_crates():
    found = []
    for crate in ("mzlab-lib", "mzlab-feat", "mzlab-feat-wasm"):
        for f in sorted((ROOT / "crates" / crate / "src").rglob("*.rs")):
            rel = f.relative_to(ROOT).as_posix()
            if rel.startswith(SORT_STD_ALLOWED) or f.stem.endswith("tests") or f.parent.name == "tests":
                continue
            testo = f.read_text(encoding="utf-8").split("#[cfg(test)]")[0]
            for n, row in enumerate(testo.splitlines(), 1):
                if SORT_STD.search(row) and not row.lstrip().startswith("//"):
                    found.append(f"{rel}:{n}")
    assert not found, f"use sortc (SortC::sortc_by...) instead of std sorts: {found}"


# Std exp/ln/pow/trig come from the system library in a native build and from the libm port in the .wasm, which differ in the last bit:
# the engine crates call `libm::` so the site, the app and the CLI write the same bytes (docs/agenti/parita.md, «Funzioni matematiche»).
MATH_STD = re.compile(r"\.(exp|ln|log10|log2|powf|sin|cos|tan|asin|acos|atan|atan2|sinh|cosh|tanh|exp2|exp_m1|ln_1p|cbrt|hypot)\(")


def test_guard_engine_math_from_libm():
    found = []
    for crate in ("mzlab-core", "mzlab-picchi", "mzlab-feat", "mzlab-lib", "mzlab-chem", "mzlab-target"):
        for f in sorted((ROOT / "crates" / crate / "src").rglob("*.rs")):
            if "test" in f.stem or f.parent.name == "tests":
                continue
            testo = f.read_text(encoding="utf-8").split("#[cfg(test)]")[0]
            for n, row in enumerate(testo.splitlines(), 1):
                if MATH_STD.search(row) and not row.lstrip().startswith("//"):
                    found.append(f"{f.relative_to(ROOT).as_posix()}:{n}")
    assert not found, f"use libm:: (libm::exp, libm::log, libm::pow, ...) instead of std math: {found}"


def test_guardia_costanti_di_massa_solo_in_elements():
    """Proton, electron and 13C step are written once (mzlab-core elements.rs); crates import them by name."""
    pat = re.compile(r"1\.00727|1\.007_276|0\.00054858|1\.00335|1\.003_35")
    found = []
    for f in sorted(ROOT.glob("crates/*/src/**/*.rs")):
        rel = f.relative_to(ROOT).as_posix()
        if rel.endswith("mzlab-core/src/elements.rs") or "tests" in f.name or "/tests" in rel:
            continue
        for n, row in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            if pat.search(row) and not row.lstrip().startswith("//"):
                found.append(f"{rel}:{n}")
    assert not found, f"mass constant literals outside elements.rs: {found}"


def test_guard_rustc_bootstrap_only_in_site_wasm_builds():
    """RUSTC_BOOTSTRAP / -Z flags live only in tools/cargo_wasm.sh; native builds and the repo config never see them,
    and the site workflows and scripts build their .wasm through it."""
    assert "RUSTC_BOOTSTRAP" not in (ROOT / ".cargo" / "config.toml").read_text(encoding="utf-8")
    assert not re.search(r'"-Z', (ROOT / ".cargo" / "config.toml").read_text(encoding="utf-8"))
    assert not re.search(r"RUSTC_BOOTSTRAP\s*[:=]\s*[\"']?1", (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
    users = [".github/workflows/pages.yml", ".github/workflows/sito-rust.yml", ".github/workflows/release.yml",
             ".github/workflows/notte.yml", "tools/lab/qqq-pages.yml", "tools/build_feat_wasm.sh", "tools/build_lettori_wasm.sh"]
    for rel in users:
        text = (ROOT / rel).read_text(encoding="utf-8")
        assert not re.search(r"(?<![\w/.-])cargo build [^\n]*wasm32-unknown-unknown", text), rel
        assert "tools/cargo_wasm.sh build" in text, rel
    others = [p for p in list((ROOT / ".github").rglob("*.yml")) + list((ROOT / "tools").rglob("*.sh"))
              if p.name != "cargo_wasm.sh" and "RUSTC_BOOTSTRAP" in p.read_text(encoding="utf-8")]
    assert not others, others
