//! A MassLynx folder built in memory for the tests of the module `waters`: function 1 scans (8-byte records), function 2 is an MRM function with two channels (4-byte values).
//! The encoders here are the inverse of the ones in `src/waters/dat.rs` and `mrm.rs`; `tools/waters_sintetico.py` writes the same folder for the browser test.
#![allow(dead_code)]

use mzlab_core::lettori::{FileSet, Membro};

/// The m/z word of the 8- and 12-byte records: 5-bit exponent on top, 27-bit mantissa with the leading one at bit 26.
pub fn word_mz(mz: f64) -> u32 {
    let e = mz.log2().floor() as i32 + 1;
    let m = (mz / 2f64.powi(e) * f64::from(1u32 << 27)).round() as u32;
    ((e as u32) << 27) | (m & 0x07FF_FFFF)
}

/// The intensity word: 21-bit mantissa with the leading one at bit 20, 5-bit exponent at bit 22.
pub fn word_int(i: u32) -> u32 {
    let e = (31 - i.leading_zeros()) as i32 + 1;
    let m = ((f64::from(i) * 2f64.powi(21 - e)).round() as u32) & 0x001F_FFFF;
    ((e as u32) << 22) | m
}

/// One row of `_FUNCnnn.IDX`, variant A (22 bytes).
pub fn riga_idx(offset: u32, conta: u32, tic: f32, rt_min: f32) -> Vec<u8> {
    let mut r = vec![0u8; 22];
    r[0..4].copy_from_slice(&offset.to_le_bytes());
    r[4..8].copy_from_slice(&(conta | 0x1800_0000).to_le_bytes());
    r[8..12].copy_from_slice(&tic.to_le_bytes());
    r[12..16].copy_from_slice(&rt_min.to_le_bytes());
    r
}

/// A 416-byte record of `_FUNCTNS.INF`: type, subtype, cycle time and the 32 slots (Q1 or low mass at +0xA0, Q3 or high mass at +0x120, dwell at +0x20).
pub fn record_funzione(tipo: u8, sotto: u8, ciclo_s: f32, canali: &[(f32, f32, f32)]) -> Vec<u8> {
    let mut r = vec![0u8; 416];
    r[0] = tipo;
    r[1] = sotto;
    r[2..6].copy_from_slice(&ciclo_s.to_le_bytes());
    for (k, (q1, q3, dw)) in canali.iter().enumerate() {
        r[0x20 + 4 * k..0x24 + 4 * k].copy_from_slice(&dw.to_le_bytes());
        r[0xA0 + 4 * k..0xA4 + 4 * k].copy_from_slice(&q1.to_le_bytes());
        r[0x120 + 4 * k..0x124 + 4 * k].copy_from_slice(&q3.to_le_bytes());
    }
    r
}

/// The peaks of scan `k` of function 1: (m/z, intensity).
pub fn picchi(k: u32) -> Vec<(f64, u32)> {
    vec![
        (150.0 + f64::from(k), 1000 + 10 * k),
        (300.25, 5000),
        (450.5, 250 * (k + 1)),
    ]
}

/// The MRM values of scan `k` of function 2 (two channels).
pub fn canali_mrm(k: u32) -> [u32; 2] {
    [100 * (k + 1), 7 * k + 3]
}

pub const RT: [f32; 3] = [0.5, 1.0, 1.5];

fn membro(cartella: &str, nome: &str, byte: Vec<u8>) -> Membro {
    Membro {
        nome: format!("{cartella}/{nome}"),
        dim: byte.len() as u64,
        dati: Box::new(byte),
        sha1: None,
    }
}

/// The files of the folder as `(name, bytes)`.
pub fn file_cartella() -> Vec<(String, Vec<u8>)> {
    let mut funtns = record_funzione(0x12, 0x25, 1.0, &[(100.0, 1000.0, 0.0)]);
    funtns.extend(record_funzione(
        0x29,
        0x01,
        1.0,
        &[(200.1, 100.2, 0.05), (300.3, 150.4, 0.05)],
    ));
    let (mut dat1, mut idx1, mut dat2, mut idx2) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (k, rt) in RT.iter().enumerate() {
        let k = k as u32;
        let off = dat1.len() as u32;
        let mut tic = 0f32;
        for (mz, i) in picchi(k) {
            dat1.extend(word_int(i).to_le_bytes());
            dat1.extend(word_mz(mz).to_le_bytes());
            tic += i as f32;
        }
        idx1.extend(riga_idx(off, 3, tic, *rt));
        let off = dat2.len() as u32;
        let v = canali_mrm(k);
        for x in v {
            dat2.extend(((21u32 << 22) | x).to_le_bytes());
        }
        idx2.extend(riga_idx(off, 2, (v[0] + v[1]) as f32, *rt));
    }
    let header = "$$ Version: 01.00\r\n$$ Acquired Name: SYN_001\r\n$$ Acquired Date: 30-Apr-2022\r\n$$ Acquired Time: 00:26:58\r\n\
$$ Instrument: XEVO-TQXS#SYN0001\r\n$$ Sample Description: synthetic sample\r\n$$ Bottle Number: 1:34\r\n$$ User Name: Mario Rossi\r\n\
$$ Cal Function 1: 0,1,T1\r\n"
        .as_bytes()
        .to_vec();
    vec![
        ("_FUNCTNS.INF".into(), funtns),
        ("_FUNC001.DAT".into(), dat1),
        ("_FUNC001.IDX".into(), idx1),
        ("_FUNC002.DAT".into(), dat2),
        ("_FUNC002.IDX".into(), idx2),
        ("_HEADER.TXT".into(), header),
    ]
}

/// The folder `SYN_001.raw` as a `FileSet`.
pub fn insieme_sintetico() -> FileSet {
    FileSet {
        membri: file_cartella()
            .into_iter()
            .map(|(n, b)| membro("SYN_001.raw", &n, b))
            .collect(),
    }
}

/// A `FileSet` from `(name, bytes)` files.
pub fn insieme(file: Vec<(String, Vec<u8>)>) -> FileSet {
    FileSet {
        membri: file
            .into_iter()
            .map(|(n, b)| membro("X.raw", &n, b))
            .collect(),
    }
}

/// Every key of the `metodo/1` document and of its sections is one the schema knows; returns the parsed document.
pub fn controlla_schema(metodo: &str, etichetta: &str) -> serde_json::Value {
    let doc: serde_json::Value =
        serde_json::from_str(metodo).unwrap_or_else(|e| panic!("{etichetta}: {e}"));
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../../mzlab/web/schemi/metodo-1.json")).unwrap();
    for (key, value) in doc.as_object().unwrap() {
        let sezione = &schema["properties"][key.as_str()];
        assert!(
            !sezione.is_null(),
            "{etichetta}: {key} is not in the schema"
        );
        if let (Some(noti), Some(ha)) = (sezione["properties"].as_object(), value.as_object()) {
            for k in ha.keys() {
                assert!(
                    noti.contains_key(k),
                    "{etichetta}: {key}.{k} is not in the schema"
                );
            }
        }
    }
    doc
}
