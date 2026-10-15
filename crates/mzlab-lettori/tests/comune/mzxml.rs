//! The synthetic mzXML files of the tests of the module `aperti` (ISB mzXML 3.2 and the nested form of 2.x).
#![allow(dead_code)]

use super::{b64, zlib};

/// One scan of the synthetic run (and the scans nested in it, for the nested form).
#[derive(Clone)]
pub struct Sc {
    pub num: u32,
    pub livello: u8,
    pub rt_s: f64,
    pub pol: &'static str,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub prec: Option<f64>,
    pub tipo: &'static str,
    pub figli: Vec<Sc>,
}

#[derive(Clone, Copy)]
pub struct Opz {
    pub a64: bool,
    pub zlib: bool,
    pub indice: bool,
    pub annidato: bool,
    /// Write `totIonCurrent`, `basePeakIntensity` and `peaksCount`.
    pub attributi: bool,
}

pub const PIATTO: Opz = Opz {
    a64: true,
    zlib: true,
    indice: true,
    annidato: false,
    attributi: true,
};

pub fn sc(num: u32, livello: u8, rt_s: f64, n: usize, prec: Option<f64>) -> Sc {
    let k = num as f64;
    Sc {
        num,
        livello,
        rt_s,
        pol: "+",
        mz: (0..n).map(|i| 100.0 + i as f64 * 2.5 + k * 0.01).collect(),
        it: (0..n)
            .map(|i| 10.0 + ((i * 37 + num as usize * 11) % 101) as f64 * 3.0)
            .collect(),
        prec,
        tipo: "Full",
        figli: Vec::new(),
    }
}

/// The scans of a small run: MS1, MS2, MS2, MS1, MS2.
pub fn corsa() -> Vec<Sc> {
    vec![
        sc(1, 1, 6.0, 30, None),
        sc(2, 2, 6.4, 12, Some(412.25)),
        sc(3, 2, 6.8, 9, Some(520.5)),
        sc(4, 1, 12.0, 33, None),
        sc(5, 2, 12.4, 10, Some(412.25)),
    ]
}

pub const TESTATA: &str = "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<mzXML xmlns=\"http://sashimi.sourceforge.net/schema_revision/mzXML_3.2\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n\
<msRun scanCount=\"5\" startTime=\"PT6S\" endTime=\"PT12.4S\">\n\
<parentFile fileName=\"file:///C:/dati/campione.raw\" fileType=\"RAWData\" fileSha1=\"0123456789abcdef0123456789abcdef01234567\"/>\n\
<msInstrument msInstrumentID=\"1\"><msManufacturer category=\"msManufacturer\" value=\"Thermo Scientific\"/><msModel category=\"msModel\" value=\"Q Exactive\"/>\
<msIonisation category=\"msIonisation\" value=\"ESI\"/><msMassAnalyzer category=\"msMassAnalyzer\" value=\"Orbitrap\"/><msDetector category=\"msDetector\" value=\"inductive detector\"/>\
<software type=\"acquisition\" name=\"Xcalibur\" version=\"4.1\"/><operator first=\"Mario\" last=\"Verdi\"/></msInstrument>\n\
<dataProcessing centroided=\"1\"><software type=\"conversion\" name=\"msconvert\" version=\"3.0\"/></dataProcessing>\n";

fn picchi(mz: &[f64], it: &[f64], o: Opz) -> String {
    let mut b = Vec::new();
    for (x, y) in mz.iter().zip(it) {
        if o.a64 {
            b.extend(x.to_be_bytes());
            b.extend(y.to_be_bytes());
        } else {
            b.extend((*x as f32).to_be_bytes());
            b.extend((*y as f32).to_be_bytes());
        }
    }
    let dati = if o.zlib { zlib(&b) } else { b };
    format!(
        "<peaks precision=\"{}\" byteOrder=\"network\" contentType=\"m/z-int\" compressionType=\"{}\" compressedLen=\"{}\">{}</peaks>",
        if o.a64 { 64 } else { 32 },
        if o.zlib { "zlib" } else { "none" },
        if o.zlib { dati.len() } else { 0 },
        b64(&dati)
    )
}

/// The `<scan>` element of `x`, its nested scans inside when `o.annidato`; `pos` collects the byte offset of every scan.
fn emetti(s: &mut String, x: &Sc, o: Opz, pos: &mut Vec<usize>) {
    pos.push(s.len());
    let tic: f64 = x.it.iter().sum();
    let bpi = x.it.iter().cloned().fold(0.0, f64::max);
    let extra = if o.attributi {
        format!(
            " peaksCount=\"{}\" totIonCurrent=\"{tic}\" basePeakIntensity=\"{bpi}\"",
            x.mz.len()
        )
    } else {
        String::new()
    };
    s.push_str(&format!(
        "<scan num=\"{}\" scanType=\"{}\" centroided=\"1\" msLevel=\"{}\" polarity=\"{}\" retentionTime=\"PT{}S\" lowMz=\"{}\" highMz=\"{}\"{extra}>",
        x.num,
        x.tipo,
        x.livello,
        x.pol,
        x.rt_s,
        x.mz.first().copied().unwrap_or(0.0),
        x.mz.last().copied().unwrap_or(0.0)
    ));
    if let Some(p) = x.prec {
        s.push_str(&format!(
            "<precursorMz precursorIntensity=\"1000\" precursorCharge=\"2\" activationMethod=\"CID\">{p}</precursorMz>"
        ));
    }
    s.push_str(&picchi(&x.mz, &x.it, o));
    s.push_str("<nameValue name=\"ion injection time\" value=\"12.5\"/>");
    if o.annidato {
        for f in &x.figli {
            emetti(s, f, o, pos);
        }
        s.push_str("</scan>\n");
    } else {
        s.push_str("</scan>\n");
        for f in &x.figli {
            emetti(s, f, o, pos);
        }
    }
}

/// The mzXML of `scans`, with `<index>` and `<indexOffset>` when `o.indice`.
pub fn mzxml(scans: &[Sc], o: Opz) -> Vec<u8> {
    let mut s = String::from(TESTATA);
    let mut pos = Vec::new();
    for x in scans {
        emetti(&mut s, x, o, &mut pos);
    }
    s.push_str("</msRun>\n");
    if o.indice {
        let inizio = s.len();
        s.push_str("<index name=\"scan\">\n");
        for (i, p) in pos.iter().enumerate() {
            s.push_str(&format!("<offset id=\"{}\">{p}</offset>\n", i + 1));
        }
        s.push_str(&format!(
            "</index>\n<indexOffset>{inizio}</indexOffset>\n<sha1>0000</sha1>\n"
        ));
    }
    s.push_str("</mzXML>\n");
    s.into_bytes()
}

/// `scans` with the MS2 scans that follow an MS1 moved inside it (the nested form of mzXML 2).
pub fn annida(scans: &[Sc]) -> Vec<Sc> {
    let mut out: Vec<Sc> = Vec::new();
    for x in scans {
        match (x.livello, out.last_mut()) {
            (2, Some(p)) => p.figli.push(x.clone()),
            _ => out.push(x.clone()),
        }
    }
    out
}

pub fn apri_mzxml(byte: Vec<u8>) -> Box<dyn mzlab_core::lettori::Lettore> {
    mzlab_lettori::apri(
        super::insieme("campione.mzXML", byte),
        &mzlab_core::lettori::Limiti::sito(),
    )
    .unwrap()
}

pub fn errore_mzxml(byte: Vec<u8>) -> &'static str {
    match mzlab_lettori::apri(
        super::insieme("campione.mzXML", byte),
        &mzlab_core::lettori::Limiti::sito(),
    ) {
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
