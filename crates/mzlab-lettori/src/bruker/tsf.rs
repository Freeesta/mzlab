//! `analysis.tsf` + `analysis.tsf_bin` (timsTOF fleX MALDI, TIMS off): one block per frame at `Frames.TimsId`, `u32` size of the profile, `u32` size of the line spectrum, then the line
//! spectrum (zstd: `NumPeaks` `f64` TOF positions and as many `f32` intensities) and the profile (zstd: `DigitizerNumSamples` `u32` samples). Same tables and calibration as the TDF.

use super::blocco::zstd;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

fn guasto(m: &str) -> Error {
    Error::new("err.dam.unreadable").with("message", format!("tsf_bin: {m}"))
}

/// A frame as TOF positions and intensities; `profilo` when there was no line spectrum and the samples of the profile stand in.
pub struct Linee {
    pub tof: Vec<f64>,
    pub int: Vec<f64>,
    pub profilo: bool,
}

fn u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

pub fn leggi_frame(
    d: &mut dyn ReadAt,
    dim: u64,
    pos: u64,
    picchi: u64,
    campioni: u64,
    limiti: &Limiti,
) -> Result<Linee> {
    let mut h = [0u8; 8];
    if pos.checked_add(8).is_none_or(|e| e > dim) {
        return Err(guasto("block past the end of the file"));
    }
    d.read_at(pos, &mut h)?;
    let (prof, riga) = (u64::from(u32_le(&h)), u64::from(u32_le(&h[4..])));
    if prof.saturating_add(riga).saturating_add(pos + 8) > dim || prof.max(riga) > limiti.byte {
        return Err(guasto("block length"));
    }
    let leggi = |d: &mut dyn ReadAt, da: u64, n: u64| -> Result<Vec<u8>> {
        let mut b = vec![0u8; n as usize];
        d.read_at(da, &mut b)?;
        Ok(b)
    };
    if riga > 0 && picchi > 0 {
        let n = limiti.voci_ok(picchi)?;
        let z = leggi(d, pos + 8, riga)?;
        let b = zstd(
            &z,
            n.checked_mul(12).ok_or_else(|| guasto("peaks"))?,
            limiti,
        )?;
        let tof = (0..n)
            .map(|i| f64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap_or([0; 8])))
            .collect();
        let o = 8 * n;
        let int = (0..n)
            .map(|i| {
                f64::from(f32::from_le_bytes(
                    b[o + 4 * i..o + 4 * i + 4].try_into().unwrap_or([0; 4]),
                ))
            })
            .collect();
        return Ok(Linee {
            tof,
            int,
            profilo: false,
        });
    }
    if prof == 0 {
        return Ok(Linee {
            tof: Vec::new(),
            int: Vec::new(),
            profilo: false,
        });
    }
    let n = limiti.voci_ok(campioni)?;
    let z = leggi(d, pos + 8 + riga, prof)?;
    let b = zstd(
        &z,
        n.checked_mul(4).ok_or_else(|| guasto("samples"))?,
        limiti,
    )?;
    let v: Vec<u32> = b.chunks_exact(4).map(u32_le).collect();
    // only the samples with signal and their neighbours: the rest of the profile is zero
    let (mut tof, mut int) = (Vec::new(), Vec::new());
    for i in 0..v.len() {
        if v[i] > 0 || v.get(i + 1).is_some_and(|&x| x > 0) || (i > 0 && v[i - 1] > 0) {
            tof.push(i as f64);
            int.push(f64::from(v[i]));
        }
    }
    Ok(Linee {
        tof,
        int,
        profilo: true,
    })
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn blocco_oltre_la_fine() {
        let mut b = vec![0u8; 16];
        b[0] = 0xff;
        b[1] = 0xff;
        assert!(leggi_frame(&mut b.clone(), 16, 0, 1, 1, &Limiti::sito()).is_err());
    }
}
