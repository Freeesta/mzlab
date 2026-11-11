//! An ANDI-MS file (ASTM E2077, netCDF-3, `.cdf`) as a `Lettore`: the scans of `scan_index`/`point_count` over `mass_values` and `intensity_values`, the TIC of `total_intensity`, the header
//! of the file as `metodo/1`. A file with `ordinate_values` and no `mass_values` is ANDI-Chromatography (ASTM E1947): one sample without scans and one analog chromatogram (`chromato.rs`).
//! Written from the two ASTM specifications and the netCDF documentation, checked against files exported by ChemStation and MassHunter.

use super::chromato;
use super::header::AndiHeader;
use super::netcdf::{broken, scale, Cdf};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::method::sha1_hex;
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::readers::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Position, Reader, ReaderEntry,
    Sample, ScanEntry, CONTRATTO,
};

mod bpc;

/// The bytes of the start of the file that `riconosci` looks at: the names of the variables are in the header.
const HEAD: usize = 1 << 18;
const UNITS: &str = "Arbitrary Intensity Units";

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "andi-ms",
    contratto: CONTRATTO,
    estensioni: &[".cdf", ".nc"],
    riconosci: recognize,
    apri: open,
};

impl AndiReader {
    /// Reads the per-scan variables and builds the index.
    /// First point and number of points of each scan: `scan_index` and `point_count`.
    fn points_per_scan(&mut self) -> Result<usize> {
        let (cdf, limits) = (&self.cdf, self.limits);
        let sx = cdf
            .find("scan_index")
            .ok_or_else(|| broken("no scan_index"))?;
        let n = cdf.count(sx, &limits)?;
        let points = cdf.find("mass_values").map_or(0, |v| cdf.elements(v));
        let starts: Vec<u64> = cdf
            .read(&mut self.data, &limits, sx, 0, n as u64)?
            .iter()
            .map(|&x| x.max(0.0) as u64)
            .collect();
        self.counts = match column(cdf, &mut self.data, &limits, "point_count")? {
            Some(c) => (0..n)
                .map(|k| c.get(k).map_or(0, |&x| x.max(0.0) as u64))
                .collect(),
            None => (0..n)
                .map(|k| {
                    starts
                        .get(k + 1)
                        .copied()
                        .unwrap_or(points)
                        .saturating_sub(starts[k])
                })
                .collect(),
        };
        self.inizi = starts;
        Ok(n)
    }

    /// The traits the whole file has (SIM, profile) and its polarity.
    fn stretches(&mut self) -> i8 {
        self.sim = [
            self.cdf.text("test_scan_function"),
            self.cdf.text("experiment_type"),
        ]
        .iter()
        .flatten()
        .any(|t| t.to_ascii_lowercase().contains("selected ion") || t.contains("SIM"));
        self.profilo = self
            .cdf
            .text("experiment_type")
            .is_some_and(|t| t.to_ascii_lowercase().contains("continuum"));

        match self
            .cdf
            .text("test_ionization_polarity")
            .map(str::to_ascii_lowercase)
        {
            Some(p) if p.contains("pos") => 1,
            Some(p) if p.contains("neg") => -1,
            _ => 0,
        }
    }

    fn scans(&mut self) -> Result<usize> {
        let all_of_them = self.points_per_scan()?;
        // a copy cut short loses the points of the last scans: the index stops before the first scan not whole in the file, and `missing` counts the rest
        let n = (0..all_of_them)
            .find(|&k| !self.whole(k))
            .unwrap_or(all_of_them);
        self.cut = all_of_them - n;
        let col = |l: &mut Self, name: &str| column(&l.cdf, &mut l.data, &l.limits, name);
        let moment = col(self, "scan_acquisition_time")?;
        let tot = col(self, "total_intensity")?;
        let (lo, hi) = (col(self, "mass_range_min")?, col(self, "mass_range_max")?);
        self.known_tic = tot.is_some();
        let pol = self.stretches();
        for k in 0..n {
            let rt = match &moment {
                Some(t) => t.get(k).copied(),
                None => self.point_time(k)?,
            };
            let v = |c: &Option<Vec<f64>>| c.as_ref().and_then(|c| c.get(k).copied());
            self.windows.push(v(&lo).zip(v(&hi)));
            self.indice.push(ScanEntry {
                id: format!("scan={}", k + 1),
                rt: rt.unwrap_or(0.0) / 60.0,
                livello: 1,
                polarita: pol,
                tic: v(&tot).unwrap_or(0.0),
                base_peak: None,
                filtro: if self.sim {
                    "SIM".into()
                } else {
                    String::new()
                },
                precursore: None,
                position: Some(Position {
                    membro: 0,
                    offset: self.inizi[k],
                    lunghezza: u32::try_from(self.counts[k]).unwrap_or(u32::MAX),
                }),
            });
        }
        Ok(n)
    }

    /// The m/z and intensities of scan `k` lie whole in the file.
    fn whole(&self, k: usize) -> bool {
        let (da, n) = (self.inizi[k], self.counts[k]);
        ["mass_values", "intensity_values"]
            .iter()
            .all(|name| self.cdf.find(name).is_none_or(|v| self.cdf.whole(v, da, n)))
    }

    /// The time (seconds) of the first point of scan `k` from `time_values`, when the file has no `scan_acquisition_time`.
    fn point_time(&mut self, k: usize) -> Result<Option<f64>> {
        let Some(v) = self.cdf.find("time_values") else {
            return Ok(None);
        };
        if self.inizi[k] >= self.cdf.elements(v) {
            return Ok(None);
        }
        let mut x = self
            .cdf
            .read(&mut self.data, &self.limits, v, self.inizi[k], 1)?;
        scale(&mut x, v);
        Ok(x.first().copied())
    }

    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The points of scan `k` as `(m/z, intensity, whether the intensity type is wider than f32)`, sorted by m/z (stable: points with the same m/z keep the file's order).
    fn points_of(&mut self, k: usize) -> Result<(Vec<f64>, Vec<f64>, bool)> {
        let (da, n) = (
            *self.inizi.get(k).ok_or_else(|| bad_index(k))?,
            self.counts[k],
        );
        let (m, y) = (
            self.cdf.find("mass_values"),
            self.cdf.find("intensity_values"),
        );
        let (Some(m), Some(y)) = (m, y) else {
            return Err(broken("no intensity_values"));
        };
        let mut mz = self.cdf.read(&mut self.data, &self.limits, m, da, n)?;
        let mut it = self.cdf.read(&mut self.data, &self.limits, y, da, n)?;
        scale(&mut mz, m);
        scale(&mut it, y);
        if mz.windows(2).any(|w| w[0] > w[1]) {
            let mut p: Vec<(f64, f64)> = mz.iter().copied().zip(it.iter().copied()).collect();
            p.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (i, (x, y)) in p.into_iter().enumerate() {
                (mz[i], it[i]) = (x, y);
            }
        }
        let broad = matches!(
            y.tipo,
            super::netcdf::Kind::Double | super::netcdf::Kind::Int
        );
        Ok((mz, it, broad))
    }

    /// Scan `k`: its points, sorted by m/z.
    fn spectrum(&mut self, k: usize) -> Result<Spectrum> {
        let v = self.indice.get(k).cloned().ok_or_else(|| bad_index(k))?;
        let (mz, it, broad) = self.points_of(k)?;
        self.bpc_note(k, &it, broad);
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
            wide: broad.then_some(it),
            mz,
        })
    }

    /// The TIC of the scans whose file has no `total_intensity`, summed from their points.
    fn complete(&mut self) -> Result<()> {
        if self.known_tic {
            return Ok(());
        }
        for k in 0..self.indice.len() {
            let (_, it, broad) = self.points_of(k)?;
            self.bpc_note(k, &it, broad);
            // the sum of what `Spectrum::intensity_f64` gives: the narrow types are summed as f32
            self.indice[k].tic = it
                .iter()
                .map(|&x| if broad { x } else { f64::from(x as f32) })
                .sum();
        }
        self.known_tic = true;
        Ok(())
    }

    fn traccia(&mut self, id: &str) -> Result<Chromatogram> {
        let (tic, kind, index) = match id {
            "TIC" => (true, ChromKind::Tic, 0),
            "BPC" => (false, ChromKind::Bpc, 1),
            _ => return Err(Error::new("error.badIndex").with("id", id)),
        };
        self.complete()?;
        let intensity = if tic {
            self.indice.iter().map(|v| v.tic).collect()
        } else {
            self.bpc_points()?
        };
        Ok(Chromatogram {
            index,
            id: id.into(),
            kind,
            q1: None,
            q3: None,
            unit: UNITS.into(),
            time: self.indice.iter().map(|v| v.rt).collect(),
            intensity,
        })
    }

    fn sha1(&mut self) -> Option<String> {
        if self.sha1.is_none() {
            let dim = self.dim;
            self.sha1 = Some(sha1_hex(&mut self.data, dim).ok());
        }
        self.sha1.clone().flatten()
    }
}

impl Reader for AndiReader {
    fn format(&self) -> Format {
        Format::Altro("andi-ms")
    }

    fn missing(&mut self, c: usize) -> usize {
        if c == 0 {
            self.cut
        } else {
            0
        }
    }

    fn samples(&self) -> &[Sample] {
        &self.samples
    }

    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.one(c)?;
        self.complete()?;
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
        self.spectrum(k)
    }

    /// Every per-scan variable of the file (times, ranges, counts) at scan `k`, with its unit.
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.one(c)?;
        if k >= self.indice.len() {
            return Err(bad_index(k));
        }
        let Some(sx) = self.cdf.find("scan_index") else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for v in self
            .cdf
            .var
            .iter()
            .filter(|v| v.dims == sx.dims && v.tipo != super::netcdf::Kind::Char)
        {
            let mut x = self
                .cdf
                .read(&mut self.data, &self.limits, v, k as u64, 1)?;
            scale(&mut x, v);
            out.push(Field {
                nome: v.nome.clone(),
                valore: x.first().map(|x| x.to_string()).unwrap_or_default(),
                unita: v
                    .text("units")
                    .map(str::to_string)
                    .filter(|u| !u.is_empty()),
                cv: None,
            });
        }
        Ok(out)
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        self.one(c)?;
        if self.indice.is_empty() && self.cut == 0 {
            return Ok(chromato::entries(&self.cdf, &mut self.data, &self.limits));
        }
        Ok(["TIC", "BPC"]
            .into_iter()
            .zip([ChromKind::Tic, ChromKind::Bpc])
            .map(|(id, kind)| ChromatogramEntry {
                id: id.into(),
                tipo: kind,
                unita: UNITS.into(),
            })
            .collect())
    }

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        if self.indice.is_empty() && self.cut == 0 {
            return chromato::trace(&self.cdf, &mut self.data, &self.limits, id);
        }
        self.traccia(id)
    }

    fn method(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        let sha1 = self.sha1();
        Ok(self.header.document(&self.name, self.dim, sha1))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        let sha1 = self.sha1();
        let doc = self.header.document(&self.name, self.dim, sha1.clone());
        Ok(Some(self.header.header(&self.name, sha1, doc)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        SpectrumExtra {
            sim: self.sim,
            observed: false,
            window: self.windows.get(s.index).copied().flatten(),
            ..SpectrumExtra::default()
        }
    }
}

pub struct AndiReader {
    data: Box<dyn ReadAt>,
    dim: u64,
    name: String,
    limits: Limits,
    cdf: Cdf,
    header: AndiHeader,
    samples: Vec<Sample>,
    indice: Vec<ScanEntry>,
    /// First point and number of points of each scan.
    inizi: Vec<u64>,
    counts: Vec<u64>,
    /// The mass range of each scan, when the file says it.
    windows: Vec<Option<(f64, f64)>>,
    /// Whether the TIC of the scans is that of the file (else it is summed from the points when asked).
    known_tic: bool,
    /// The base peak of each scan, noted at each decode (NaN: not yet); `bpc_visti` counts the noted ones.
    bpc: Vec<f64>,
    bpc_seen: usize,
    /// Scans listed by the file whose points lie past the end of a copy cut short.
    cut: usize,
    profilo: bool,
    sim: bool,
    sha1: Option<Option<String>>,
}

fn recognize(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.find(".cdf").or_else(|| fs.find(".nc")) else {
        return Fiducia::No;
    };
    let head = fs.testa(i, HEAD);
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

/// A whole variable of the scans as numbers, `scale_factor`/`add_offset` applied (`None`: the file has no such variable).
fn column(
    cdf: &Cdf,
    data: &mut dyn ReadAt,
    limits: &Limits,
    name: &str,
) -> Result<Option<Vec<f64>>> {
    let Some(v) = cdf.find(name) else {
        return Ok(None);
    };
    let mut x = cdf.read(data, limits, v, 0, cdf.elements(v))?;
    scale(&mut x, v);
    Ok(Some(x))
}

fn bad_index(k: usize) -> Error {
    Error::new("error.badIndex").with("k", k)
}

fn open(mut fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let i = fs
        .find(".cdf")
        .or_else(|| fs.find(".nc"))
        .ok_or_else(|| fs.not_accepted())?;
    let refused = fs.not_accepted();
    let m = fs.membri.get_mut(i).ok_or(refused)?;
    let nome = m
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.cdf")
        .to_string();
    let dim = m.dim;
    let mut dati: Box<dyn ReadAt> = std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new()));
    let cdf = Cdf::apri(&mut dati, dim, limits)?;
    let testata = AndiHeader::read(&cdf, &mut dati, limits)?;
    let mut l = AndiReader {
        data: dati,
        dim,
        name: nome,
        limits: *limits,
        cdf,
        header: testata,
        samples: Vec::new(),
        indice: Vec::new(),
        inizi: Vec::new(),
        counts: Vec::new(),
        windows: Vec::new(),
        known_tic: false,
        bpc: Vec::new(),
        bpc_seen: 0,
        cut: 0,
        profilo: false,
        sim: false,
        sha1: None,
    };
    let scans = if l.cdf.find("mass_values").is_some() {
        l.scans()?
    } else if l.cdf.find("ordinate_values").is_some() {
        0
    } else {
        return Err(Error::new("err.file.notAccepted").with("name", l.name));
    };
    let stem = l
        .name
        .rsplit_once('.')
        .map_or(l.name.as_str(), |(s, _)| s)
        .to_string();
    l.samples = vec![Sample {
        id: 1,
        nome: l.header.title.clone().unwrap_or(stem),
        scansioni: scans,
        size: None,
    }];
    Ok(Box::new(l))
}
