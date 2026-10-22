use crate::error::Result;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

/// Injection data sub-structure within SeqRow (64 bytes).
#[derive(Debug)]
pub struct InjectionData {
    pub row_number: u32,
    pub vial: String,
    pub injection_volume: f64,
    pub sample_weight: f64,
    pub sample_volume: f64,
    pub istd_amount: f64,
    pub dilution_factor: f64,
}

/// Sequence table row.
#[derive(Debug)]
pub struct SeqRow {
    pub injection: InjectionData,
    pub id: String,
    pub comment: String,
    pub user_labels: Vec<String>,
    pub inst_method: String,
    pub proc_method: String,
    pub file_name: String,
    pub path: String,
    pub vial: String,
    /// Numbers of the row whose meaning is not decoded, by position name (`unk1`, `unk2`, `unk_long`): kept so a dump can be matched against another reader's fields.
    pub unknown_numbers: Vec<(&'static str, u32)>,
    /// Texts of the row whose meaning is not decoded, by position name (`unk_a`, `unk_b`, `unk_c`, `unk_d`, `unk_e`…`unk_s`), empty ones included.
    pub unknown_texts: Vec<(&'static str, String)>,
}

const UNKNOWN_TEXT_TAIL: [&str; 15] = [
    "unk_e", "unk_f", "unk_g", "unk_h", "unk_i", "unk_j", "unk_k", "unk_l", "unk_m", "unk_n",
    "unk_o", "unk_p", "unk_q", "unk_r", "unk_s",
];

impl SeqRow {
    pub(crate) fn read<R: Read + Seek>(r: &mut BinaryReader<R>, version: u32) -> Result<Self> {
        // InjectionData: 64 bytes
        let unk1 = r.read_u32()?;
        let row_number = r.read_u32()?;
        let unk2 = r.read_u32()?;
        let vial_inj = r.read_utf16_fixed(12)?;
        let injection_volume = r.read_f64()?;
        let sample_weight = r.read_f64()?;
        let sample_volume = r.read_f64()?;
        let istd_amount = r.read_f64()?;
        let dilution_factor = r.read_f64()?;

        let injection = InjectionData {
            row_number,
            vial: vial_inj,
            injection_volume,
            sample_weight,
            sample_volume,
            istd_amount,
            dilution_factor,
        };

        // PascalString fields
        let unk_a = r.read_pascal_string()?;
        let unk_b = r.read_pascal_string()?;
        let id = r.read_pascal_string()?;
        let comment = r.read_pascal_string()?;
        let mut user_labels = Vec::new();
        for _ in 0..5 {
            user_labels.push(r.read_pascal_string()?);
        }
        let inst_method = r.read_pascal_string()?;
        let proc_method = r.read_pascal_string()?;
        let file_name = r.read_pascal_string()?;
        let path = r.read_pascal_string()?;
        let vial = r.read_pascal_string()?;

        let mut unknown_numbers = vec![("unk1", unk1), ("unk2", unk2)];
        let mut unknown_texts = vec![("unk_a", unk_a), ("unk_b", unk_b)];

        if version >= 57 {
            unknown_texts.push(("unk_c", r.read_pascal_string()?));
            unknown_texts.push(("unk_d", r.read_pascal_string()?));
            unknown_numbers.push(("unk_long", r.read_u32()?));
        }

        if version >= 60 {
            for name in UNKNOWN_TEXT_TAIL {
                unknown_texts.push((name, r.read_pascal_string()?));
            }
        }

        Ok(Self {
            injection,
            id,
            comment,
            user_labels,
            inst_method,
            proc_method,
            file_name,
            path,
            vial,
            unknown_numbers,
            unknown_texts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{pascal_string, utf16_fixed};
    use std::io::Cursor;

    /// A row with distinct values everywhere; the 14 pascal strings after the injection block are `t0`…`t13`
    /// (unk_a, unk_b, id, comment, 5 user labels, 3 methods/paths, vial).
    fn row_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        for n in [7u32, 3, 9] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend(utf16_fixed("1:A", 12));
        for x in [2.5f64, 1.0, 0.5, 4.0, 10.0] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        for n in 0..14 {
            b.extend(pascal_string(&format!("t{n}")));
        }
        b
    }

    #[test]
    fn v50_row_keeps_the_two_unknown_texts_and_numbers() {
        let b = row_bytes();
        let row = SeqRow::read(&mut BinaryReader::new(Cursor::new(b)), 50).unwrap();
        assert_eq!(row.injection.row_number, 3);
        assert_eq!(row.unknown_numbers, [("unk1", 7), ("unk2", 9)]);
        let names: Vec<_> = row.unknown_texts.iter().map(|(k, _)| *k).collect();
        assert_eq!(names, ["unk_a", "unk_b"]);
        assert_eq!(row.unknown_texts[1].1, "t1");
        assert_eq!(row.id, "t2");
    }

    #[test]
    fn v66_row_keeps_every_unknown_field_by_position() {
        // 14 texts up to `vial`, then unk_c, unk_d, the u32, then the 15 of v60+
        let mut b = row_bytes();
        b.extend(pascal_string("c"));
        b.extend(pascal_string("d"));
        b.extend_from_slice(&42u32.to_le_bytes());
        for n in 0..15 {
            b.extend(pascal_string(&format!("e{n}")));
        }
        let row = SeqRow::read(&mut BinaryReader::new(Cursor::new(b)), 66).unwrap();
        assert_eq!(row.unknown_numbers.last(), Some(&("unk_long", 42)));
        assert_eq!(row.unknown_texts.len(), 2 + 2 + 15);
        assert_eq!(row.unknown_texts[2], ("unk_c", "c".to_string()));
        assert_eq!(row.unknown_texts[18], ("unk_s", "e14".to_string()));
    }
}
