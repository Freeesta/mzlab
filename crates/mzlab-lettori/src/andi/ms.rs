//! An ANDI-MS file (ASTM E2077, netCDF-3, `.cdf`) as a `Lettore`: the scans of `scan_index`/`point_count` over `mass_values` and `intensity_values`, the TIC of `total_intensity`, the header
//! of the file as `metodo/1`. A file with `ordinate_values` and no `mass_values` is ANDI-Chromatography (ASTM E1947): one sample without scans and one analog chromatogram (`cromato.rs`).
//! Written from the two ASTM specifications and the netCDF documentation, checked against files exported by ChemStation and MassHunter.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Posizione, VoceCromatogramma,
    VoceLettore, VoceScansione, CONTRATTO,
};
use mzlab_core::metodo::sha1_hex;
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

use super::cromato;
use super::netcdf::{Cdf, Var};
use super::testata::Testata;

/// The bytes of the start of the file that `riconosci` looks at: the names of the variables are in the header.
const TESTA: usize = 1 << 18;
const UNITA: &str = "Arbitrary Intensity Units";

pub const VOCE: VoceLettore = VoceLettore {
    id: "andi-ms",
    contratto: CONTRATTO,
    estensioni: &[".cdf", ".nc"],
    riconosci,
    apri,
};

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.trova(".cdf").or_else(|| fs.trova(".nc")) else {
        return Fiducia::No;
    };
    let head = fs.testa(i, TESTA);
    let ha = |n: &[u8]| head.windows(n.len()).any(|w| w == n);
    if head.starts_with(b"\x89HDF") {
        Fiducia::Forse
    } else if !head.starts_with(b"CDF") {
        Fiducia::No
    } else if ha(b"mass_values") {
        Fiducia::Si
    } else if ha(b"ordinate_values") {
        Fiducia::Forse
    } else {
        Fiducia::No
    }
}

pub struct LettoreAndi {
    dati: Box<dyn ReadAt>,
    dim: u64,
    nome: String,
    limiti: Limiti,
    cdf: Cdf,
    testata: Testata,
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    /// First point and number of points of each scan.
    inizi: Vec<u64>,
    conti: Vec<u64>,
    /// The mass range of each scan, when the file says it.
    finestre: Vec<Option<(f64, f64)>>,
    /// Whether the TIC of the scans is that of the file (else it is summed from the points when asked).
    tic_noto: bool,
    profilo: bool,
    sim: bool,
    sha1: Option<Option<String>>,
}

/// A whole variable of the scans as numbers (`None`: the file has no such variable).
fn colonna(
    cdf: &Cdf,
    dati: &mut dyn ReadAt,
    limiti: &Limiti,
    nome: &str,
) -> Result<Option<Vec<f64>>> {
    let Some(v) = cdf.trova(nome) else {
        return Ok(None);
    };
    cdf.leggi(dati, limiti, v, 0, cdf.elementi(v)).map(Some)
}

fn scala(x: &mut [f64], v: &Var) {
    let (f, o) = (
        v.numero("scale_factor").unwrap_or(1.0),
        v.numero("add_offset").unwrap_or(0.0),
    );
    if f != 1.0 || o != 0.0 {
        x.iter_mut().for_each(|y| *y = *y * f + o);
    }
}

fn indice_cattivo(k: usize) -> Error {
    Error::new("error.badIndex").with("k", k)
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let i = fs
        .trova(".cdf")
        .or_else(|| fs.trova(".nc"))
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let m = fs
        .membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let nome = m
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.cdf")
        .to_string();
    let dim = m.dim;
    let mut dati: Box<dyn ReadAt> = std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new()));
    let cdf = Cdf::apri(&mut dati, dim, limiti)?;
    let testata = Testata::leggi(&cdf, &mut dati, limiti)?;
    let mut l = LettoreAndi {
        dati,
        dim,
        nome,
        limiti: *limiti,
        cdf,
        testata,
        campioni: Vec::new(),
        indice: Vec::new(),
        inizi: Vec::new(),
        conti: Vec::new(),
        finestre: Vec::new(),
        tic_noto: false,
        profilo: false,
        sim: false,
        sha1: None,
    };
    let scansioni = if l.cdf.trova("mass_values").is_some() {
        l.scansioni()?
    } else if l.cdf.trova("ordinate_values").is_some() {
        0
    } else {
        return Err(Error::new("err.file.notAccepted").with("name", l.nome));
    };
    let stem = l
        .nome
        .rsplit_once('.')
        .map_or(l.nome.as_str(), |(s, _)| s)
        .to_string();
    l.campioni = vec![Campione {
        id: 1,
        nome: l.testata.titolo.clone().unwrap_or(stem),
        scansioni,
    }];
    Ok(Box::new(l))
}

impl LettoreAndi {
    /// Reads the per-scan variables and builds the index.
    fn scansioni(&mut self) -> Result<usize> {
        let (cdf, limiti) = (&self.cdf, self.limiti);
        let sx = cdf
            .trova("scan_index")
            .ok_or_else(|| Error::new("err.dam.unreadable").with("message", "no scan_index"))?;
        let n = limiti.voci_ok(cdf.elementi(sx))?;
        let punti = cdf.trova("mass_values").map_or(0, |v| cdf.elementi(v));
        let inizi: Vec<u64> = cdf
            .leggi(&mut self.dati, &limiti, sx, 0, n as u64)?
            .iter()
            .map(|&x| x.max(0.0) as u64)
            .collect();
        self.conti = match colonna(cdf, &mut self.dati, &limiti, "point_count")? {
            Some(c) => (0..n)
                .map(|k| c.get(k).map_or(0, |&x| x.max(0.0) as u64))
                .collect(),
            None => (0..n)
                .map(|k| {
                    inizi
                        .get(k + 1)
                        .copied()
                        .unwrap_or(punti)
                        .saturating_sub(inizi[k])
                })
                .collect(),
        };
        self.inizi = inizi;
        let col = |l: &mut Self, nome: &str| colonna(&l.cdf, &mut l.dati, &l.limiti, nome);
        let tempo = col(self, "scan_acquisition_time")?;
        let tot = col(self, "total_intensity")?;
        let (lo, hi) = (col(self, "mass_range_min")?, col(self, "mass_range_max")?);
        self.tic_noto = tot.is_some();
        self.sim = [
            self.cdf.testo("test_scan_function"),
            self.cdf.testo("experiment_type"),
        ]
        .iter()
        .flatten()
        .any(|t| t.to_ascii_lowercase().contains("selected ion") || t.contains("SIM"));
        self.profilo = self
            .cdf
            .testo("experiment_type")
            .is_some_and(|t| t.to_ascii_lowercase().contains("continuum"));
        let pol = match self
            .cdf
            .testo("test_ionization_polarity")
            .map(str::to_ascii_lowercase)
        {
            Some(p) if p.contains("pos") => 1,
            Some(p) if p.contains("neg") => -1,
            _ => 0,
        };
        for k in 0..n {
            let rt = match &tempo {
                Some(t) => t.get(k).copied(),
                None => self.tempo_punto(k)?,
            };
            let v = |c: &Option<Vec<f64>>| c.as_ref().and_then(|c| c.get(k).copied());
            self.finestre.push(v(&lo).zip(v(&hi)));
            self.indice.push(VoceScansione {
                id: format!("scan={}", k + 1),
                rt: rt.unwrap_or(0.0) / 60.0,
                livello: 1,
                polarita: pol,
                tic: v(&tot).unwrap_or(0.0),
                picco_base: None,
                filtro: if self.sim {
                    "SIM".into()
                } else {
                    String::new()
                },
                precursore: None,
                posizione: Some(Posizione {
                    membro: 0,
                    offset: self.inizi[k],
                    lunghezza: u32::try_from(self.conti[k]).unwrap_or(u32::MAX),
                }),
            });
        }
        Ok(n)
    }

    /// The time (seconds) of the first point of scan `k` from `time_values`, when the file has no `scan_acquisition_time`.
    fn tempo_punto(&mut self, k: usize) -> Result<Option<f64>> {
        let Some(v) = self.cdf.trova("time_values") else {
            return Ok(None);
        };
        if self.inizi[k] >= self.cdf.elementi(v) {
            return Ok(None);
        }
        Ok(self
            .cdf
            .leggi(&mut self.dati, &self.limiti, v, self.inizi[k], 1)?
            .first()
            .copied())
    }

    fn uno(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// Scan `k`: its points, sorted by m/z.
    fn spettro(&mut self, k: usize) -> Result<Spectrum> {
        let v = self
            .indice
            .get(k)
            .cloned()
            .ok_or_else(|| indice_cattivo(k))?;
        let (da, n) = (self.inizi[k], self.conti[k]);
        let (m, y) = (
            self.cdf.trova("mass_values"),
            self.cdf.trova("intensity_values"),
        );
        let (Some(m), Some(y)) = (m, y) else {
            return Err(Error::new("err.dam.unreadable").with("message", "no intensity_values"));
        };
        let mut mz = self.cdf.leggi(&mut self.dati, &self.limiti, m, da, n)?;
        let mut it = self.cdf.leggi(&mut self.dati, &self.limiti, y, da, n)?;
        scala(&mut mz, m);
        scala(&mut it, y);
        if mz.windows(2).any(|w| w[0] > w[1]) {
            let mut p: Vec<(f64, f64)> = mz.iter().copied().zip(it.iter().copied()).collect();
            p.sort_by(|a, b| a.0.total_cmp(&b.0));
            (mz, it) = p.into_iter().unzip();
        }
        let largo = matches!(
            y.tipo,
            super::netcdf::Tipo::Double | super::netcdf::Tipo::Int
        );
        Ok(Spectrum {
            index: k,
            id: v.id,
            level: 1,
            rt: v.rt,
            polarity: v.polarita,
            profile: self.profilo,
            filter: v.filtro,
            precursor: None,
            intensity: it.iter().map(|&x| x as f32).collect(),
            wide: largo.then(|| it.clone()),
            mz,
        })
    }

    /// The TIC of the scans whose file has no `total_intensity`, summed from their points.
    fn completa(&mut self) -> Result<()> {
        if self.tic_noto {
            return Ok(());
        }
        for k in 0..self.indice.len() {
            let s = self.spettro(k)?;
            self.indice[k].tic = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
        }
        self.tic_noto = true;
        Ok(())
    }

    fn traccia(&mut self, id: &str) -> Result<Chromatogram> {
        let (tic, kind, index) = match id {
            "TIC" => (true, ChromKind::Tic, 0),
            "BPC" => (false, ChromKind::Bpc, 1),
            _ => return Err(Error::new("error.badIndex").with("id", id)),
        };
        self.completa()?;
        let intensity = if tic {
            self.indice.iter().map(|v| v.tic).collect()
        } else {
            let mut o = Vec::new();
            for k in 0..self.indice.len() {
                let s = self.spettro(k)?;
                o.push(
                    (0..s.mz.len())
                        .map(|i| s.intensity_f64(i))
                        .fold(0.0, f64::max),
                );
            }
            o
        };
        Ok(Chromatogram {
            index,
            id: id.into(),
            kind,
            q1: None,
            q3: None,
            unit: UNITA.into(),
            time: self.indice.iter().map(|v| v.rt).collect(),
            intensity,
        })
    }

    fn sha1(&mut self) -> Option<String> {
        if self.sha1.is_none() {
            let dim = self.dim;
            self.sha1 = Some(sha1_hex(&mut self.dati, dim).ok());
        }
        self.sha1.clone().flatten()
    }
}

impl Lettore for LettoreAndi {
    fn formato(&self) -> Formato {
        Formato::Altro("andi-ms")
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.uno(c)?;
        self.completa()?;
        Ok(&self.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.uno(c)?;
        self.spettro(k)
    }

    /// Every per-scan variable of the file (times, ranges, counts) at scan `k`, with its unit.
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.uno(c)?;
        if k >= self.indice.len() {
            return Err(indice_cattivo(k));
        }
        let Some(sx) = self.cdf.trova("scan_index") else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for v in self
            .cdf
            .var
            .iter()
            .filter(|v| v.dims == sx.dims && v.tipo != super::netcdf::Tipo::Char)
        {
            let x = self
                .cdf
                .leggi(&mut self.dati, &self.limiti, v, k as u64, 1)?;
            out.push(Campo {
                nome: v.nome.clone(),
                valore: x.first().map(|x| x.to_string()).unwrap_or_default(),
                unita: v
                    .testo("units")
                    .map(str::to_string)
                    .filter(|u| !u.is_empty()),
                cv: None,
            });
        }
        Ok(out)
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.uno(c)?;
        if self.indice.is_empty() {
            return Ok(cromato::voci(&self.cdf, &mut self.dati, &self.limiti));
        }
        Ok(["TIC", "BPC"]
            .into_iter()
            .zip([ChromKind::Tic, ChromKind::Bpc])
            .map(|(id, tipo)| VoceCromatogramma {
                id: id.into(),
                tipo,
                unita: UNITA.into(),
            })
            .collect())
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.uno(c)?;
        if self.indice.is_empty() {
            return cromato::traccia(&self.cdf, &mut self.dati, &self.limiti, id);
        }
        self.traccia(id)
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        self.uno(c)?;
        let sha1 = self.sha1();
        Ok(self.testata.documento(&self.nome, self.dim, sha1))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.uno(c)?;
        let sha1 = self.sha1();
        let doc = self.testata.documento(&self.nome, self.dim, sha1.clone());
        Ok(Some(self.testata.header(&self.nome, sha1, doc)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        SpectrumExtra {
            sim: self.sim,
            observed: false,
            window: self.finestre.get(s.index).copied().flatten(),
            params: Vec::new(),
        }
    }
}
