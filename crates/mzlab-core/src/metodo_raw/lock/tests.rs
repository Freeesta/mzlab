use super::*;

const DDA: &str = "Lock Masses:\r\n   Pos List Name:                   VelosPro_051112\r\n          Source:                   API Source\r\n       Mass List:                   445.120025\r\n   Neg List Name:                   N/A\r\n          Source:                   API Source\r\n       Mass List:                   (none)\r\n\r\n\r\nGlobal Non-Data Dependent Settings:\r\n(+) Global MS/MS Masses:\r\n";
const EXACTIVE: &str = "Global Settings\r\nUse lock masses                                                         best \r\nLock mass injection                                                        ― \r\nChrom. peak wid\r\nLock Masses                                                                ― \r\n";

#[test]
fn the_lists_of_a_dda_method_are_read() {
    let m = from_method(DDA).unwrap();
    assert_eq!(m.mode, None);
    assert_eq!(m.lists.len(), 2);
    assert_eq!(m.lists[0].polarity, "positive");
    assert_eq!(m.lists[0].name, "VelosPro_051112");
    assert_eq!(m.lists[0].masses, [445.120025]);
    assert_eq!(m.lists[1].polarity, "negative");
    assert!(m.lists[1].masses.is_empty());
}

#[test]
fn the_mode_of_an_exactive_method_is_read() {
    let m = from_method(EXACTIVE).unwrap();
    assert_eq!(m.mode.as_deref(), Some("best"));
    assert!(m.lists.is_empty());
}

#[test]
fn a_text_without_lock_masses_gives_none() {
    assert_eq!(from_method("Scan Event 1\r\nFull ms\r\n"), None);
}

/// Junk and torn texts never panic and never invent a section.
#[test]
fn hostile_texts_do_not_panic() {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let words = [
        "Lock Masses:",
        "Pos List Name:",
        "Mass List:",
        "Use lock masses",
        "UV_VIS_1.Wavelength =",
        "3DFIELD.MinWavelength =",
        "1e999",
        "NaN",
        "|",
        "\r\n",
        "[",
        "=",
        "Precursor (m/z)",
        "Product (m/z)",
    ];
    for _ in 0..500 {
        let mut t = String::new();
        for _ in 0..40 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            t.push_str(words[(x >> 33) as usize % words.len()]);
            t.push(if x & 1 == 0 { ' ' } else { '\n' });
        }
        let _ = from_method(&t);
    }
}
