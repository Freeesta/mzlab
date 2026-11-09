//! A netCDF-3 encoder for the tests of the family `andi` (classic and 64-bit offset, record variables, every `nc_type`) and the ANDI-MS / ANDI-Chromatography files built with it.
#![allow(dead_code)]

use mzlab_core::lettori::{Limits, Reader};

#[derive(Clone)]
pub enum At {
    T(String),
    N(u32, Vec<f64>),
}

pub fn t(s: &str) -> At {
    At::T(s.into())
}

#[derive(Clone)]
pub struct Vb {
    pub name: String,
    pub dims: Vec<usize>,
    /// nc_type: 1 byte, 2 char, 3 short, 4 int, 5 float, 6 double.
    pub kind: u32,
    pub att: Vec<(String, At)>,
    pub data: Vec<f64>,
    pub text: Vec<u8>,
}

pub fn var(nome: &str, dims: &[usize], tipo: u32, dati: Vec<f64>) -> Vb {
    Vb {
        name: nome.into(),
        dims: dims.to_vec(),
        kind: tipo,
        att: Vec::new(),
        data: dati,
        text: Vec::new(),
    }
}

pub fn car(nome: &str, dims: &[usize], text: &str, row: usize) -> Vb {
    let mut b = text.as_bytes().to_vec();
    b.resize(row, 0);
    Vb {
        name: nome.into(),
        dims: dims.to_vec(),
        kind: 2,
        att: Vec::new(),
        data: Vec::new(),
        text: b,
    }
}

pub fn width(kind: u32) -> usize {
    [1, 1, 2, 4, 4, 8][kind as usize - 1]
}

fn pad(o: &mut Vec<u8>) {
    while !o.len().is_multiple_of(4) {
        o.push(0);
    }
}

fn name(o: &mut Vec<u8>, s: &str) {
    o.extend((s.len() as u32).to_be_bytes());
    o.extend(s.as_bytes());
    pad(o);
}

fn element(o: &mut Vec<u8>, kind: u32, x: f64) {
    match kind {
        1 => o.push(x as i8 as u8),
        2 => o.push(x as u8),
        3 => o.extend((x as i16).to_be_bytes()),
        4 => o.extend((x as i32).to_be_bytes()),
        5 => o.extend((x as f32).to_be_bytes()),
        _ => o.extend(x.to_be_bytes()),
    }
}

fn attributes(o: &mut Vec<u8>, a: &[(String, At)]) {
    if a.is_empty() {
        o.extend([0u8; 8]);
        return;
    }
    o.extend(0x0Cu32.to_be_bytes());
    o.extend((a.len() as u32).to_be_bytes());
    for (n, v) in a {
        name(o, n);
        match v {
            At::T(s) => {
                o.extend(2u32.to_be_bytes());
                o.extend((s.len() as u32).to_be_bytes());
                o.extend(s.as_bytes());
            }
            At::N(ty, x) => {
                o.extend(ty.to_be_bytes());
                o.extend((x.len() as u32).to_be_bytes());
                x.iter().for_each(|&y| element(o, *ty, y));
            }
        }
        pad(o);
    }
}

/// The bytes of one variable's data for the elements `da..a`.
fn data_of(v: &Vb, da: usize, a: usize) -> Vec<u8> {
    let mut o = Vec::new();
    if v.kind == 2 {
        o.extend(&v.text[da..a]);
    } else {
        v.data[da..a]
            .iter()
            .for_each(|&x| element(&mut o, v.kind, x));
    }
    o
}

/// The header of a file whose variables begin at `begins` and take `vsizes` bytes.
fn header_cdf(
    v2: bool,
    dims: &[(&str, u32)],
    globals: &[(String, At)],
    vars: &[Vb],
    numrecs: u32,
    begins: &[u64],
    vsizes: &[usize],
) -> Vec<u8> {
    let mut o = vec![b'C', b'D', b'F', if v2 { 2 } else { 1 }];
    o.extend(numrecs.to_be_bytes());
    o.extend(0x0Au32.to_be_bytes());
    o.extend((dims.len() as u32).to_be_bytes());
    for (n, l) in dims {
        name(&mut o, n);
        o.extend(l.to_be_bytes());
    }
    attributes(&mut o, globals);
    o.extend(0x0Bu32.to_be_bytes());
    o.extend((vars.len() as u32).to_be_bytes());
    for (i, (v, &b)) in vars.iter().zip(begins).enumerate() {
        name(&mut o, &v.name);
        o.extend((v.dims.len() as u32).to_be_bytes());
        v.dims
            .iter()
            .for_each(|&d| o.extend((d as u32).to_be_bytes()));
        attributes(&mut o, &v.att);
        o.extend(v.kind.to_be_bytes());
        o.extend((vsizes[i] as u32).to_be_bytes());
        if v2 {
            o.extend(b.to_be_bytes());
        } else {
            o.extend((b as u32).to_be_bytes());
        }
    }
    o
}

/// A classic (`v2 == false`) or 64-bit offset file: `dims` are (name, length) with 0 = unlimited, `numrecs` the number of records.
pub fn cdf(
    v2: bool,
    dims: &[(&str, u32)],
    globals: &[(String, At)],
    vars: &[Vb],
    numrecs: u32,
) -> Vec<u8> {
    let elements = |v: &Vb| -> usize {
        v.dims
            .iter()
            .map(|&d| {
                if dims[d].1 == 0 {
                    numrecs as usize
                } else {
                    dims[d].1 as usize
                }
            })
            .product()
    };
    let record = |v: &Vb| v.dims.first().is_some_and(|&d| dims[d].1 == 0);
    let per_rec = |v: &Vb| -> usize { v.dims[1..].iter().map(|&d| dims[d].1 as usize).product() };
    let again = |v: &Vb| width(v.kind) * if record(v) { per_rec(v) } else { elements(v) };
    let un_record = vars.iter().filter(|v| record(v)).count() == 1;
    let vsize = |v: &Vb| {
        if record(v) && un_record {
            again(v)
        } else {
            (again(v) + 3) & !3
        }
    };
    let vsizes: Vec<usize> = vars.iter().map(vsize).collect();
    let header = |begins: &[u64]| header_cdf(v2, dims, globals, vars, numrecs, begins, &vsizes);
    let mut begins = vec![0u64; vars.len()];
    let mut pos = header(&begins).len() as u64;
    for (i, v) in vars.iter().enumerate().filter(|(_, v)| !record(v)) {
        begins[i] = pos;
        pos += vsize(v) as u64;
    }
    let mut r = pos;
    for (i, v) in vars.iter().enumerate().filter(|(_, v)| record(v)) {
        begins[i] = r;
        r += vsize(v) as u64;
    }
    let mut o = header(&begins);
    for v in vars.iter().filter(|v| !record(v)) {
        o.extend(data_of(v, 0, elements(v)));
        pad(&mut o);
    }
    for k in 0..numrecs as usize {
        for v in vars.iter().filter(|v| record(v)) {
            let e = per_rec(v);
            o.extend(data_of(v, k * e, (k + 1) * e));
            if !un_record {
                pad(&mut o);
            }
        }
    }
    o
}

/// The shape of a synthetic ANDI-MS run.
#[derive(Clone)]
pub struct Run {
    pub v2: bool,
    /// nc_type of `mass_values` / `intensity_values`.
    pub kind_mz: u32,
    pub kind_it: u32,
    pub mz_scale: Option<f64>,
    /// `(scale_factor, add_offset)` stored on `scan_acquisition_time`; the file holds `(t - offset) / factor`.
    pub time_scale: Option<(f64, f64)>,
    pub sim: bool,
    pub profile: bool,
    pub polarity: &'static str,
}

pub const FLAT_RUN: Run = Run {
    v2: false,
    kind_mz: 5,
    kind_it: 5,
    mz_scale: None,
    time_scale: None,
    sim: false,
    profile: false,
    polarity: "Positive Polarity",
};

/// The points of scan `k` (m/z, intensity).
pub fn points(k: usize) -> Vec<(f64, f64)> {
    (0..3 + k % 4)
        .map(|i| {
            (
                50.0 + i as f64 * 7.0 + k as f64,
                100.0 + ((i * 13 + k * 5) % 29) as f64,
            )
        })
        .collect()
}

pub const SCANS: usize = 6;

pub fn globals_ms(c: &Run) -> Vec<(String, At)> {
    vec![
        ("experiment_title".to_string(), t("run one")),
        (
            "experiment_date_time_stamp".to_string(),
            t("20190312141102+0100"),
        ),
        ("operator_name".to_string(), t("Some Operator")),
        (
            "experiment_type".to_string(),
            t(if c.profile {
                "Continuum Mass Spectrum"
            } else {
                "Centroided Mass Spectrum"
            }),
        ),
        ("test_ionization_mode".to_string(), t("Electron Impact")),
        ("test_ionization_polarity".to_string(), t(c.polarity)),
        (
            "test_scan_function".to_string(),
            t(if c.sim {
                "Selected Ion Detection"
            } else {
                "Mass Scan"
            }),
        ),
    ]
}

pub fn andi_ms(c: &Run) -> Vec<u8> {
    andi_ms_with(c, SCANS, points, true)
}

/// An ANDI-MS file of `scansioni` scans whose points are `punti(k)`; `tic`: whether it has `total_intensity`.
pub fn andi_ms_with(
    c: &Run,
    scans: usize,
    points: impl Fn(usize) -> Vec<(f64, f64)>,
    tic: bool,
) -> Vec<u8> {
    let counts: Vec<usize> = (0..scans).map(|k| points(k).len()).collect();
    let total: usize = counts.iter().sum();
    let mut starts = vec![0usize];
    for &n in &counts[..counts.len() - 1] {
        starts.push(starts.last().unwrap() + n);
    }
    let mz: Vec<f64> = (0..scans)
        .flat_map(|k| points(k).into_iter().map(|p| p.0))
        .map(|x| c.mz_scale.map_or(x, |f| x / f))
        .collect();
    let it: Vec<f64> = (0..scans)
        .flat_map(|k| points(k).into_iter().map(|p| p.1))
        .collect();
    // dimensions: 0 point_number (unlimited), 1 scan_number, 2 instrument_number, 3 _32_byte_string, 4 range
    let dims = [
        ("point_number", 0u32),
        ("scan_number", scans as u32),
        ("instrument_number", 1),
        ("_32_byte_string", 32),
        ("range", 2),
    ];
    let per_scan = |n: &str, ty: u32, x: Vec<f64>| var(n, &[1], ty, x);
    let mut vm = var("mass_values", &[0], c.kind_mz, mz);
    vm.att.push(("units".into(), t("M/Z")));
    if let Some(f) = c.mz_scale {
        vm.att.push(("scale_factor".into(), At::N(6, vec![f])));
    }
    let mut vi = var("intensity_values", &[0], c.kind_it, it.clone());
    vi.att
        .push(("units".into(), t("Arbitrary Intensity Units")));
    let mut moment = per_scan(
        "scan_acquisition_time",
        6,
        (0..scans)
            .map(|k| {
                let t = 60.0 + 1.5 * k as f64;
                c.time_scale.map_or(t, |(f, o)| (t - o) / f)
            })
            .collect(),
    );
    if let Some((f, o)) = c.time_scale {
        moment.att.push(("scale_factor".into(), At::N(6, vec![f])));
        moment.att.push(("add_offset".into(), At::N(6, vec![o])));
    }
    let mut vars = vec![
        moment,
        per_scan(
            "total_intensity",
            6,
            (0..scans)
                // a NaN point (the tests of the BPC) leaves the TIC of the file finite
                .map(|k| points(k).iter().map(|p| p.1).filter(|x| !x.is_nan()).sum())
                .collect(),
        ),
        per_scan("mass_range_min", 6, vec![40.0; scans]),
        per_scan("mass_range_max", 6, vec![400.0; scans]),
        per_scan("scan_index", 4, starts.iter().map(|&x| x as f64).collect()),
        per_scan("point_count", 4, counts.iter().map(|&x| x as f64).collect()),
        vm,
        vi,
        car("instrument_name", &[2, 3], "GCMS Test", 32),
        car("instrument_mfr", &[2, 3], "Testmaker", 32),
        car("instrument_model", &[2, 3], "Model 7", 32),
        car("instrument_serial_no", &[2, 3], "SN-0042", 32),
    ];
    if !tic {
        vars.retain(|v| v.name != "total_intensity");
    }
    let g = globals_ms(c);
    let _ = total;
    cdf(c.v2, &dims, &g, &vars, total as u32)
}

/// An ANDI-Chromatography file: `n` points of one FID trace, uniform sampling.
pub fn andi_chromatogram(n: usize) -> Vec<u8> {
    let dims = [("point_number", 0u32), ("_32_byte_string", 32)];
    let mut y = var(
        "ordinate_values",
        &[0],
        6,
        (0..n).map(|i| 1000.0 + (i * 7 % 50) as f64).collect(),
    );
    y.att.push(("units".into(), t("Volts")));
    let vars = vec![
        y,
        var("actual_delay_time", &[], 6, vec![30.0]),
        var("actual_sampling_interval", &[], 6, vec![0.5]),
    ];
    let g = vec![
        ("detector_name".to_string(), t("FID1A")),
        ("sample_name".to_string(), t("blank")),
    ];
    cdf(false, &dims, &g, &vars, n as u32)
}

pub fn open_andi(byte: Vec<u8>) -> Box<dyn Reader> {
    mzlab_lettori::apri(super::file_set("campione.cdf", byte), &Limits::sito()).unwrap()
}

pub fn andi_error(byte: Vec<u8>) -> &'static str {
    match mzlab_lettori::apri(super::file_set("campione.cdf", byte), &Limits::sito()) {
        Err(e) => e.key,
        Ok(mut l) => {
            for k in 0..l.index(0).map(|i| i.len()).unwrap_or(0) {
                if let Err(e) = l.scan(0, k) {
                    return e.key;
                }
            }
            "ok"
        }
    }
}
