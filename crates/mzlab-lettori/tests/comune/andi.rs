//! A netCDF-3 encoder for the tests of the family `andi` (classic and 64-bit offset, record variables, every `nc_type`) and the ANDI-MS / ANDI-Chromatography files built with it.
#![allow(dead_code)]

use mzlab_core::lettori::{Lettore, Limiti};

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
    pub nome: String,
    pub dims: Vec<usize>,
    /// nc_type: 1 byte, 2 char, 3 short, 4 int, 5 float, 6 double.
    pub tipo: u32,
    pub att: Vec<(String, At)>,
    pub dati: Vec<f64>,
    pub testo: Vec<u8>,
}

pub fn var(nome: &str, dims: &[usize], tipo: u32, dati: Vec<f64>) -> Vb {
    Vb {
        nome: nome.into(),
        dims: dims.to_vec(),
        tipo,
        att: Vec::new(),
        dati,
        testo: Vec::new(),
    }
}

pub fn car(nome: &str, dims: &[usize], testo: &str, riga: usize) -> Vb {
    let mut b = testo.as_bytes().to_vec();
    b.resize(riga, 0);
    Vb {
        nome: nome.into(),
        dims: dims.to_vec(),
        tipo: 2,
        att: Vec::new(),
        dati: Vec::new(),
        testo: b,
    }
}

pub fn larghezza(tipo: u32) -> usize {
    [1, 1, 2, 4, 4, 8][tipo as usize - 1]
}

fn pad(o: &mut Vec<u8>) {
    while !o.len().is_multiple_of(4) {
        o.push(0);
    }
}

fn nome(o: &mut Vec<u8>, s: &str) {
    o.extend((s.len() as u32).to_be_bytes());
    o.extend(s.as_bytes());
    pad(o);
}

fn elemento(o: &mut Vec<u8>, tipo: u32, x: f64) {
    match tipo {
        1 => o.push(x as i8 as u8),
        2 => o.push(x as u8),
        3 => o.extend((x as i16).to_be_bytes()),
        4 => o.extend((x as i32).to_be_bytes()),
        5 => o.extend((x as f32).to_be_bytes()),
        _ => o.extend(x.to_be_bytes()),
    }
}

fn attributi(o: &mut Vec<u8>, a: &[(String, At)]) {
    if a.is_empty() {
        o.extend([0u8; 8]);
        return;
    }
    o.extend(0x0Cu32.to_be_bytes());
    o.extend((a.len() as u32).to_be_bytes());
    for (n, v) in a {
        nome(o, n);
        match v {
            At::T(s) => {
                o.extend(2u32.to_be_bytes());
                o.extend((s.len() as u32).to_be_bytes());
                o.extend(s.as_bytes());
            }
            At::N(ty, x) => {
                o.extend(ty.to_be_bytes());
                o.extend((x.len() as u32).to_be_bytes());
                x.iter().for_each(|&y| elemento(o, *ty, y));
            }
        }
        pad(o);
    }
}

/// The bytes of one variable's data for the elements `da..a`.
fn dati_di(v: &Vb, da: usize, a: usize) -> Vec<u8> {
    let mut o = Vec::new();
    if v.tipo == 2 {
        o.extend(&v.testo[da..a]);
    } else {
        v.dati[da..a]
            .iter()
            .for_each(|&x| elemento(&mut o, v.tipo, x));
    }
    o
}

/// The header of a file whose variables begin at `begins` and take `vsizes` bytes.
fn testata_cdf(
    v2: bool,
    dims: &[(&str, u32)],
    globali: &[(String, At)],
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
        nome(&mut o, n);
        o.extend(l.to_be_bytes());
    }
    attributi(&mut o, globali);
    o.extend(0x0Bu32.to_be_bytes());
    o.extend((vars.len() as u32).to_be_bytes());
    for (i, (v, &b)) in vars.iter().zip(begins).enumerate() {
        nome(&mut o, &v.nome);
        o.extend((v.dims.len() as u32).to_be_bytes());
        v.dims
            .iter()
            .for_each(|&d| o.extend((d as u32).to_be_bytes()));
        attributi(&mut o, &v.att);
        o.extend(v.tipo.to_be_bytes());
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
    globali: &[(String, At)],
    vars: &[Vb],
    numrecs: u32,
) -> Vec<u8> {
    let elementi = |v: &Vb| -> usize {
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
    let ancora = |v: &Vb| larghezza(v.tipo) * if record(v) { per_rec(v) } else { elementi(v) };
    let un_record = vars.iter().filter(|v| record(v)).count() == 1;
    let vsize = |v: &Vb| {
        if record(v) && un_record {
            ancora(v)
        } else {
            (ancora(v) + 3) & !3
        }
    };
    let vsizes: Vec<usize> = vars.iter().map(vsize).collect();
    let intestazione =
        |begins: &[u64]| testata_cdf(v2, dims, globali, vars, numrecs, begins, &vsizes);
    let mut begins = vec![0u64; vars.len()];
    let mut pos = intestazione(&begins).len() as u64;
    for (i, v) in vars.iter().enumerate().filter(|(_, v)| !record(v)) {
        begins[i] = pos;
        pos += vsize(v) as u64;
    }
    let mut r = pos;
    for (i, v) in vars.iter().enumerate().filter(|(_, v)| record(v)) {
        begins[i] = r;
        r += vsize(v) as u64;
    }
    let mut o = intestazione(&begins);
    for v in vars.iter().filter(|v| !record(v)) {
        o.extend(dati_di(v, 0, elementi(v)));
        pad(&mut o);
    }
    for k in 0..numrecs as usize {
        for v in vars.iter().filter(|v| record(v)) {
            let e = per_rec(v);
            o.extend(dati_di(v, k * e, (k + 1) * e));
            if !un_record {
                pad(&mut o);
            }
        }
    }
    o
}

/// The shape of a synthetic ANDI-MS run.
#[derive(Clone)]
pub struct Corsa {
    pub v2: bool,
    /// nc_type of `mass_values` / `intensity_values`.
    pub tipo_mz: u32,
    pub tipo_it: u32,
    pub scala_mz: Option<f64>,
    pub sim: bool,
    pub profilo: bool,
    pub polarita: &'static str,
}

pub const PIATTA: Corsa = Corsa {
    v2: false,
    tipo_mz: 5,
    tipo_it: 5,
    scala_mz: None,
    sim: false,
    profilo: false,
    polarita: "Positive Polarity",
};

/// The points of scan `k` (m/z, intensity).
pub fn punti(k: usize) -> Vec<(f64, f64)> {
    (0..3 + k % 4)
        .map(|i| {
            (
                50.0 + i as f64 * 7.0 + k as f64,
                100.0 + ((i * 13 + k * 5) % 29) as f64,
            )
        })
        .collect()
}

pub const SCANSIONI: usize = 6;

fn globali_ms(c: &Corsa) -> Vec<(String, At)> {
    vec![
        ("experiment_title".to_string(), t("run one")),
        (
            "experiment_date_time_stamp".to_string(),
            t("20190312141102+0100"),
        ),
        ("operator_name".to_string(), t("Some Operator")),
        (
            "experiment_type".to_string(),
            t(if c.profilo {
                "Continuum Mass Spectrum"
            } else {
                "Centroided Mass Spectrum"
            }),
        ),
        ("test_ionization_mode".to_string(), t("Electron Impact")),
        ("test_ionization_polarity".to_string(), t(c.polarita)),
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

pub fn andi_ms(c: &Corsa) -> Vec<u8> {
    let conti: Vec<usize> = (0..SCANSIONI).map(|k| punti(k).len()).collect();
    let totale: usize = conti.iter().sum();
    let mut inizi = vec![0usize];
    for &n in &conti[..conti.len() - 1] {
        inizi.push(inizi.last().unwrap() + n);
    }
    let mz: Vec<f64> = (0..SCANSIONI)
        .flat_map(|k| punti(k).into_iter().map(|p| p.0))
        .map(|x| c.scala_mz.map_or(x, |f| x / f))
        .collect();
    let it: Vec<f64> = (0..SCANSIONI)
        .flat_map(|k| punti(k).into_iter().map(|p| p.1))
        .collect();
    // dimensions: 0 point_number (unlimited), 1 scan_number, 2 instrument_number, 3 _32_byte_string, 4 range
    let dims = [
        ("point_number", 0u32),
        ("scan_number", SCANSIONI as u32),
        ("instrument_number", 1),
        ("_32_byte_string", 32),
        ("range", 2),
    ];
    let per_scan = |n: &str, ty: u32, x: Vec<f64>| var(n, &[1], ty, x);
    let mut vm = var("mass_values", &[0], c.tipo_mz, mz);
    vm.att.push(("units".into(), t("M/Z")));
    if let Some(f) = c.scala_mz {
        vm.att.push(("scale_factor".into(), At::N(6, vec![f])));
    }
    let mut vi = var("intensity_values", &[0], c.tipo_it, it.clone());
    vi.att
        .push(("units".into(), t("Arbitrary Intensity Units")));
    let vars = vec![
        per_scan(
            "scan_acquisition_time",
            6,
            (0..SCANSIONI).map(|k| 60.0 + 1.5 * k as f64).collect(),
        ),
        per_scan(
            "total_intensity",
            6,
            (0..SCANSIONI)
                .map(|k| punti(k).iter().map(|p| p.1).sum())
                .collect(),
        ),
        per_scan("mass_range_min", 6, vec![40.0; SCANSIONI]),
        per_scan("mass_range_max", 6, vec![400.0; SCANSIONI]),
        per_scan("scan_index", 4, inizi.iter().map(|&x| x as f64).collect()),
        per_scan("point_count", 4, conti.iter().map(|&x| x as f64).collect()),
        vm,
        vi,
        car("instrument_name", &[2, 3], "GCMS Test", 32),
        car("instrument_mfr", &[2, 3], "Testmaker", 32),
        car("instrument_model", &[2, 3], "Model 7", 32),
        car("instrument_serial_no", &[2, 3], "SN-0042", 32),
    ];
    let g = globali_ms(c);
    let _ = totale;
    cdf(c.v2, &dims, &g, &vars, totale as u32)
}

/// An ANDI-Chromatography file: `n` points of one FID trace, uniform sampling.
pub fn andi_cromato(n: usize) -> Vec<u8> {
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

pub fn apri_andi(byte: Vec<u8>) -> Box<dyn Lettore> {
    mzlab_lettori::apri(super::insieme("campione.cdf", byte), &Limiti::sito()).unwrap()
}

pub fn errore_andi(byte: Vec<u8>) -> &'static str {
    match mzlab_lettori::apri(super::insieme("campione.cdf", byte), &Limiti::sito()) {
        Err(e) => e.key,
        Ok(mut l) => {
            for k in 0..l.indice(0).map(|i| i.len()).unwrap_or(0) {
                if let Err(e) = l.scansione(0, k) {
                    return e.key;
                }
            }
            "ok"
        }
    }
}
