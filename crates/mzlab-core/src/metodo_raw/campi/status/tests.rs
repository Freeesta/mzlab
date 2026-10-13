use super::*;
use opentfraw::generic_data::{GenericDataDescriptor, GenericDataHeader, GenericRecord};
use opentfraw::GenericType;

fn log() -> StatusLog {
    let d = |t, label: &str| GenericDataDescriptor {
        field_type: t,
        length: 0,
        label: label.into(),
    };
    let rec = |a: f64, b: &str, c: f64| GenericRecord {
        values: vec![
            ("=== Ion Source ===:".into(), GenericValue::Gap),
            ("Spray Voltage (V):".into(), GenericValue::Float64(a)),
            ("Status:".into(), GenericValue::String(b.into())),
            ("Fore Vacuum (mbar):".into(), GenericValue::Float64(c)),
        ],
    };
    StatusLog {
        header: GenericDataHeader {
            fields: vec![
                d(GenericType::Gap, "=== Ion Source ===:"),
                d(GenericType::Float64, "Spray Voltage (V):"),
                d(GenericType::AsciiString, "Status:"),
                d(GenericType::Float64, "Fore Vacuum (mbar):"),
            ],
        },
        times_min: vec![0.0, 1.5],
        records: vec![rec(3400.5, "Ok", 1.25), rec(3500.0, "Ok", 1.25)],
    }
}

#[test]
fn a_channel_is_its_value_when_constant_else_its_range_with_the_unit() {
    let f = fields(&log());
    let get = |k: &str| f.iter().find(|c| c.chiave == k);
    let v = get("Spray Voltage (min..max)").expect("range");
    assert_eq!(
        (v.valore.as_str(), v.unita.as_deref()),
        ("3400.5..3500", Some("V"))
    );
    let s = get("Status").expect("text");
    assert_eq!((s.valore.as_str(), s.unita.as_deref()), ("Ok", None));
    let p = get("Fore Vacuum").expect("constant");
    assert_eq!(
        (p.valore.as_str(), p.unita.as_deref()),
        ("1.25", Some("mbar"))
    );
    assert_eq!(get("Records").map(|c| c.valore.as_str()), Some("2"));
    assert_eq!(
        get("Time (min..max)").map(|c| c.valore.as_str()),
        Some("0..1.5")
    );
    assert!(f.iter().all(|c| !c.chiave.contains("Ion Source")));
}

#[test]
fn the_series_carry_name_unit_and_time_and_skip_the_texts() {
    let s = log().series();
    let names: Vec<_> = s
        .iter()
        .map(|x| (x.name.as_str(), x.unit.as_str()))
        .collect();
    assert_eq!(names, [("Spray Voltage", "V"), ("Fore Vacuum", "mbar")]);
    assert_eq!(s[0].time_min, [0.0, 1.5]);
    assert_eq!(s[0].values, [3400.5, 3500.0]);
}

/// A stream laid out like a v66 file: padding, the schema, then `count` records (`f32` time + fields).
fn stream(count: u32) -> (Vec<u8>, u64) {
    let pascal = |t: &str| {
        let u: Vec<u16> = t.encode_utf16().collect();
        let mut b = (u.len() as u32).to_le_bytes().to_vec();
        u.iter().for_each(|x| b.extend(x.to_le_bytes()));
        b
    };
    let mut out = vec![0xAB; 40];
    let fields = [(0xBu32, 1u32, "Spray Voltage (V):"), (0xC, 8, "Status:")];
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
        out.extend(b"Ok\0\0\0\0\0\0");
    }
    (out, addr)
}

#[test]
fn a_schema_that_ends_at_the_records_is_found_and_the_records_read() {
    let (bytes, addr) = stream(3);
    let region = bytes.len() as u64 - addr;
    let log = opentfraw::status_log::decode(std::io::Cursor::new(&bytes), addr, 3, region)
        .expect("decoded");
    assert_eq!((log.records.len(), log.header.fields.len()), (3, 2));
    assert_eq!(log.times_min, [0.0, 0.5, 1.0]);
    let s = log.series();
    assert_eq!(
        (
            s[0].name.as_str(),
            s[0].unit.as_str(),
            s[0].values.as_slice()
        ),
        ("Spray Voltage", "V", &[3000.0, 3001.0, 3002.0][..])
    );
    let f = fields(&log);
    assert!(f.iter().any(|c| c.chiave == "Status" && c.valore == "Ok"));
}

#[test]
fn without_a_schema_or_with_records_that_do_not_fit_nothing_is_decoded() {
    let (bytes, addr) = stream(3);
    let decode =
        |a: u64, n: u32| opentfraw::status_log::decode(std::io::Cursor::new(&bytes), a, n, 0);
    assert!(decode(addr - 6, 3).is_none(), "no schema ends there");
    assert!(decode(addr, 1_000_000).is_none(), "more records than bytes");
    assert!(decode(addr, 0).is_none());
}
