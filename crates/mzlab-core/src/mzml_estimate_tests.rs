use super::*;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

/// An mzML of `n` scans of `k` points; `wide` writes the intensities as 64-bit.
fn file(n: usize, k: usize, wide: bool) -> Vec<u8> {
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(n).unwrap();
    for sc in 0..n {
        let it: Vec<f32> = (0..k).map(|i| 10.0 + i as f32).collect();
        w.spectrum(&Spectrum {
            index: sc,
            id: format!("scan={}", sc + 1),
            level: 1 + (sc % 3 == 0) as u8,
            precursor: (sc % 3 == 0).then_some(250.0),
            rt: sc as f64 * 0.01,
            mz: (0..k).map(|i| 100.0 + i as f64 * 0.5).collect(),
            wide: wide.then(|| it.iter().map(|&v| f64::from(v) + 0.25).collect()),
            intensity: it,
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// What the decode holds: spectra and chromatograms, their points, the points of the 64-bit intensities.
fn decoded(b: &[u8]) -> Inputs {
    let run = crate::mzml::read(b).unwrap();
    let pts = |n: usize| n as u64;
    Inputs {
        spectra: (run.spectra.len() + run.chromatograms.len()) as u64,
        points: run.spectra.iter().map(|s| pts(s.mz.len())).sum::<u64>()
            + run
                .chromatograms
                .iter()
                .map(|c| pts(c.time.len()))
                .sum::<u64>(),
        wide: run
            .spectra
            .iter()
            .map(|s| pts(s.wide.as_ref().map_or(0, Vec::len)))
            .sum(),
        ms2: run.spectra.iter().filter(|s| s.level == 2).count() as u64,
        ms2_points: run
            .spectra
            .iter()
            .filter(|s| s.level == 2)
            .map(|s| pts(s.mz.len()))
            .sum(),
    }
}

#[test]
fn the_tags_count_what_the_decode_holds() {
    for wide in [false, true] {
        let b = file(37, 120, wide);
        let x = inputs(&b);
        assert_eq!(x, decoded(&b), "wide {wide}");
        assert_eq!(
            (x.wide, x.ms2, x.ms2_points),
            (if wide { 37 * 120 } else { 0 }, 13, 13 * 120)
        );
    }
}

#[test]
fn a_group_of_64_bit_arrays_counts_the_spectra_that_refer_to_it() {
    let b = br#"<mzML><referenceableParamGroupList count="1"><referenceableParamGroup id="g"><cvParam accession="MS:1000523" name="64-bit float"/></referenceableParamGroup></referenceableParamGroupList>
<spectrumList count="2"><spectrum index="0" id="a" defaultArrayLength="10"><referenceableParamGroupRef ref="g"/><binaryDataArrayList count="2"><binaryDataArray encodedLength="0"><cvParam accession="MS:1000514"/></binaryDataArray><binaryDataArray encodedLength="0"><cvParam accession="MS:1000515"/></binaryDataArray></binaryDataArrayList></spectrum>
<spectrum index="1" id="b" defaultArrayLength="4"><binaryDataArrayList count="3"><binaryDataArray><cvParam accession="MS:1000514"/></binaryDataArray><binaryDataArray><cvParam accession="MS:1000515"/><cvParam accession="MS:1000521"/></binaryDataArray><binaryDataArray><cvParam accession="MS:1002893"/></binaryDataArray></binaryDataArrayList></spectrum></spectrumList></mzML>"#;
    // the first refers to a 64-bit group, the second has a third array (ion mobility)
    assert_eq!(
        inputs(b),
        Inputs {
            spectra: 2,
            points: 14,
            wide: 14,
            ms2: 0,
            ms2_points: 0
        }
    );
}

#[test]
fn past_the_cap_the_read_is_refused_with_its_numbers() {
    let b = file(50, 2000, false);
    let need = bytes(&inputs(&b), b.len());
    assert!(check(need, need).is_ok());
    let e = check(need, need - (2 << 20)).unwrap_err().to_string();
    assert!(
        e.contains("error.memoryBudget") && e.contains("fase=read"),
        "{e}"
    );
    assert!(mgf_bytes(&inputs(&b)) > 0 && f64_copy_bytes(&inputs(&b)) > 0);
}
