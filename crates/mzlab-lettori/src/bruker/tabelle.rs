//! The tables of `analysis.tdf` / `analysis.tsf` that the reader uses, loaded once: frames, precursors, isolation windows, calibrations, metadata and the properties of the first frame.
//! Every table is optional (a file lacks what its acquisition mode does not use); every count is checked against `Limiti`.

use super::calib::MzCal;

use super::sqlite_min::{Sqlite, Val};

use mzlab_core::error::Result;

use mzlab_core::lettori::Limiti;

use std::collections::{BTreeMap, HashMap};

impl Frame {
    /// The sum of the counts of the frame as `tdf_bin` holds them (the table's sum, undone from the normalisation to 100 ms).
    pub fn raw_sum(&self) -> f64 {
        if self.accumulo > 0.0 {
            self.summed * self.accumulo / 100.0
        } else {
            self.summed
        }
    }
}

impl Data {
    pub fn read(db: &mut Sqlite, limits: &Limiti) -> Result<Data> {
        let mut d = Data::default();
        rows(db, limits, "GlobalMetadata", &["Key", "Value"], |r| {
            if let (Some(k), Some(v)) = (r[0].text(), r[1].as_text()) {
                d.meta.insert(k.to_string(), v);
            }
        })?;
        d.frames_e_calibrazioni(db, limits)?;
        d.msms(db, limits)?;
        d.dia_prm(db, limits)?;
        d.proprieta(db, limits)?;
        Ok(d)
    }

    fn frames_e_calibrazioni(&mut self, db: &mut Sqlite, limits: &Limiti) -> Result<()> {
        self.frames(db, limits)?;
        self.calibrazioni(db, limits)
    }

    fn frames(&mut self, db: &mut Sqlite, limits: &Limiti) -> Result<()> {
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
        rows(db, limits, "Frames", &c, |r| {
            fr.push(Frame {
                id: i(&r[0]),
                moment: n(&r[1]),
                polarita: match r[2].text() {
                    Some("+") => 1,
                    Some("-") => -1,
                    _ => 0,
                },
                mode: i(&r[3]),
                tipo: i(&r[4]),
                pos: i(&r[5]),
                summed: n(&r[6]),
                max: n(&r[7]),
                scansioni: i(&r[8]).max(0) as u64,
                peaks: i(&r[9]).max(0) as u64,
                cal: i(&r[10]),
                t1: n(&r[11]),
                t2: n(&r[12]),
                accumulo: n(&r[13]),
            })
        })?;
        fr.sort_by_key(|f| f.id);
        self.frames = fr;
        Ok(())
    }

    fn calibrazioni(&mut self, db: &mut Sqlite, limits: &Limiti) -> Result<()> {
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
        rows(db, limits, "MzCalibration", &c, |r| {
            let m = MzCal {
                model: i(&r[1]),
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
            .table("TimsCalibration")
            .map(|t| t.colonne.clone())
            .unwrap_or_default();
        let cn: Vec<&str> = cols.iter().map(String::as_str).collect();
        let mut tims = Vec::new();
        rows(db, limits, "TimsCalibration", &cn, |r| {
            if tims.is_empty() {
                for (k, v) in cn.iter().zip(r) {
                    if let Some(t) = v.as_text() {
                        tims.push((k.to_string(), t));
                    }
                }
            }
        })?;
        self.tims = tims;
        Ok(())
    }

    fn msms(&mut self, db: &mut Sqlite, limits: &Limiti) -> Result<()> {
        let mut prec = HashMap::new();
        let c = [
            "Id",
            "MonoisotopicMz",
            "LargestPeakMz",
            "Charge",
            "Intensity",
        ];
        rows(db, limits, "Precursors", &c, |r| {
            let mz = r[1].f64().or_else(|| r[2].f64());
            prec.insert(
                i(&r[0]),
                Precursore {
                    mz,
                    load: r[3].i64(),
                    intensities: r[4].f64(),
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
        rows(db, limits, "PasefFrameMsMsInfo", &c, |r| {
            p.push((i(&r[0]), i(&r[1]), window(&r[2..])))
        })?;
        self.pasef = p;
        Ok(())
    }

    fn dia_prm(&mut self, db: &mut Sqlite, limits: &Limiti) -> Result<()> {
        let mut g: HashMap<i64, Vec<Window>> = HashMap::new();
        let c = [
            "WindowGroup",
            "ScanNumBegin",
            "ScanNumEnd",
            "IsolationMz",
            "IsolationWidth",
            "CollisionEnergy",
        ];
        rows(db, limits, "DiaFrameMsMsWindows", &c, |r| {
            g.entry(i(&r[0])).or_default().push(window(&r[1..]))
        })?;
        self.dia_groups = g;
        let mut df = HashMap::new();
        rows(
            db,
            limits,
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
        rows(db, limits, "PrmFrameMsMsInfo", &c, |r| {
            p.push((i(&r[0]), i(&r[1]), window(&r[2..])))
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
        rows(db, limits, "FrameMsMsInfo", &c, |r| {
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
    fn proprieta(&mut self, db: &mut Sqlite, limits: &Limiti) -> Result<()> {
        let mut group = None;
        rows_until(db, limits, "Frames", &["PropertyGroup"], |r| {
            group = r[0].i64();
            group.is_none()
        })?;
        let Some(group) = group else {
            return Ok(());
        };
        let mut def: HashMap<i64, (String, String)> = HashMap::new();
        let c = ["Id", "PermanentName", "DisplayName", "DisplayDimension"];
        rows(db, limits, "PropertyDefinitions", &c, |r| {
            let name = r[1].text().unwrap_or_default().to_string();
            let vis = r[2].text().unwrap_or_default();
            let unita = r[3].text().unwrap_or_default();
            def.insert(
                i(&r[0]),
                (
                    if name.is_empty() {
                        vis.to_string()
                    } else {
                        name
                    },
                    unita.to_string(),
                ),
            );
        })?;
        let mut out = Vec::new();
        rows(
            db,
            limits,
            "GroupProperties",
            &["PropertyGroup", "Property", "Value"],
            |r| {
                if i(&r[0]) != group {
                    return;
                }
                if let (Some((name, unita)), Some(v)) = (def.get(&i(&r[1])), r[2].as_text()) {
                    out.push((name.clone(), v, unita.clone()));
                }
            },
        )?;
        self.proprieta = out;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub id: i64,
    /// Seconds.
    pub moment: f64,
    pub polarita: i8,
    pub mode: i64,
    pub tipo: i64,
    /// Offset of the block in `*_bin`.
    pub pos: i64,
    pub summed: f64,
    pub max: f64,
    pub scansioni: u64,
    pub peaks: u64,
    pub cal: i64,
    pub t1: f64,
    pub t2: f64,
    /// Ms of ion accumulation: `SummedIntensities` and `MaxIntensity` are normalised to 100 ms, the counts of `tdf_bin` are not.
    pub accumulo: f64,
}

/// An isolation window with the scans of the mobility ramp it covers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Window {
    pub scan: (u32, u32),
    pub mz: f64,
    pub width: f64,
    pub ce: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Precursore {
    pub mz: Option<f64>,
    pub load: Option<i64>,
    pub intensities: Option<f64>,
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
pub struct Data {
    pub frames: Vec<Frame>,
    pub cal: HashMap<i64, MzCal>,
    pub meta: BTreeMap<String, String>,
    pub precursori: HashMap<i64, Precursore>,
    /// `(frame, precursor, window)` of DDA-PASEF.
    pub pasef: Vec<(i64, i64, Window)>,
    pub dia_groups: HashMap<i64, Vec<Window>>,
    pub dia_frame: HashMap<i64, i64>,
    /// `(frame, target, window)` of PRM-PASEF.
    pub prm: Vec<(i64, i64, Window)>,
    pub info_frame: HashMap<i64, InfoFrame>,
    /// `(name, value, unit)` of the properties of the first frame's group.
    pub proprieta: Vec<(String, String, String)>,
    /// Rows of `TimsCalibration` as `(column, value)`.
    pub tims: Vec<(String, String)>,
}

/// The rows of `nome` with only `colonne`, in that order (`Null` for a column the file lacks). A missing table is no rows. The other columns of a row are skipped, not decoded.
fn rows<F>(db: &mut Sqlite, limits: &Limiti, name: &str, columns: &[&str], mut f: F) -> Result<()>
where
    F: FnMut(&[Val]),
{
    rows_until(db, limits, name, columns, |r| {
        f(r);
        true
    })
}

/// `righe` that stops at the first row `f` answers `false` for.
fn rows_until<F>(
    db: &mut Sqlite,
    limits: &Limiti,
    name: &str,
    columns: &[&str],
    mut f: F,
) -> Result<()>
where
    F: FnMut(&[Val]) -> bool,
{
    let Some(t) = db.table(name).cloned() else {
        return Ok(());
    };
    let idx: Vec<Option<usize>> = columns.iter().map(|c| t.col(c)).collect();
    let mut n = 0u64;
    db.rows_cols(&t, &idx, |r| {
        n += 1;
        limits.voci_ok(n)?;
        Ok(f(r))
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

fn window(r: &[Val]) -> Window {
    Window {
        scan: (scan(&r[0]), scan(&r[1])),
        mz: n(&r[2]),
        width: n(&r[3]),
        ce: n(&r[4]),
    }
}
