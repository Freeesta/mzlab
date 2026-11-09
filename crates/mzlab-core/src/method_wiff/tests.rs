use super::*;

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect()
}

#[test]
fn counted_strings_are_read_between_binary_fields() {
    let mut b = vec![0u8; 12];
    for s in ["t60", "Lab\\B\\MS2.wiff", "x.dam"] {
        b.extend((2 * s.encode_utf16().count() as u16).to_le_bytes());
        b.extend(utf16(s));
        b.extend([0, 0, 7, 0]);
    }
    assert_eq!(counted_strings(&b), ["t60", "Lab\\B\\MS2.wiff", "x.dam"]);
}

#[test]
fn log_records_drop_the_length_byte_and_join_the_blocks_of_1024() {
    let long: String = (0..1024)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    let mut b = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    b.extend(utf16("6Software Application::0:"));
    b.extend([0, 0, 0xc2, 2, 0, 0]);
    b.extend(utf16(
        &format!("Shimadzu Instrument driver:User=\\:Computer=PC1,{long}")[..1024],
    ));
    b.extend([0, 0, 0x78, 0x69, 0, 0, 8, 0]);
    b.extend(utf16(",tail of the record"));
    let r = log_records(&b);
    assert_eq!(r[0], "Software Application::0:");
    assert_eq!(r.len(), 2);
    assert!(
        r[1].starts_with("Shimadzu Instrument driver:User=")
            && r[1].ends_with(",tail of the record"),
        "{}",
        r[1]
    );
}

#[test]
fn the_lc_record_gives_sections_and_values_with_units() {
    let e = driver_entries("Shimadzu Instrument driver:User=\\:Computer=PC1,     Injection Volume:30 uL,Pumps:(LC-20ADXR),     Total Flow:0.25 mL/min,     P.Max 1:8000 psi,,Oven (CTO-20A),     Temperature:40 ");
    assert!(e
        .iter()
        .any(|(s, k, v)| s.starts_with("Pumps") && k == "P.Max 1" && v == "8000 psi"));
    assert!(e
        .iter()
        .any(|(s, k, _)| s.starts_with("Oven") && k == "Temperature"));
    assert_eq!(
        value_unit("0.25 mL/min"),
        Some((0.25, "mL/min".to_string()))
    );
    assert_eq!(value_unit("Yes"), None);
}

#[test]
fn the_creation_time_of_a_property_set_is_a_local_timestamp() {
    // 2025-11-11 13:15:22 as a FILETIME
    let ft = (1_762_866_922u64 + 11_644_473_600) * 10_000_000;
    assert_eq!(filetime(ft), "2025-11-11T13:15:22");
    let mut b = vec![0u8; 0x30];
    b[0x2c..0x30].copy_from_slice(&0x30u32.to_le_bytes());
    b.extend([0u8; 4]);
    b.extend(1u32.to_le_bytes());
    b.extend(12u32.to_le_bytes());
    b.extend(16u32.to_le_bytes());
    b.extend(64u32.to_le_bytes());
    b.extend(ft.to_le_bytes());
    assert_eq!(
        properties(&b),
        vec![(12, "2025-11-11T13:15:22".to_string())]
    );
}
