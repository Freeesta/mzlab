//! Which `MethodN` a sample ran: the container stores one method per distinct acquisition method, not one per sample, so a batch that mixes
//! methods (MRM injections, then a full-scan blank) has fewer methods than samples and `MethodN` of sample N can be another method.
//! The sample names its `.dam` file in `SampleDABE/DATA`, each method in `AcqMethodFileInfoStm`: the two are matched by that file name.

use crate::ole::Ole;

/// The text of the UTF-16LE run that ends at `end` (exclusive), going back to the first control character, a backslash or the start.
fn run_before(b: &[u8], end: usize) -> String {
    let mut chars = Vec::new();
    let mut at = end;
    while at >= 2 {
        match (b.get(at - 2), b.get(at - 1)) {
            (Some(&lo), Some(&0)) if lo >= 0x20 && lo != b'\\' => chars.push(lo as char),
            _ => break,
        }
        at -= 2;
    }
    chars.iter().rev().collect()
}

/// The `.dam` file names (lower case, no folder) written as UTF-16LE in `b`, whatever the byte alignment.
fn dam_names(b: &[u8]) -> Vec<String> {
    let ext: Vec<u8> = ".dam".chars().flat_map(|c| [c as u8, 0]).collect();
    b.windows(ext.len())
        .enumerate()
        .filter(|(_, w)| w.eq_ignore_ascii_case(&ext))
        .map(|(at, _)| run_before(b, at + ext.len()).to_ascii_lowercase())
        .filter(|n| n.len() > 4)
        .collect()
}

/// The method of `sample` found by the name of its `.dam` file: its own `MethodN` when that one matches, else the first method that does.
/// `None` when the sample names no method file or no method has that file name.
pub(super) fn by_name(ole: &Ole, sample: u32) -> Option<u32> {
    let data = ole.read(&format!("SampleSubtree/Sample{sample}/SampleDABE/DATA"))?;
    let wanted = dam_names(&data);
    if wanted.is_empty() {
        return None;
    }
    let matches = |k: u32| {
        ole.read(&format!("MethodSubtree/Method{k}/AcqMethodFileInfoStm"))
            .is_some_and(|b| {
                dam_names(&b)
                    .iter()
                    .any(|n| wanted.iter().any(|w| w.ends_with(n.as_str())))
            })
    };
    if matches(sample) {
        return Some(sample);
    }
    (1..=256).find(|&k| matches(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(t: &str) -> Vec<u8> {
        t.bytes().flat_map(|c| [c, 0]).collect()
    }

    #[test]
    fn dam_names_end_at_the_extension_whatever_the_alignment() {
        // a folder before the name, a control character after it, and the same name one byte off the even alignment
        let mut b = utf16("C:\\Methods\\Quant_A.DAM");
        b.extend([1, 0]);
        b.push(0x55);
        b.extend(utf16("x\\Blank.dam"));
        assert_eq!(dam_names(&b), vec!["quant_a.dam", "blank.dam"]);
        // a bare extension is no name
        assert!(dam_names(&utf16(".dam")).is_empty());
    }
}
