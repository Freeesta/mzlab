//! The label of a sample of a multi-sample file (`features --sample`): the column of the table. The sample name, never the file name, with the extension of
//! its file kept so that every table cuts that and not a dot of the name; members whose names collide take `NAME (FILE #ID)`. Pure, so the Flow uses the same rule.

/// One member of a series taken from a multi-sample file: its file name, sample id and sample name.
pub(crate) struct Member<'a> {
    pub file: &'a str,
    pub id: u32,
    pub sample: &'a str,
}

fn extension(file: &str) -> &str {
    file.rsplit_once('.')
        .filter(|(head, _)| !head.is_empty())
        .map_or("", |(_, e)| e)
}

fn base(file: &str) -> &str {
    file.rsplit_once('.')
        .filter(|(head, _)| !head.is_empty())
        .map_or(file, |(h, _)| h)
}

fn with_extension(name: &str, file: &str) -> String {
    match extension(file) {
        "" => name.to_string(),
        e => format!("{name}.{e}"),
    }
}

/// The labels of `members` (one per member, in order) and whether each had to take the tie-break (`NAME (FILE #ID)`). An empty sample name is `FILE #ID`.
pub(crate) fn labels(members: &[Member]) -> Vec<(String, bool)> {
    let plain: Vec<String> = members
        .iter()
        .map(|m| {
            let n = m.sample.trim();
            if n.is_empty() {
                format!("{} #{}", base(m.file), m.id)
            } else {
                n.to_string()
            }
        })
        .collect();
    let mut out: Vec<(String, bool)> = members
        .iter()
        .zip(&plain)
        .map(|(m, p)| {
            let clash = plain.iter().filter(|q| q.as_str() == p).count() > 1;
            if clash {
                (
                    with_extension(&format!("{p} ({} #{})", base(m.file), m.id), m.file),
                    true,
                )
            } else {
                (with_extension(p, m.file), false)
            }
        })
        .collect();
    // the same file given twice (two folders): the later ones are numbered, so no two columns share a name
    for k in 1..out.len() {
        let seen = out[..k].iter().filter(|(n, _)| *n == out[k].0).count();
        if seen > 0 {
            let (n, tie) = &out[k];
            let name = n
                .rsplit_once('.')
                .map_or(format!("{n} [{}]", seen + 1), |(h, e)| {
                    format!("{h} [{}].{e}", seen + 1)
                });
            out[k] = (name, *tie);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m<'a>(file: &'a str, id: u32, sample: &'a str) -> Member<'a> {
        Member { file, id, sample }
    }

    #[test]
    fn a_sample_name_is_the_label_and_the_file_keeps_only_its_extension() {
        let l = labels(&[m("run.wiff", 1, "Sample_A"), m("run.wiff", 2, "Sample 1.5")]);
        assert_eq!(l[0], ("Sample_A.wiff".to_string(), false));
        assert_eq!(l[1], ("Sample 1.5.wiff".to_string(), false));
    }

    #[test]
    fn names_that_collide_take_the_file_and_the_id_and_the_others_do_not() {
        let l = labels(&[
            m("a.wiff", 1, "S"),
            m("b.wiff", 1, "S"),
            m("a.wiff", 2, "T"),
            m("a.wiff", 3, "T"),
        ]);
        let names: Vec<&str> = l.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "S (a #1).wiff",
                "S (b #1).wiff",
                "T (a #2).wiff",
                "T (a #3).wiff"
            ]
        );
        assert!(l.iter().all(|(_, tie)| *tie));
        let l = labels(&[m("a.wiff", 1, "S"), m("b.wiff", 1, "U")]);
        assert_eq!(
            l,
            [("S.wiff".to_string(), false), ("U.wiff".to_string(), false)]
        );
    }

    #[test]
    fn one_file_twice_is_numbered_so_no_name_repeats() {
        let l = labels(&[m("a.wiff", 1, "S"), m("a.wiff", 1, "S")]);
        assert_eq!(l[0].0, "S (a #1).wiff");
        assert_eq!(l[1].0, "S (a #1) [2].wiff");
    }

    #[test]
    fn an_empty_name_falls_back_to_the_file_and_the_id() {
        let l = labels(&[m("a.wiff", 3, " ")]);
        assert_eq!(l[0].0, "a #3.wiff");
    }
}

#[cfg(test)]
#[path = "features_label_tests.rs"]
mod cli_tests;
