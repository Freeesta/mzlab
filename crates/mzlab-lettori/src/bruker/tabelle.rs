//! The tables of `analysis.tdf` / `analysis.tsf` that the reader uses, loaded once: frames, precursors, isolation windows, calibrations, metadata and the properties of the first frame.
//! Every table is optional (a file lacks what its acquisition mode does not use); every count is checked against `Limiti`.

use super::calib::MzCal;
use super::sqlite_min::{Sqlite, Val};
use mzlab_core::error::Result;
use mzlab_core::lettori::Limiti;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub id: i64,
    /// Seconds.
    pub tempo: f64,
    pub polarita: i8,
    pub modo: i64,
    pub tipo: i64,
    /// Offset of the block in `*_bin`.
    pub pos: i64,
    pub somma: f64,
    pub max: f64,
    pub scansioni: u64,
    pub picchi: u64,
    pub cal: i64,
    pub t1: f64,
    pub t2: f64,
    /// Ms of ion accumulation: `SummedIntensities` and `MaxIntensity` are normalised to 100 ms, the counts of `tdf_bin` are not.
    pub accumulo: f64,
}

/// An isolation window with the scans of the mobility ramp it covers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Finestra {
    pub scan: (u32, u32),
    pub mz: f64,
    pub larghezza: f64,
    pub ce: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Precursore {
    pub mz: Option<f64>,
    pub carica: Option<i64>,
    pub intensita: Option<f64>,
}

/// `FrameMsMsInfo` (isolation of a whole frame: MRM and MALDI MS/MS).
#[derive(Debug, Clone, Copy, Default)]
pub struct InfoFrame {
    pub mz: f64,
    pub larghezza: f64,
    pub carica: Option<i64>,
    pub ce: f64,
}

#[derive(Debug, Default)]
pub struct Dati {
    pub frames: Vec<Frame>,
    pub cal: HashMap<i64, MzCal>,
    pub meta: BTreeMap<String, String>,
    pub precursori: HashMap<i64, Precursore>,
    /// `(frame, precursor, window)` of DDA-PASEF.
    pub pasef: Vec<(i64, i64, Finestra)>,
    pub dia_gruppi: HashMap<i64, Vec<Finestra>>,
    pub dia_frame: HashMap<i64, i64>,
    /// `(frame, target, window)` of PRM-PASEF.
    pub prm: Vec<(i64, i64, Finestra)>,
    pub info_frame: HashMap<i64, InfoFrame>,
    /// `(name, value, unit)` of the properties of the first frame's group.
    pub proprieta: Vec<(String, String, String)>,
    /// Rows of `TimsCalibration` as `(column, value)`.
    pub tims: Vec<(String, String)>,
}

/// The rows of `nome` with only `colonne`, in that order (`Null` for a column the file lacks). A missing table is no rows.
fn righe<F>(db: &mut Sqlite, limiti: &Limiti, nome: &str, colonne: &[&str], mut f: F) -> Result<()>
where
    F: FnMut(&[Val]),
{
    let Some(t) = db.tabella(nome).cloned() else {
        return Ok(());
    };
    let idx: Vec<Option<usize>> = colonne.iter().map(|c| t.col(c)).collect();
    let mut n = 0u64;
    db.righe(&t, |r| {
        n += 1;
        limiti.voci_ok(n)?;
        let v: Vec<Val> = idx
            .iter()
            .map(|i| i.and_then(|i| r.get(i).cloned()).unwrap_or(Val::Null))
            .collect();
        f(&v);
        Ok(true)
    })
}

fn n(v: &Val) -> f64 {
    v.f64().unwrap_or(0.0)
}
fn i(v: &Val) -> i64 {
    v.i64().unwrap_or(0)
}
fn scan(v: &Val) -> u32 {
    u32::try_from(i(v).max(0)).unwrap_or(u32::MAX)
}

fn finestra(r: &[Val]) -> Finestra {
    Finestra {
        scan: (scan(&r[0]), scan(&r[1])),
        mz: n(&r[2]),
        larghezza: n(&r[3]),
        ce: n(&r[4]),
    }
}

impl Frame {
    /// The sum of the counts of the frame as `tdf_bin` holds them (the table's sum, undone from the normalisation to 100 ms).
    pub fn somma_grezza(&self) -> f64 {
        if self.accumulo > 0.0 {
            self.somma * self.accumulo / 100.0
        } else {
            self.somma
        }
    }
}

impl Dati {
    pub fn leggi(db: &mut Sqlite, limiti: &Limiti) -> Result<Dati> {
        let mut d = Dati::default();
        righe(db, limiti, "GlobalMetadata", &["Key", "Value"], |r| {
            if let (Some(k), Some(v)) = (r[0].testo(), r[1].come_testo()) {
                d.meta.insert(k.to_string(), v);
            }
        })?;
        d.frames_e_calibrazioni(db, limiti)?;
        d.msms(db, limiti)?;
        d.proprieta(db, limiti)?;
        Ok(d)
    }

    fn frames_e_calibrazioni(&mut self, db: &mut Sqlite, limiti: &Limiti) -> Result<()> {
        let mut fr = Vec::new();
        let c = [
            "Id",
            "Time",
            "Polarity",
            "ScanMode",
            "MsMsType",
            "TimsId",
            "SummedIntensities",
            "MaxIntensity",
            "NumScans",
            "NumPeaks",
            "MzCalibration",
            "T1",
            "T2",
            "AccumulationTime",
        ];
        righe(db, limiti, "Frames", &c, |r| {
            fr.push(Frame {
                id: i(&r[0]),
                tempo: n(&r[1]),
                polarita: match r[2].testo() {
                    Some("+") => 1,
                    Some("-") => -1,
                    _ => 0,
                },
                modo: i(&r[3]),
                tipo: i(&r[4]),
                pos: i(&r[5]),
                somma: n(&r[6]),
                max: n(&r[7]),
                scansioni: i(&r[8]).max(0) as u64,
                picchi: i(&r[9]).max(0) as u64,
                cal: i(&r[10]),
                t1: n(&r[11]),
                t2: n(&r[12]),
                accumulo: n(&r[13]),
            })
        })?;
        fr.sort_by_key(|f| f.id);
        self.frames = fr;
        let mut cal = HashMap::new();
        let c = [
            "Id",
            "ModelType",
            "DigitizerTimebase",
            "DigitizerDelay",
            "C0",
            "C1",
            "C2",
            "C3",
            "C4",
            "T1",
            "T2",
            "dC1",
            "dC2",
        ];
        righe(db, limiti, "MzCalibration", &c, |r| {
            let m = MzCal {
                modello: i(&r[1]),
                timebase: n(&r[2]),
                delay: n(&r[3]),
                c: [n(&r[4]), n(&r[5]), n(&r[6]), n(&r[7]), n(&r[8])],
                temp: [n(&r[9]), n(&r[10])],
                coef: [n(&r[11]), n(&r[12])],
            };
            cal.insert(i(&r[0]), m);
        })?;
        self.cal = cal;
        let cols: Vec<String> = db
            .tabella("TimsCalibration")
            .map(|t| t.colonne.clone())
            .unwrap_or_default();
        let cn: Vec<&str> = cols.iter().map(String::as_str).collect();
        let mut tims = Vec::new();
        righe(db, limiti, "TimsCalibration", &cn, |r| {
            if tims.is_empty() {
                for (k, v) in cn.iter().zip(r) {
                    if let Some(t) = v.come_testo() {
                        tims.push((k.to_string(), t));
                    }
                }
            }
        })?;
        self.tims = tims;
        Ok(())
    }

    fn msms(&mut self, db: &mut Sqlite, limiti: &Limiti) -> Result<()> {
        let mut prec = HashMap::new();
        let c = [
            "Id",
            "MonoisotopicMz",
            "LargestPeakMz",
            "Charge",
            "Intensity",
        ];
        righe(db, limiti, "Precursors", &c, |r| {
            let mz = r[1].f64().or_else(|| r[2].f64());
            prec.insert(
                i(&r[0]),
                Precursore {
                    mz,
                    carica: r[3].i64(),
                    intensita: r[4].f64(),
                },
            );
        })?;
        self.precursori = prec;
        let mut p = Vec::new();
        let c = [
            "Frame",
            "Precursor",
            "ScanNumBegin",
            "ScanNumEnd",
            "IsolationMz",
            "IsolationWidth",
            "CollisionEnergy",
        ];
        righe(db, limiti, "PasefFrameMsMsInfo", &c, |r| {
            p.push((i(&r[0]), i(&r[1]), finestra(&r[2..])))
        })?;
        self.pasef = p;
        let mut g: HashMap<i64, Vec<Finestra>> = HashMap::new();
        let c = [
            "WindowGroup",
            "ScanNumBegin",
            "ScanNumEnd",
            "IsolationMz",
            "IsolationWidth",
            "CollisionEnergy",
        ];
        righe(db, limiti, "DiaFrameMsMsWindows", &c, |r| {
            g.entry(i(&r[0])).or_default().push(finestra(&r[1..]))
        })?;
        self.dia_gruppi = g;
        let mut df = HashMap::new();
        righe(
            db,
            limiti,
            "DiaFrameMsMsInfo",
            &["Frame", "WindowGroup"],
            |r| {
                df.insert(i(&r[0]), i(&r[1]));
            },
        )?;
        self.dia_frame = df;
        let mut p = Vec::new();
        let c = [
            "Frame",
            "Target",
            "ScanNumBegin",
            "ScanNumEnd",
            "IsolationMz",
            "IsolationWidth",
            "CollisionEnergy",
        ];
        righe(db, limiti, "PrmFrameMsMsInfo", &c, |r| {
            p.push((i(&r[0]), i(&r[1]), finestra(&r[2..])))
        })?;
        self.prm = p;
        let mut m = HashMap::new();
        let c = [
            "Frame",
            "TriggerMass",
            "IsolationWidth",
            "PrecursorCharge",
            "CollisionEnergy",
        ];
        righe(db, limiti, "FrameMsMsInfo", &c, |r| {
            m.insert(
                i(&r[0]),
                InfoFrame {
                    mz: n(&r[1]),
                    larghezza: n(&r[2]),
                    carica: r[3].i64(),
                    ce: n(&r[4]),
                },
            );
        })?;
        self.info_frame = m;
        Ok(())
    }

    /// The properties of the property group of the first frame (the method settings that stay constant), with the names and units the file defines.
    fn proprieta(&mut self, db: &mut Sqlite, limiti: &Limiti) -> Result<()> {
        let mut gruppo = None;
        righe(db, limiti, "Frames", &["PropertyGroup"], |r| {
            gruppo = gruppo.or_else(|| r[0].i64());
        })?;
        let Some(gruppo) = gruppo else {
            return Ok(());
        };
        let mut def: HashMap<i64, (String, String)> = HashMap::new();
        let c = ["Id", "PermanentName", "DisplayName", "DisplayDimension"];
        righe(db, limiti, "PropertyDefinitions", &c, |r| {
            let nome = r[1].testo().unwrap_or_default().to_string();
            let vis = r[2].testo().unwrap_or_default();
            let unita = r[3].testo().unwrap_or_default();
            def.insert(
                i(&r[0]),
                (
                    if nome.is_empty() {
                        vis.to_string()
                    } else {
                        nome
                    },
                    unita.to_string(),
                ),
            );
        })?;
        let mut out = Vec::new();
        righe(
            db,
            limiti,
            "GroupProperties",
            &["PropertyGroup", "Property", "Value"],
            |r| {
                if i(&r[0]) != gruppo {
                    return;
                }
                if let (Some((nome, unita)), Some(v)) = (def.get(&i(&r[1])), r[2].come_testo()) {
                    out.push((nome.clone(), v, unita.clone()));
                }
            },
        )?;
        self.proprieta = out;
        Ok(())
    }
}
