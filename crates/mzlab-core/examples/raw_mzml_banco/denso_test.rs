//! Tests of the dense case: the real decoders re-read the generated values bit for bit, the generator is deterministic, the reader serves the scans.
use crate::denso::*;
use crate::denso_lettore::Denso;
use mzlab_core::lettori::Lettore;
use opentfraw::pacchetto::{decodifica, decodifica_picchi};

const PICCOLO: Parametri = Parametri {
    scansioni: 3,
    picchi: 50,
    seme: 7,
    con_profilo: true,
    bin: (8, 12),
};

fn bits(v: &[(f64, f32)]) -> Vec<(u64, u32)> {
    v.iter().map(|(m, a)| (m.to_bits(), a.to_bits())).collect()
}

/// The decoders of the real reader give back, bit for bit, what the generator wrote (profile, centroids, resolutions, nodes) for 3 scans.
#[test]
fn il_decoder_rilegge_i_valori_generati_al_bit() {
    for k in 0..3 {
        let (mut b, mut v) = (Vec::new(), Verita::default());
        pacchetto(&mut b, &PICCOLO, k, Some(&mut v));
        let pk = decodifica(&b).unwrap();
        let profilo = pk.profile.as_ref().unwrap();
        assert_eq!(profilo.first_value.to_bits(), F0.to_bits());
        assert_eq!(profilo.step.to_bits(), PASSO.to_bits());
        let letti: Vec<(u32, f32, f32)> = profilo
            .chunks
            .iter()
            .flat_map(|c| {
                c.signal
                    .iter()
                    .enumerate()
                    .map(|(j, s)| (c.first_bin + j as u32, *s, c.fudge.unwrap()))
            })
            .collect();
        assert_eq!(letti.len(), v.profilo.len());
        for (a, b) in letti.iter().zip(&v.profilo) {
            assert_eq!(
                (a.0, a.1.to_bits(), a.2.to_bits()),
                (b.0, b.1.to_bits(), b.2.to_bits())
            );
        }
        let picchi: Vec<(f64, f32)> = pk.peaks.iter().map(|p| (p.mz, p.abundance)).collect();
        assert_eq!(bits(&picchi), bits(&v.picchi));
        assert_eq!(
            pk.resolutions
                .iter()
                .map(|r| r.to_bits())
                .collect::<Vec<_>>(),
            v.risoluzioni
                .iter()
                .map(|r| r.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(pk.noise_nodes.len(), NODI);
        // the list-only entry point gives the same peaks
        let solo = decodifica_picchi(&b).unwrap();
        assert_eq!(
            bits(&solo.iter().map(|p| (p.mz, p.abundance)).collect::<Vec<_>>()),
            bits(&v.picchi)
        );
    }
}

/// The same parameters give the same file: the bytes of the 3-scan case hash to a fixed value.
#[test]
fn il_generatore_da_lo_stesso_sha256_a_ogni_giro() {
    let dir = std::env::temp_dir().join(format!("raw_mzml_denso_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("a.bin"), dir.join("b.bin"));
    let (_, _, sa) = genera(&a, &PICCOLO, true).unwrap();
    let (_, _, sb) = genera(&b, &PICCOLO, true).unwrap();
    assert_eq!(sa, sb);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    assert_eq!(sa.as_deref(), Some(SHA_ATTESO));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The `Lettore` serves the scans of the file with the values generated, centroids and profile points (m/z converted from the bins).
#[test]
fn il_lettore_finto_serve_le_scansioni_generate() {
    let dir = std::env::temp_dir().join(format!("raw_mzml_denso_l_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("d.bin");
    genera(&f, &PICCOLO, false).unwrap();
    let mut l = Denso::apri(&f).unwrap();
    assert_eq!(l.indice(0).unwrap().len(), 3);
    for k in 0..3 {
        let (mut b, mut v) = (Vec::new(), Verita::default());
        pacchetto(&mut b, &PICCOLO, k, Some(&mut v));
        let s = l.scansione(0, k).unwrap();
        assert!(!s.profile);
        assert_eq!(
            bits(
                &s.mz
                    .iter()
                    .copied()
                    .zip(s.intensity.iter().copied())
                    .collect::<Vec<_>>()
            ),
            bits(&v.picchi)
        );
        let p = l.scansione_profilo(0, k).unwrap().unwrap();
        assert!(p.profile);
        assert_eq!(p.mz.len(), v.profilo.len());
        // the frequency step is negative, so m/z grows with the bin
        assert!(
            p.mz.windows(2).all(|w| w[0] < w[1]),
            "m/z must grow with the bin"
        );
        let atteso: Vec<u32> = v.profilo.iter().map(|x| x.1.to_bits()).collect();
        assert_eq!(
            p.intensity.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            atteso
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The full-size case reaches the density the criteria ask for.
#[test]
fn il_caso_denso_ha_almeno_200000_punti_per_scansione() {
    let (mut b, mut v) = (Vec::new(), Verita::default());
    pacchetto(&mut b, &Parametri::denso(1), 0, Some(&mut v));
    assert!(v.profilo.len() >= 200_000, "{}", v.profilo.len());
}

const SHA_ATTESO: &str = "5ae7271d7a6da6693ae677ce9593b7cdcd75a5469314b719989e6b45e009e182";
