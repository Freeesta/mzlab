// The tests of trasporto.rs: what the module writes is what the engine reads, bit by bit.
use super::*;

fn voci() -> Vec<VoceScansione> {
    vec![
        VoceScansione {
            id: "scan=1 \"q\"\né".into(),
            rt: 0.1234567890123,
            livello: 2,
            polarita: -1,
            tic: 1e300,
            picco_base: None,
            filtro: "FTMS + c ESI Full ms2 445.12@hcd30.00 [50.00-500.00]".into(),
            precursore: Some(445.12),
            posizione: Some(Posizione {
                membro: 3,
                offset: 1 << 40,
                lunghezza: 77,
            }),
        },
        VoceScansione {
            id: String::new(),
            rt: f64::INFINITY,
            livello: 1,
            polarita: 0,
            tic: 0.1 + 0.2,
            picco_base: Some(200.25),
            filtro: String::new(),
            precursore: None,
            posizione: None,
        },
        VoceScansione {
            id: "scan=3".into(),
            rt: -0.0,
            livello: 255,
            polarita: 1,
            tic: 0.0,
            picco_base: Some(-1.5),
            filtro: "FTMS + c ESI Full ms2 445.12@hcd30.00 [50.00-500.00]".into(),
            precursore: Some(0.0),
            posizione: Some(Posizione {
                membro: 0,
                offset: 0,
                lunghezza: 0,
            }),
        },
    ]
}

#[test]
fn l_indice_torna_uguale() {
    let v = voci();
    let b = encode_indice(&v);
    assert_eq!(decode_indice(&b).unwrap(), v);
    // each filter text is written once
    assert_eq!(
        b.windows(4).filter(|w| *w == b"FTMS").count(),
        1,
        "the repeated filter is kept once"
    );
    assert_eq!(decode_indice(&encode_indice(&[])).unwrap(), vec![]);
}

#[test]
fn un_indice_troncato_o_di_un_altra_versione_si_rifiuta() {
    let b = encode_indice(&voci());
    for n in [0, 3, 23, 24, b.len() / 2, b.len() - 1] {
        assert!(decode_indice(&b[..n]).is_err(), "{n}");
    }
    let mut v1 = b.clone();
    v1[0] = 1;
    assert!(decode_indice(&v1).is_err());
    // a count that no block could hold is refused before allocating
    let mut enorme = b.clone();
    enorme[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_indice(&enorme).is_err());
}

fn scansione(wide: Option<Vec<f64>>) -> Spectrum {
    Spectrum {
        index: 7,
        id: "ignored".into(),
        level: 2,
        rt: 12.5,
        polarity: -1,
        profile: true,
        filter: "ignored".into(),
        precursor: Some(445.5),
        mz: vec![100.0, 200.5, 300.25],
        intensity: vec![1.5, 2.0, f32::NAN],
        wide,
    }
}

#[test]
fn la_scansione_torna_uguale_senza_id_e_filtro() {
    let s = scansione(None);
    let b = encode_scansione(&s);
    assert_eq!(b.len(), TESTA_SCANSIONE + 3 * 12);
    let d = decode_scansione(7, &b).unwrap();
    assert_eq!((d.id.as_str(), d.filter.as_str()), ("", ""));
    assert_eq!(
        (d.index, d.level, d.polarity, d.profile, d.rt, d.precursor),
        (7, 2, -1, true, 12.5, Some(445.5))
    );
    assert_eq!(d.mz, s.mz);
    assert_eq!(d.intensity[..2], s.intensity[..2]);
    assert!(d.intensity[2].is_nan());
    assert!(d.wide.is_none());
    let senza = Spectrum {
        precursor: None,
        profile: false,
        ..s
    };
    let d = decode_scansione(0, &encode_scansione(&senza)).unwrap();
    assert_eq!((d.precursor, d.profile), (None, false));
}

#[test]
fn le_intensita_larghe_restano_larghe_solo_se_servono() {
    let w = vec![1.5, 16777217.0, 3.0];
    let s = Spectrum {
        intensity: w.iter().map(|&x| x as f32).collect(),
        ..scansione(Some(w.clone()))
    };
    let b = encode_scansione(&s);
    assert_eq!(b.len(), TESTA_SCANSIONE + 3 * 16);
    assert_eq!(decode_scansione(0, &b).unwrap().wide, Some(w));
    // 64-bit values that a 32-bit one holds exactly are not kept wide
    let esatte = vec![1.5, 2.0, 3.0];
    let s = Spectrum {
        intensity: esatte.iter().map(|&x| x as f32).collect(),
        ..scansione(Some(esatte))
    };
    assert!(decode_scansione(0, &encode_scansione(&s))
        .unwrap()
        .wide
        .is_none());
}

#[test]
fn una_scansione_di_misura_sbagliata_si_rifiuta() {
    let b = encode_scansione(&scansione(None));
    assert!(decode_scansione(0, &b[..b.len() - 1]).is_err());
    assert!(decode_scansione(0, &b[..10]).is_err());
    let mut lungo = b.clone();
    lungo.push(0);
    assert!(decode_scansione(0, &lungo).is_err());
    let mut enorme = b;
    enorme[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_scansione(0, &enorme).is_err());
    let vuota = Spectrum {
        mz: vec![],
        intensity: vec![],
        ..scansione(None)
    };
    assert_eq!(
        decode_scansione(0, &encode_scansione(&vuota))
            .unwrap()
            .mz
            .len(),
        0
    );
}
