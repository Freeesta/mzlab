//! The optical (DAD / PDA) matrix as mzML spectra: one `MS:1000804` spectrum per instant with a wavelength array in nm, the layout ProteoWizard writes
//! and `mzml_dad` reads. Used by `lettori_mzml::Conversione` when `Opzioni::dad` is on.

use std::io::Write;

use super::codifica::Data;
use super::{cv, esc, MzmlWriter};
use crate::error::Result;

impl<W: Write> MzmlWriter<W> {
    /// Appends the optical spectrum number `i` at `time_min` minutes: `wl` in nm and `ab` in mAU (32-bit, as the detector stores them).
    pub fn uv_spectrum(&mut self, i: usize, time_min: f64, wl: &[f64], ab: &[f32]) -> Result<()> {
        let id = format!("uv={i}");
        let mut s = std::mem::take(&mut self.text);
        s.clear();
        s += &format!(
            "<spectrum index=\"{}\" id=\"{}\" defaultArrayLength=\"{}\">",
            self.spectra,
            esc(&id),
            wl.len()
        );
        s += &cv("MS:1000804", "electromagnetic radiation spectrum", "");
        s += &format!("<cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{time_min}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>");
        s +=
            "<userParam name=\"intensity unit\" value=\"mAU\"/><binaryDataArrayList count=\"2\">\n";
        let res = self
            .enc
            .array(&mut s, "wavelength", "nm", Data::F64(wl))
            .and_then(|()| self.enc.array(&mut s, "intensity", "au", Data::F32(ab)));
        s += "</binaryDataArrayList></spectrum>\n";
        self.spectra += 1;
        self.mark(true, &id);
        let w = self.put(s.as_bytes());
        self.text = s;
        res.and(w)
    }
}
