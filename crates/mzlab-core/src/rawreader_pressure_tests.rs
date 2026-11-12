//! A pump pressure of a `.raw` in psi, bar or Pa goes in the mzML in pascal as a `pressure array` (MS:1000821, UO:0000110): the file passes the strict
//! check, reads back with the unit `pascal`, and the bar the page shows is the bar of the file (one conversion, never two).

use super::{in_pascal, tipo_analogico, ChromKind};
use crate::mzml;
use crate::mzml::Chromatogram;
use crate::mzmlwrite::{Header, MzmlWriter};

fn trace(unit: &str, v: f64) -> opentfraw::AnalogTrace {
    opentfraw::AnalogTrace {
        controller: 1,
        name: "Pump_Pressure".into(),
        unit: unit.into(),
        driver: String::new(),
        model: String::new(),
        serial: String::new(),
        time_min: vec![0.0, 0.5, 1.0],
        values: vec![v, v * 1.01, v * 0.99],
    }
}

#[test]
fn a_pressure_in_psi_bar_or_pa_round_trips_as_the_same_bar() {
    for (unit, v, bar) in [
        ("psi", 4000.0, 275.790_291_726_72),
        ("bar", 275.79, 275.79),
        ("Pa", 2.7579e7, 275.79),
    ] {
        let t = trace(unit, v);
        let kind = tipo_analogico(&t);
        assert_eq!(kind, ChromKind::Pressure);
        let t = in_pascal(kind, t);
        let h = Header {
            run: "x".into(),
            sources: vec!["x.raw".into()],
            software: ("mzLab".into(), "0".into()),
            ..Default::default()
        };
        let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
        w.open_spectra(0).unwrap();
        w.close_spectra().unwrap();
        w.open_chromatograms(1).unwrap();
        w.chromatogram(&Chromatogram {
            index: 0,
            id: t.name.clone(),
            kind,
            q1: None,
            q3: None,
            unit: t.unit.clone(),
            time: t.time_min.clone(),
            intensity: t.values.clone(),
        })
        .unwrap();
        w.close_chromatograms().unwrap();
        let out = w.finish().unwrap();
        crate::mzml_validate::check_written(&out[..]).unwrap();
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains(r#"accession="MS:1000821" name="pressure array" value="" unitCvRef="UO" unitAccession="UO:0000110" unitName="pascal""#), "{unit}");
        let run = mzml::read(&out[..]).unwrap();
        let c = run
            .chromatograms
            .iter()
            .find(|c| c.kind == mzml::ChromKind::Pressure)
            .unwrap();
        assert_eq!(c.unit, "pascal", "{unit}");
        let shown = crate::analog::pressure_to_bar(&c.intensity, &c.unit).unwrap();
        assert!(
            (shown[0] - bar).abs() <= bar * 1e-9,
            "{unit}: {} bar against {bar}",
            shown[0]
        );
    }
}

#[test]
fn another_channel_keeps_the_unit_of_the_file() {
    let t = trace("°C", 40.0);
    let kind = tipo_analogico(&t);
    let t = in_pascal(kind, t);
    assert_eq!((t.unit.as_str(), t.values[0]), ("°C", 40.0));
}
