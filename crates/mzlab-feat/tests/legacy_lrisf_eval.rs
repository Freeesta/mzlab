//! `isf_eval` end to end on a synthetic full-scan series built here (centroids shifted by +0.30 Da): the parent decays, an ion at the same RT with a
//! constant ratio is inherited from it, a product forming at another RT is not. Synthetic data only.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lrisf::{isf_eval, IsfSample};

const TIMES: [f64; 5] = [5.0, 10.0, 20.0, 40.0, 60.0];
const PARENT: f64 = 364.0;

fn index_of(t: f64) -> Vec<u8> {
    let name = format!("isf{t}");
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: name.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 200;
    w.open_spectra(n).unwrap();
    let decay = (-0.03 * t).exp();
    for i in 0..n {
        let rt = 6.0 + 0.02 * i as f64;
        let g = |c: f64, a: f64| a * (-0.5 * ((rt - c) / 0.05).powi(2)).exp();
        let comps = [
            (PARENT, g(8.0, 1.0e6 * decay)),
            (PARENT - 18.0, g(8.0, 1.2e5 * decay)),
            (PARENT + 16.0, g(7.2, 4.0e5 * (1.0 - decay))),
        ];
        let noise = 2e3 * (1.0 + ((i * 7919) % 13) as f64 / 13.0);
        let mut peaks: Vec<(f64, f32)> = vec![(150.7, noise as f32), (500.2, noise as f32)];
        peaks.extend(
            comps
                .iter()
                .filter(|c| c.1 > 100.0)
                .map(|c| (c.0 + 0.3, c.1 as f32)),
        );
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ p ESI Q1MS [100.000-600.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, &format!("{name}.mzML")).unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

#[test]
fn inherited_ion_scores_above_a_product() {
    let bytes: Vec<Vec<u8>> = TIMES.iter().map(|&t| index_of(t)).collect();
    let mut idxs: Vec<Index<Vec<u8>>> = bytes
        .iter()
        .map(|b| Index::open(b.clone(), b.len() as u64).unwrap())
        .collect();
    let mut samples: Vec<IsfSample<Vec<u8>>> = idxs
        .iter_mut()
        .zip(TIMES)
        .map(|(idx, t)| {
            let si = idx.section_of(1, None).unwrap();
            IsfSample {
                label: format!("t{t}"),
                time: Some(t),
                idx,
                si,
            }
        })
        .collect();
    let ions = [PARENT - 18.0 + 0.3, PARENT + 16.0 + 0.3, 900.0];
    let got = isf_eval(&mut samples, PARENT + 0.3, &ions, Some("C13H24N4O3S"));
    assert_eq!(got.len(), 3);
    let (frag, prod) = (
        got[0].as_ref().expect("fragment"),
        got[1].as_ref().expect("product"),
    );
    assert!(
        frag.p_inherited > prod.p_inherited,
        "{} vs {}",
        frag.p_inherited,
        prod.p_inherited
    );
    assert!(frag.calibrated && !frag.explanation.is_empty());
    assert!((frag.probs.iter().map(|p| p.1).sum::<f64>() - 1.0).abs() < 1e-9);
}
