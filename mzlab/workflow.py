"""Workflow serialization (.mzflow) and MGF / MSP spectral export.

.mzflow: JSON format (formato: "mzflow/1") containing file SHA-256 digests,
program version and commit, language, parameters (ppm, peak detection, corrections,
filters, library thresholds), and the sequence of operations with UTC timestamps.
"""
from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

FORMAT_VERSION = "mzflow/1"
LEGACY_FORMATS = ("mzworkflow/1",)   # old name, still accepted when opening
KNOWN_OPS = frozenset({
    "export_all_ms2_mgf", "export_all_ms2_msp", "export_mgf_spectrum", "export_msp_spectrum",
    "export_report", "export_workflow", "qaqc_measure", "qaqc_ratio", "workflow_applied",
})


def compute_sha256(path: Path | str) -> str:
    """Compute SHA-256 of a file in 1 MB chunks."""
    p = Path(path)
    if not p.is_file():
        return ""
    h = hashlib.sha256()
    with open(p, "rb") as fh:
        while chunk := fh.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


def create_workflow(
    files: list[dict],
    parameters: dict,
    operations: list[dict] | None = None,
    panels: list[dict] | None = None,
    program_version: str = "0.1.0",
    git_commit_hash: str = "unknown",
    language: str = "it",
) -> dict:
    """Create a .mzflow dictionary structure."""
    return {
        "formato": FORMAT_VERSION,
        "programma": {
            "nome": "mzLab",
            "versione": program_version,
            "commit": git_commit_hash,
        },
        "data_creazione": datetime.now(timezone.utc).isoformat(),
        "lingua": language,
        "file": [
            {
                "nome": str(f.get("name") or f.get("file") or ""),
                "sha256": str(f.get("sha256") or ""),
                "dimensione": int(f.get("size") or f.get("dimensione") or 0),
                "tipo": str(f.get("type") or f.get("tipo") or "sample"),
                "etichetta": str(f.get("label") or f.get("etichetta") or ""),
                "tempo": f.get("time") if f.get("time") is not None else f.get("tempo"),
            }
            for f in files
        ],
        "parametri": parameters,
        "pannelli": panels or [],
        "operazioni": operations or [],
    }


def validate_workflow(wf: dict) -> tuple[bool, str]:
    """Validate a .mzflow dictionary structure."""
    if not isinstance(wf, dict):
        return False, "Not a valid JSON object"
    if wf.get("formato") not in (FORMAT_VERSION, *LEGACY_FORMATS):
        return False, f"Invalid format: expected {FORMAT_VERSION}, got {wf.get('formato')!r}"
    if "file" not in wf or not isinstance(wf["file"], list):
        return False, "Missing or invalid 'file' list"
    if "parametri" not in wf or not isinstance(wf["parametri"], dict):
        return False, "Missing or invalid 'parametri' dictionary"
    return True, "OK"


def unknown_operations(wf: dict) -> list[str]:
    """Names of the operations a reader of mzflow/1 does not know (ignored with a warning: forward compatibility)."""
    seen = []
    for o in wf.get("operazioni") or []:
        n = o.get("op") if isinstance(o, dict) else None
        if n not in KNOWN_OPS and n not in seen:
            seen.append(n)
    return seen


def verify_file_hashes(wf: dict, actual_files: dict[str, str]) -> list[dict]:
    """Check workflow file hashes against actual file hashes.

    actual_files: dict mapping file name -> sha256 hex string.
    Returns list of mismatches: [{'file': name, 'expected': exp, 'actual': act}].
    """
    mismatches = []
    for f in wf.get("file", []):
        name = f.get("nome")
        exp = str(f.get("sha256") or "").strip().lower()
        if name in actual_files:
            act = str(actual_files[name] or "").strip().lower()
            if exp and act and exp != act:
                mismatches.append({"file": name, "expected": exp, "actual": act})
    return mismatches


# -------------------------------------------------------------------- MGF export and parser
def format_mgf(spectra: list[dict], lr: bool = False) -> str:
    """Format a list of MS2 spectra into Mascot Generic Format (MGF).

    Each spectrum dict:
      - sid / scan: int or str (scan identifier)
      - rt: float (retention time in minutes)
      - prec: float (precursor m/z)
      - charge: int (default 1)
      - polarity: int (1 for positive, -1 for negative, default 1)
      - file: str (file name)
      - peaks: list of (mz, intensity) tuples/lists
      - title_extra: str (optional, ignored if lr=True)
    In LR, only spectra, charge, RT, precursor, and file name are exported (no annotations or names).
    """
    blocks = []
    for s in spectra:
        lines = ["BEGIN IONS"]
        sid = s.get("sid", s.get("scan", ""))
        rt = float(s.get("rt", 0.0) or 0.0)
        file_name = s.get("file", "")
        title = f"scan={sid} rt={rt:.4f} file={file_name}"
        if not lr and s.get("title_extra"):
            title += f" {s['title_extra']}"
        lines.append(f"TITLE={title}")
        lines.append(f"RTINSECONDS={rt * 60:.2f}")
        if s.get("prec") is not None:
            lines.append(f"PEPMASS={s['prec']}")
        charge = int(s.get("charge", 1) or 1)
        pol = int(s.get("polarity", 1) or 1)
        sign = "+" if pol >= 0 else "-"
        lines.append(f"CHARGE={abs(charge)}{sign}")
        for mz, inten in s.get("peaks", []):
            lines.append(f"{float(mz):.5f} {float(inten):.1f}")
        lines.append("END IONS")
        blocks.append("\n".join(lines))
    return "\n\n".join(blocks) + ("\n" if blocks else "")


def parse_mgf(text: str) -> list[dict]:
    """Minimal MGF parser for tests and verification."""
    spectra = []
    current: dict | None = None
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line == "BEGIN IONS":
            current = {"params": {}, "peaks": []}
        elif line == "END IONS":
            if current:
                spectra.append(current)
                current = None
        elif current is not None:
            if "=" in line:
                k, v = line.split("=", 1)
                current["params"][k.strip().upper()] = v.strip()
            else:
                parts = line.split()
                if len(parts) >= 2:
                    current["peaks"].append((float(parts[0]), float(parts[1])))
    return spectra


# -------------------------------------------------------------------- MSP export and parser
def format_msp(spectra: list[dict], lr: bool = False) -> str:
    """Format a list of MS2 spectra into NIST MSP format.

    In LR, only spectra, charge, RT, precursor, and file name are exported (no annotations or names).
    """
    blocks = []
    for s in spectra:
        lines = []
        sid = s.get("sid", s.get("scan", ""))
        file_name = s.get("file", "")
        name = f"{file_name}_scan_{sid}"
        if not lr and s.get("name_extra"):
            name += f"_{s['name_extra']}"
        lines.append(f"NAME: {name}")
        if s.get("prec") is not None:
            lines.append(f"PRECURSORMZ: {s['prec']}")
        pol = int(s.get("polarity", 1) or 1)
        ionmode = "Positive" if pol >= 0 else "Negative"
        prectype = "[M+H]+" if pol >= 0 else "[M-H]-"
        lines.append(f"PRECURSORTYPE: {prectype}")
        lines.append(f"IONMODE: {ionmode}")
        rt = float(s.get("rt", 0.0) or 0.0)
        lines.append(f"RETENTIONTIME: {rt:.4f}")
        peaks = s.get("peaks", [])
        lines.append(f"Num Peaks: {len(peaks)}")
        for mz, inten in peaks:
            lines.append(f"{float(mz):.5f} {float(inten):.1f}")
        blocks.append("\n".join(lines))
    return "\n\n".join(blocks) + ("\n" if blocks else "")


def parse_msp(text: str) -> list[dict]:
    """Minimal MSP parser for tests and verification."""
    spectra = []
    current: dict | None = None
    in_peaks = False
    for line in text.splitlines():
        line = line.strip()
        if not line:
            if current:
                spectra.append(current)
                current = None
                in_peaks = False
            continue
        if ":" in line and not in_peaks:
            k, v = line.split(":", 1)
            k, v = k.strip().upper(), v.strip()
            if current is None:
                current = {"params": {}, "peaks": []}
            current["params"][k] = v
            if k == "NUM PEAKS":
                in_peaks = True
        elif in_peaks or (current and " " in line):
            parts = line.replace(";", "").split()
            if len(parts) >= 2:
                try:
                    current["peaks"].append((float(parts[0]), float(parts[1])))
                except ValueError:
                    pass
    if current:
        spectra.append(current)
    return spectra
