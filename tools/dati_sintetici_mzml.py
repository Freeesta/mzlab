"""The parts of a valid mzML that tools/dati_sintetici.py shares: cvList, file description, software and processing lists, the chromatogram type term
(the PSI-MS rules of `mzlab-cli validate`)."""
from pathlib import Path

_CVLIST = ('<cvList count="2"><cv id="MS" fullName="Proteomics Standards Initiative Mass Spectrometry Ontology" version="4.1.163" URI="x"/>'
           '<cv id="UO" fullName="Unit Ontology" version="09:04:2014" URI="x"/></cvList>')


_FILE_PARTS_A = ('<fileDescription><fileContent><cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum" value=""/></fileContent></fileDescription>'
                 '<softwareList count="1"><software id="mzlab_synthetic" version="0.0"><cvParam cvRef="MS" accession="MS:1000799" name="custom unreleased software tool" value="tools/dati_sintetici.py"/></software></softwareList>')
_FILE_PARTS_B = ('<dataProcessingList count="1"><dataProcessing id="synthetic"><processingMethod order="0" softwareRef="mzlab_synthetic">'
                 '<cvParam cvRef="MS" accession="MS:1000544" name="Conversion to mzML" value=""/></processingMethod></dataProcessing></dataProcessingList>')


def _chrom_type(cid: str, extra: str) -> str:
    """The one chromatogram type term the PSI-MS rules require, from the id (TIC, BPC, TWC, SRM ...) unless `extra` already gives it."""
    if "MS:1003019" in extra: return ""
    acc, name = (("MS:1000235", "total ion current chromatogram") if cid == "TIC" else ("MS:1000628", "basepeak chromatogram") if cid == "BPC"
                 else ("MS:1000812", "absorption chromatogram") if cid == "TWC" else ("MS:1001473", "selected reaction monitoring chromatogram") if cid.startswith("SRM")
                 else ("MS:1003019", "pressure chromatogram"))
    return f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/>'


def _write(path: Path, specs: list[str], chroms: list[str]) -> None:
    head = ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
            + _CVLIST + _FILE_PARTS_A +
            '<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1">'
            '<cvParam cvRef="MS" accession="MS:1000651" name="3200 QTRAP" value=""/></instrumentConfiguration></instrumentConfigurationList>'
            + _FILE_PARTS_B + f'<run id="{path.stem}">')
    body = (f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>") if specs else ""
    path.write_text(head + body + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>",
                    encoding="utf-8")
