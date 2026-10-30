"""The words of the regulatory screening (study §7): a match is a candidate with its evidence, never a verdict on the sample.

Greps the report wording (`corsa_reg.rs`), the page section (`rapporto-flow.js`), the matching steps and any `normative.*` text of the catalogues for the words
that would turn a list match into a finding about the sample.
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FORBIDDEN = re.compile(r"\b(detected|present|non-compliant|exceeds?|violations?|contaminated)\b", re.I)


def _rust_strings(path):
    return re.findall(r'"((?:[^"\\]|\\.)*)"', (ROOT / path).read_text(encoding="utf-8"))


def _page_section():
    s = (ROOT / "mzlab/web/rapporto-flow.js").read_text(encoding="utf-8")
    start = s.index("function regulatory(")
    return [s[start:s.index("\n  }\n", start)]]


def _lang_texts():
    out = []
    for f in ("it.js", "en.js"):
        for line in (ROOT / "mzlab/web/lang" / f).read_text(encoding="utf-8").splitlines():
            if re.search(r'["\']?(normative|regulatory)\.', line):
                out.append(line)
    return out


def test_regulatory_wording_has_no_verdict_word():
    texts = {
        "corsa_reg.rs": _rust_strings("crates/mzlab-core/src/flow/corsa_reg.rs"),
        "flow_regulatory.rs": _rust_strings("crates/mzlab-feat/src/flow_regulatory.rs"),
        "regulatory.rs": _rust_strings("crates/mzlab-feat/src/regulatory.rs"),
        "rapporto-flow.js": _page_section(),
        "lang": _lang_texts(),
    }
    bad = [(k, m.group(0), t[:80]) for k, ts in texts.items() for t in ts for m in [FORBIDDEN.search(t)] if m]
    assert not bad, bad


def test_report_carries_the_study_disclaimer():
    s = (ROOT / "crates/mzlab-core/src/flow/corsa_reg.rs").read_text(encoding="utf-8")
    assert "it is not an assessment of compliance with any law" in s
    assert "Confirm candidates with reference standards before any regulatory use." in s
