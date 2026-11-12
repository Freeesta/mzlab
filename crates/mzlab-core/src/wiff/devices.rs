//! The real-time device traces of a `.wiff` sample (`Devices/Device_N/DevData`: column pressure, pump flow, oven temperature, ...) as chromatograms of the
//! reader: a pressure in pascal (`pressure array` in the mzML), every other trace with the unit the file names. The time of point k is k × the channel step
//! from the start of the acquisition: an assumption, the stream holds no time of its own (`docs/agenti/wiff-varianti.md`).

use super::dad::device_channels;
use crate::mzml::{ChromKind, Chromatogram};
use crate::ole::Ole;
use crate::pressure_unit::{is_pressure, pascal_per};

/// The traces with points; two channels of the same name get the number of their device after it. `index` is set where the list is built.
pub(super) fn traces(ole: &Ole, sample: u32) -> Vec<Chromatogram> {
    let mut out: Vec<Chromatogram> = Vec::new();
    for d in device_channels(ole, sample)
        .into_iter()
        .filter(|d| !d.values.is_empty())
    {
        let mut id = d.name;
        if out.iter().any(|c| c.id == id) {
            id.push_str(" #");
            id.push(char::from(b'0' + d.device_index as u8)); // devices 0..8
        }
        let (mut kind, mut unit, mut intensity) = (ChromKind::Analog, d.unit, d.values);
        if is_pressure(&id, &unit) {
            kind = ChromKind::Pressure;
            if let Some(k) = pascal_per(&unit) {
                intensity.iter_mut().for_each(|v| *v *= k);
                unit = "pascal".into();
            }
        }
        out.push(Chromatogram {
            index: 0,
            id,
            kind,
            q1: None,
            q3: None,
            unit,
            time: d.time_min,
            intensity,
        });
    }
    out
}
