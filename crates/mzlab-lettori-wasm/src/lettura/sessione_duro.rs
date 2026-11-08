// A reader whose answers hold the awkward values (control characters, NaN, infinities, `-0.0`, numbers at both ends of the range), for the tests of sessione.rs.
use super::*;
use mzlab_core::lettori::{Campione, Campo, Formato, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};

pub const ODD_CHARS: &str = "a\0b\u{1}\u{1f}\u{7f}\"\\/\n\r\t\u{8}\u{c}é€😀\u{2028}";

impl Lettore for Hard {
    fn format(&self) -> Formato {
        Formato::Altro("duro")
    }
    fn samples(&self) -> &[Campione] {
        static C: std::sync::OnceLock<Vec<Campione>> = std::sync::OnceLock::new();
        C.get_or_init(|| {
            vec![
                Campione {
                    id: 0,
                    nome: ODD_CHARS.into(),
                    scansioni: 0,
                    dimensione: None,
                },
                Campione {
                    id: u32::MAX,
                    nome: String::new(),
                    scansioni: usize::MAX >> 12,
                    dimensione: Some(1),
                },
            ]
        })
    }
    fn index(&mut self, _: usize) -> Result<&[VoceScansione]> {
        Ok(&[])
    }
    fn scan(&mut self, _: usize, _: usize) -> Result<Spectrum> {
        Err(mzlab_core::Error::new("error.badIndex"))
    }
    fn scan_header(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
        Ok(vec![
            Campo {
                nome: ODD_CHARS.into(),
                valore: ODD_CHARS.into(),
                unita: Some(ODD_CHARS.into()),
                cv: None,
            },
            Campo {
                nome: String::new(),
                valore: String::new(),
                unita: None,
                cv: Some(String::new()),
            },
        ])
    }
    fn chromatograms(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok([ChromKind::Tic, ChromKind::Srm, ChromKind::Analog]
            .iter()
            .map(|&tipo| VoceCromatogramma {
                id: ODD_CHARS.into(),
                tipo,
                unita: ODD_CHARS.into(),
            })
            .collect())
    }
    fn chromatogram(&mut self, _: usize, id: &str) -> Result<Chromatogram> {
        let n = numbers();
        Ok(Chromatogram {
            index: 3,
            id: id.into(),
            kind: ChromKind::Srm,
            q1: Some(0.1 + 0.2),
            q3: Some(f64::NAN),
            unit: ODD_CHARS.into(),
            time: n.clone(),
            intensity: n.iter().rev().copied().collect(),
        })
    }
    fn method(&mut self, _: usize) -> Result<String> {
        Ok("{}".into())
    }
    fn mzml_header(&mut self, c: usize) -> Result<Option<mzlab_core::mzmlwrite::Header>> {
        let t = |x: &str| x.to_string();
        Ok((c == 0).then(|| mzlab_core::mzmlwrite::Header {
            run: t(ODD_CHARS),
            sources: vec![t(ODD_CHARS), t("")],
            instrument: Some((t("MS:1"), t(ODD_CHARS))),
            components: vec![(t("source"), t("MS:2"), t(ODD_CHARS))],
            software: (t(ODD_CHARS), t("1.0")),
            acquired: Some(t("2026-10-29T10:00:00+01:00")),
            serial: None,
            source_sha1: vec![t("00ff")],
            source_cv: vec![(t("MS:3"), t(ODD_CHARS))],
            sample: Some((t("s1"), t(ODD_CHARS))),
            run_params: vec![(t(ODD_CHARS), t(ODD_CHARS))],
            content: vec![(t("MS:4"), t("n"))],
            extra_software: vec![(t("MS:5"), t("n"), t("v"))],
            centroided: true,
            ..Default::default()
        }))
    }
}

pub struct Hard;

pub fn numbers() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        0.1,
        0.1 + 0.2,
        1e-7,
        1e21,
        1e300,
        5e-324,
        f64::MIN_POSITIVE,
        -2.5,
        123456789.123456,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    v.extend((0..5000).map(|i| (i as f64).sqrt() * 1e-3 + 1.0 / 3.0));
    v
}
