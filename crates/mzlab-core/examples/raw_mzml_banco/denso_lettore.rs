//! The reader of the dense case: a `Lettore` that serves the generated packets by offset, decoded with the real decoders.
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Campione, Posizione, VoceScansione};
use mzlab_core::mzml::Spectrum;

/// The reader of the dense case: scans decoded from the file by offset, as `LettoreRaw` does from the `.raw`.
pub struct Denso {
    pub(crate) file: File,
    voci: Vec<VoceScansione>,
    campioni: Vec<Campione>,
    nome: String,
    buf: Vec<u8>,
    /// Trailer userParams per scan (the real trailer of a Fusion has about 130 fields).
    pub campi_trailer: usize,
}

/// The index entry of scan `k` from its 12-byte table row (offset, length).
fn voce(k: usize, c: &[u8]) -> VoceScansione {
    let offset = u64::from_le_bytes(c[..8].try_into().unwrap_or_default());
    let lunghezza = u32::from_le_bytes(c[8..].try_into().unwrap_or_default());
    VoceScansione {
        id: format!("controllerType=0 controllerNumber=1 scan={}", k + 1),
        rt: k as f64 * 0.0005,
        livello: 1,
        polarita: 1,
        tic: 1.0e9,
        picco_base: Some(500.0),
        filtro: "FTMS + p ESI Full ms [150.0000-2000.0000]".into(),
        precursore: None,
        posizione: Some(Posizione {
            membro: 0,
            offset,
            lunghezza,
        }),
    }
}

impl Denso {
    pub fn apri(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut n = [0u8; 8];
        file.read_exact_at(&mut n, 0)?;
        let n = u64::from_le_bytes(n) as usize;
        let mut t = vec![0u8; n * 12];
        file.read_exact_at(&mut t, 8)?;
        let voci = (0..n).map(|k| voce(k, &t[k * 12..k * 12 + 12])).collect();
        let nome = path
            .file_name()
            .map_or(String::new(), |s| s.to_string_lossy().into_owned());
        Ok(Denso {
            file,
            voci,
            campioni: vec![Campione {
                id: 1,
                nome: "denso".into(),
                scansioni: n,
            }],
            nome,
            buf: Vec::new(),
            campi_trailer: 130,
        })
    }

    fn leggi(&mut self, k: usize) -> Result<()> {
        let p = self
            .voci
            .get(k)
            .and_then(|v| v.posizione)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        self.buf.resize(p.lunghezza as usize, 0);
        self.file
            .read_exact_at(&mut self.buf, p.offset)
            .map_err(|_| Error::new("err.file.raw"))
    }

    fn spettro(&self, k: usize, profile: bool, mz: Vec<f64>, intensity: Vec<f32>) -> Spectrum {
        let v = &self.voci[k];
        Spectrum {
            index: k,
            id: v.id.clone(),
            level: v.livello,
            rt: v.rt,
            polarity: v.polarita,
            profile,
            filter: v.filtro.clone(),
            precursor: None,
            mz,
            intensity,
            wide: None,
        }
    }
}

#[path = "denso_serve.rs"]
mod serve;
