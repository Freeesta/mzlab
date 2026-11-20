//! Which bytes of a dataset a chosen set of samples needs (`sample-select/1`): the site copies only these ranges of a big `.wiff.scan` into OPFS, the app and
//! `mzlab-cli sample select --ranges` print the same plan. One planner for the three, so the bytes agree. A member with no ranges of its own is copied whole.

use super::selection;
use super::Listing;
use crate::error::Result;
use crate::json::Out;

/// One member of the dataset: its name and size, and the `[start, end)` ranges to copy (the whole member unless it is the data file of a pair).
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub name: String,
    pub size: u64,
    pub ranges: Vec<(u64, u64)>,
}

/// The plan: the chosen ids, the members with their ranges, and a short tag that is the same for the same ranges (the site names the partial copy with it).
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub samples: Vec<u32>,
    pub members: Vec<Member>,
}

/// Sorted, touching or overlapping ranges joined, empty ones dropped.
fn merge(mut v: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    v.retain(|r| r.1 > r.0);
    crate::sortc::slice_ord(&mut v);
    let mut out: Vec<(u64, u64)> = Vec::with_capacity(v.len());
    for r in v {
        match out.last_mut() {
            Some(l) if r.0 <= l.1 => l.1 = l.1.max(r.1),
            _ => out.push(r),
        }
    }
    out
}

/// The plan for `ids` (already checked or not: an id the file lacks is `err.sample.unknown`, exit 5) over `members` (name, size) of a dataset that `facts` lists.
/// Without section positions (not a `.wiff`) every member is copied whole. For a `.wiff.scan`: the bytes before the first section (header), the sections of
/// the chosen samples, and the bytes after the indexed part (sections the index lacks, padding); the other samples' sections are left out.
pub fn plan(facts: &Listing, members: &[(String, u64)], ids: &[u32]) -> Result<Plan> {
    let present: Vec<u32> = facts.sections.iter().map(|s| s.0).collect();
    let chosen = if present.is_empty() {
        ids.to_vec()
    } else {
        selection::resolve(&present, ids)?
    };
    let whole = |m: &(String, u64)| Member {
        name: m.0.clone(),
        size: m.1,
        ranges: if m.1 > 0 { vec![(0, m.1)] } else { vec![] },
    };
    let data = facts.scan.and_then(|(len, end)| {
        let at = members
            .iter()
            .position(|m| m.0.to_ascii_lowercase().ends_with(".scan") && m.1 == len)?;
        Some((at, len, end))
    });
    let out = members
        .iter()
        .enumerate()
        .map(|(i, m)| match data {
            Some((at, len, end)) if at == i && !facts.sections.is_empty() => {
                let real: Vec<_> = facts.sections.iter().filter(|s| s.2 > s.1).collect();
                let first = real.iter().map(|s| s.1).min().unwrap_or(len);
                let mut r = vec![(0, first), (end.min(len), len)];
                r.extend(
                    real.iter()
                        .filter(|s| chosen.contains(&s.0))
                        .map(|s| (s.1, s.2)),
                );
                Member {
                    ranges: merge(r),
                    ..whole(m)
                }
            }
            _ => whole(m),
        })
        .collect();
    Ok(Plan {
        samples: chosen,
        members: out,
    })
}

impl Plan {
    /// Bytes this plan copies, and bytes of the whole dataset.
    pub fn bytes(&self) -> (u64, u64) {
        let kept = self
            .members
            .iter()
            .flat_map(|m| &m.ranges)
            .map(|r| r.1 - r.0);
        (kept.sum(), self.members.iter().map(|m| m.size).sum())
    }

    /// A short hex name of the ranges (FNV-1a 64): the same ranges give the same tag, another choice another one.
    pub fn tag(&self) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for m in &self.members {
            for b in m.name.bytes().chain(m.size.to_le_bytes()) {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
            for r in &m.ranges {
                for b in r.0.to_le_bytes().into_iter().chain(r.1.to_le_bytes()) {
                    h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
                }
            }
        }
        format!("{h:016x}")
    }

    /// One line per member for a terminal: `name size start-end,start-end`.
    pub fn text(&self) -> String {
        let mut t = String::new();
        for m in &self.members {
            let r: Vec<String> = m
                .ranges
                .iter()
                .map(|r| format!("{}-{}", r.0, r.1))
                .collect();
            t += &format!("{} {} {}\n", m.name, m.size, r.join(","));
        }
        t
    }

    /// `sample-select/1`: `{"schema","samples":[ids],"bytes","total","tag","members":[{"name","size","ranges":[[start,end],..]}]}`.
    pub fn json(&self) -> String {
        let (kept, total) = self.bytes();
        let mut o = Out::default();
        o.raw("{\"schema\":\"sample-select/1\",\"samples\":[");
        for (i, s) in self.samples.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).int(i64::from(*s));
        }
        o.raw("],\"bytes\":").int(kept as i64);
        o.raw(",\"total\":").int(total as i64);
        o.raw(",\"tag\":").str(&self.tag());
        o.raw(",\"members\":[");
        for (i, m) in self.members.iter().enumerate() {
            o.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
                .str(&m.name);
            o.raw(",\"size\":").int(m.size as i64).raw(",\"ranges\":[");
            for (j, r) in m.ranges.iter().enumerate() {
                o.raw(if j > 0 { ",[" } else { "[" })
                    .int(r.0 as i64)
                    .raw(",")
                    .int(r.1 as i64)
                    .raw("]");
            }
            o.raw("]}");
        }
        o.raw("]}");
        o.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Listing {
        Listing {
            scan: Some((1000, 900)),
            sections: vec![(1, 100, 400), (2, 400, 700), (3, 700, 900)],
            ..Listing::default()
        }
    }
    fn members() -> Vec<(String, u64)> {
        vec![("a.wiff".into(), 50), ("a.wiff.scan".into(), 1000)]
    }

    #[test]
    fn keeps_header_chosen_sections_and_tail_and_joins_touching_ones() {
        let p = plan(&facts(), &members(), &[1, 3]).unwrap();
        assert_eq!(p.members[0].ranges, vec![(0, 50)]);
        assert_eq!(p.members[1].ranges, vec![(0, 400), (700, 1000)]);
        assert_eq!(p.bytes(), (50 + 400 + 300, 1050));
    }

    #[test]
    fn all_samples_is_the_whole_file_and_an_unknown_id_is_refused() {
        let p = plan(&facts(), &members(), &[1, 2, 3]).unwrap();
        assert_eq!(p.members[1].ranges, vec![(0, 1000)]);
        let e = plan(&facts(), &members(), &[9]).unwrap_err();
        assert_eq!(e.key, "err.sample.unknown");
    }

    #[test]
    fn no_sections_copies_every_member_whole_and_the_tag_follows_the_choice() {
        let p = plan(&Listing::default(), &members(), &[]).unwrap();
        assert_eq!(p.bytes().0, p.bytes().1);
        let a = plan(&facts(), &members(), &[1]).unwrap();
        let b = plan(&facts(), &members(), &[2]).unwrap();
        assert_ne!(a.tag(), b.tag());
        assert_eq!(a.tag(), plan(&facts(), &members(), &[1]).unwrap().tag());
    }
}
