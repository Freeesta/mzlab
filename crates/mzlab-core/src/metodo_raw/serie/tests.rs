use super::*;
use opentfraw::generic_data::GenericDataDescriptor;

fn d(t: GenericType, label: &str) -> GenericDataDescriptor {
    GenericDataDescriptor {
        field_type: t,
        length: 0,
        label: label.into(),
    }
}

fn rec(v: Vec<(&str, GenericValue)>) -> GenericRecord {
    GenericRecord {
        values: v.into_iter().map(|(k, x)| (k.to_string(), x)).collect(),
    }
}

/// Two sections; `Spray Voltage` and `Fore Vacuum` change, `Vaporizer` is constant, `Status` is text.
fn log() -> StatusLog {
    let r = |a: f64, b: f64| {
        rec(vec![
            ("=== Ion Source ===:", GenericValue::Gap),
            ("Spray Voltage (V):", GenericValue::Float64(a)),
            ("Vaporizer (°C):", GenericValue::Float64(300.0)),
            ("Status:", GenericValue::String("Ok".into())),
            ("=== Vacuum ===:", GenericValue::Gap),
            ("Fore Vacuum (mbar):", GenericValue::Float64(b)),
        ])
    };
    StatusLog {
        header: GenericDataHeader {
            fields: vec![
                d(GenericType::Gap, "=== Ion Source ===:"),
                d(GenericType::Float64, "Spray Voltage (V):"),
                d(GenericType::Float64, "Vaporizer (°C):"),
                d(GenericType::AsciiString, "Status:"),
                d(GenericType::Gap, "=== Vacuum ===:"),
                d(GenericType::Float64, "Fore Vacuum (mbar):"),
            ],
        },
        times_min: vec![0.0, 1.0, 2.5],
        records: vec![r(3400.0, 1.0), r(3500.0, 1.5), r(3500.0, 2.0)],
    }
}

#[test]
fn only_the_channels_that_change_are_series_with_their_section_and_unit() {
    let v = indice_stato(&log());
    let got: Vec<(&str, &str, &str, usize)> = v
        .iter()
        .map(|x| (x.id.as_str(), x.gruppo.as_str(), x.unita.as_str(), x.n))
        .collect();
    assert_eq!(
        got,
        [
            ("status:Ion Source/Spray Voltage", "Ion Source", "V", 3),
            ("status:Vacuum/Fore Vacuum", "Vacuum", "mbar", 3),
        ]
    );
    assert_eq!((v[0].min, v[0].max), (3400.0, 3500.0));
}

#[test]
fn the_points_of_a_status_series_carry_the_time_of_each_record() {
    let s = serie_stato(&log(), "status:Vacuum/Fore Vacuum").expect("series");
    assert_eq!(
        (s.unita.as_str(), s.t, s.y),
        ("mbar", vec![0.0, 1.0, 2.5], vec![1.0, 1.5, 2.0])
    );
    assert!(serie_stato(&log(), "status:Ion Source/Vaporizer").is_none());
}

#[test]
fn a_log_without_time_has_no_series() {
    let mut l = log();
    l.times_min.clear();
    assert!(indice_stato(&l).is_empty());
}

fn trailer() -> (GenericDataHeader, Vec<GenericRecord>, Vec<Scansione>) {
    let h = GenericDataHeader {
        fields: vec![
            d(GenericType::Float32, "Ion Injection Time (ms):"),
            d(GenericType::Float32, "AGC Target:"),
            d(GenericType::Int32, "Charge State:"),
            d(GenericType::Float32, "LM m/z-Correction (ppm):"),
        ],
    };
    // scans: MS1 MS2 MS2 MS1 MS2; the injection time changes at both levels, the charge at MS2 only, the AGC target never
    let r = |t: f32, c: i32, lm: f32| {
        rec(vec![
            ("Ion Injection Time (ms):", GenericValue::Float32(t)),
            ("AGC Target:", GenericValue::Float32(1e6)),
            ("Charge State:", GenericValue::Int32(c)),
            ("LM m/z-Correction (ppm):", GenericValue::Float32(lm)),
        ])
    };
    let recs = vec![
        r(10.0, 0, 1.0),
        r(50.0, 2, 1.0),
        r(60.0, 3, 1.0),
        r(12.0, 0, 1.0),
        r(70.0, 2, 1.0),
    ];
    let sc = |t_min, livello| Scansione { t_min, livello };
    let scans = vec![sc(0.1, 1), sc(0.2, 2), sc(0.3, 2), sc(0.4, 1), sc(0.5, 2)];
    (h, recs, scans)
}

#[test]
fn a_trailer_field_that_changes_at_several_levels_is_one_series_per_level() {
    let (h, recs, scans) = trailer();
    let v = indice_trailer(&h, &recs, &scans);
    let got: Vec<(&str, &str, usize)> = v
        .iter()
        .map(|x| (x.id.as_str(), x.nome.as_str(), x.n))
        .collect();
    assert_eq!(
        got,
        [
            (
                "trailer:Ion Injection Time@MS1",
                "Ion Injection Time (MS1)",
                2
            ),
            (
                "trailer:Ion Injection Time@MS2",
                "Ion Injection Time (MS2)",
                3
            ),
            ("trailer:Charge State", "Charge State", 3),
        ]
    );
}

#[test]
fn the_points_of_a_trailer_series_are_the_scans_of_its_level() {
    let (h, recs, scans) = trailer();
    let s = serie_trailer(&h, &recs, &scans, "trailer:Ion Injection Time@MS2").expect("series");
    assert_eq!(
        (s.unita.as_str(), s.t, s.y),
        ("ms", vec![0.2, 0.3, 0.5], vec![50.0, 60.0, 70.0])
    );
    let c = serie_trailer(&h, &recs, &scans, "trailer:Charge State").expect("series");
    assert_eq!((c.t, c.y), (vec![0.2, 0.3, 0.5], vec![2.0, 3.0, 2.0]));
    assert!(serie_trailer(&h, &recs, &scans, "trailer:AGC Target").is_none());
}

/// A stream laid out like a v66 file with two sections: a title, a changing channel, a constant one, a second title, a changing channel.
fn stream(count: u32) -> (Vec<u8>, u64) {
    let pascal = |t: &str| {
        let u: Vec<u16> = t.encode_utf16().collect();
        let mut b = (u.len() as u32).to_le_bytes().to_vec();
        u.iter().for_each(|x| b.extend(x.to_le_bytes()));
        b
    };
    let mut out = vec![0xAB; 40];
    let fields = [
        (0x0u32, 0u32, "=== Ion Source ===:"),
        (0xB, 1, "Spray Voltage (V):"),
        (0xB, 1, "Vaporizer (°C):"),
        (0x0, 0, "=== Vacuum ===:"),
        (0xB, 1, "Fore Vacuum (mbar):"),
    ];
    out.extend((fields.len() as u32).to_le_bytes());
    for (t, l, name) in fields {
        out.extend(t.to_le_bytes());
        out.extend(l.to_le_bytes());
        out.extend(pascal(name));
    }
    let addr = out.len() as u64;
    for i in 0..count {
        out.extend((i as f32 * 0.5).to_le_bytes());
        out.extend((3000.0 + f64::from(i)).to_le_bytes());
        out.extend(300.0f64.to_le_bytes());
        out.extend((1.0 + f64::from(i) / 4.0).to_le_bytes());
    }
    (out, addr)
}

#[test]
fn the_sections_of_a_log_read_from_bytes_give_the_series() {
    let (bytes, addr) = stream(4);
    let region = bytes.len() as u64 - addr;
    let log = opentfraw::status_log::decode(std::io::Cursor::new(&bytes), addr, 4, region)
        .expect("decoded");
    let ids: Vec<String> = indice_stato(&log).into_iter().map(|v| v.id).collect();
    assert_eq!(
        ids,
        [
            "status:Ion Source/Spray Voltage",
            "status:Vacuum/Fore Vacuum"
        ]
    );
    let s = serie_stato(&log, "status:Ion Source/Spray Voltage").expect("series");
    assert_eq!(
        (s.unita.as_str(), s.t, s.y),
        (
            "V",
            vec![0.0, 0.5, 1.0, 1.5],
            vec![3000.0, 3001.0, 3002.0, 3003.0]
        )
    );
}
