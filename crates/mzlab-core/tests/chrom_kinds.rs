//! The kind and the unit of the non-MS chromatograms: same rules as `mzlab/reader/mzml.py` (parity with the Python engine).

use mzlab_core::mzml::{read, ChromKind};

fn chrom(index: usize, id: &str, cv: &str, unit: &str) -> String {
    // two points, uncompressed 64-bit floats: time 0, 1 min; values 5, 6
    let t = "AAAAAAAAAAAAAAAAAADwPw==";
    let y = "AAAAAAAAFEAAAAAAAAAYQA==";
    format!(
        r#"<chromatogram index="{index}" id="{id}" defaultArrayLength="2">{cv}<binaryDataArrayList count="2">
<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000595" name="time array" value="" unitName="minute"/><binary>{t}</binary></binaryDataArray>
<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000515" name="intensity array" value="" unitName="{unit}"/><binary>{y}</binary></binaryDataArray>
</binaryDataArrayList></chromatogram>"#
    )
}

#[test]
fn kinds_and_units() {
    let cl = [
        chrom(0, "TIC", r#"<cvParam cvRef="MS" accession="MS:1000235" name="total ion current chromatogram" value=""/>"#, ""),
        chrom(1, "TWC", "", "mAU"),
        chrom(2, "Pump Pressure 1", r#"<cvParam cvRef="MS" accession="MS:1003019" name="pressure chromatogram" value=""/>"#, "pascal"),
        chrom(3, "Column oven", "", "degree Celsius"),
        chrom(4, "UV 254", r#"<cvParam cvRef="MS" accession="MS:1000812" name="x" value=""/>"#, ""),
    ]
    .join("");
    let xml = format!(r#"<mzML><run id="r"><chromatogramList count="5">{cl}</chromatogramList></run></mzML>"#);
    let run = read(xml.as_bytes()).unwrap();
    let k: Vec<ChromKind> = run.chromatograms.iter().map(|c| c.kind).collect();
    assert_eq!(k, [ChromKind::Tic, ChromKind::Pda, ChromKind::Pressure, ChromKind::Analog, ChromKind::Pda]);
    assert_eq!(run.chromatograms[2].unit, "pascal");
    assert_eq!(run.chromatograms[3].unit, "degree Celsius");
}

#[test]
fn integer_arrays_are_read_as_integers() {
    // msconvert writes some arrays as 32/64-bit integers (MS:1000519 / MS:1000522): read as floats they are garbage
    let t = "AAAAAAAAAAAAAAAAAADwPw==";
    let one = |acc: &str, y: &str| {
        format!(
            r#"<mzML><run id="r"><chromatogramList count="1"><chromatogram index="0" id="x" defaultArrayLength="2"><binaryDataArrayList count="2">
<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000595" name="time array" value="" unitName="minute"/><binary>{t}</binary></binaryDataArray>
<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="{acc}" name="int" value=""/><cvParam cvRef="MS" accession="MS:1000515" name="intensity array" value=""/><binary>{y}</binary></binaryDataArray>
</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>"#
        )
    };
    for (acc, y) in [("MS:1000519", "BQAAAAYAAAA="), ("MS:1000522", "BQAAAAAAAAAGAAAAAAAAAA==")] {
        let run = read(one(acc, y).as_bytes()).unwrap();
        assert_eq!(run.chromatograms[0].intensity, [5.0, 6.0], "{acc}");
    }
}
