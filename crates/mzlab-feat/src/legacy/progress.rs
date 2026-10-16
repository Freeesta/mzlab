//! Progress of the Dig of mzFinder: the phases of a run with their names and weights, and the overall fraction done from `(phase, done, total)`.
//! The pages of the two engines (low and high resolution) call this and only draw what comes back; they never compute a fraction themselves.
//! The weights are a coarse model of where a run spends its time (measured on synthetic series of 9 and 38 files), so the fraction is an estimate:
//! it never goes down, reaches 1.0 only in the last phase, and the interface shows the time left as «about».

/// One phase: the key the page knows it by, its label, and the share of the whole run it stands for.
struct Phase {
    key: &'static str,
    label: &'static str,
    weight: f64,
}

/// Low resolution: detection of every file, hypotheses, alignment, unexpected ions, MS2 matches, tables.
const LR: [Phase; 6] = [
    Phase {
        key: "calibration",
        label: "Detecting peaks",
        weight: 0.08,
    },
    Phase {
        key: "candidates",
        label: "Generating candidates",
        weight: 0.30,
    },
    Phase {
        key: "xic",
        label: "Aligning peaks and extracting XICs",
        weight: 0.38,
    },
    Phase {
        key: "unexpected",
        label: "Searching for unexpected ions",
        weight: 0.04,
    },
    Phase {
        key: "ms2",
        label: "Looking for MS2 scans",
        weight: 0.12,
    },
    Phase {
        key: "tables",
        label: "Building the tables",
        weight: 0.08,
    },
];

/// High resolution: features of every file, alignment, candidates with their proofs, then the MSn tree of the standard.
const HR: [Phase; 4] = [
    Phase {
        key: "features",
        label: "Detecting peaks",
        weight: 0.35,
    },
    Phase {
        key: "align",
        label: "Aligning the series",
        weight: 0.15,
    },
    Phase {
        key: "candidates",
        label: "Scoring candidates",
        weight: 0.40,
    },
    Phase {
        key: "tree",
        label: "Building the MSn tree",
        weight: 0.10,
    },
];

/// `{"phase", "label", "done", "total", "frac"}` for a step of the phase `phase` of the engine `kind` (`"lr"` or `"hr"`): `done` of `total` units finished
/// (files, for the phases that go file by file; 0 of 0 for a phase that has no count), `frac` the overall fraction of the run, 0..1.
/// `"null"` when the engine or the phase is unknown.
pub fn json(kind: &str, phase: &str, done: u32, total: u32) -> String {
    let list: &[Phase] = match kind {
        "lr" => &LR,
        "hr" => &HR,
        _ => return "null".into(),
    };
    let Some(i) = list.iter().position(|p| p.key == phase) else {
        return "null".into();
    };
    let before: f64 = list[..i].iter().map(|p| p.weight).sum();
    let inside = if total == 0 {
        0.0
    } else {
        (done.min(total) as f64) / (total as f64)
    };
    let frac = (before + list[i].weight * inside).clamp(0.0, 1.0);
    let label = if total > 1 {
        format!(
            "{}: file {} of {}",
            list[i].label,
            done.min(total).max(1),
            total
        )
    } else {
        list[i].label.to_string()
    };
    format!(
        "{{\"phase\":\"{}\",\"label\":\"{}\",\"done\":{},\"total\":{},\"frac\":{:.4}}}",
        list[i].key, label, done, total, frac
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frac(kind: &str, phase: &str, done: u32, total: u32) -> f64 {
        let j = json(kind, phase, done, total);
        let k = j.find("\"frac\":").unwrap() + 7;
        j[k..j.len() - 1].parse().unwrap()
    }

    #[test]
    fn weights_add_up_to_one() {
        assert!((LR.iter().map(|p| p.weight).sum::<f64>() - 1.0).abs() < 1e-9);
        assert!((HR.iter().map(|p| p.weight).sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn never_goes_down_through_a_run() {
        for (kind, list) in [("lr", &LR[..]), ("hr", &HR[..])] {
            let mut last = -1.0;
            for p in list {
                for d in 0..=5 {
                    let f = frac(kind, p.key, d, 5);
                    assert!(f >= last, "{kind} {} {d}: {f} < {last}", p.key);
                    last = f;
                }
            }
            assert!((last - 1.0).abs() < 1e-3, "{kind} ends at {last}");
        }
    }

    #[test]
    fn file_by_file_phase_names_the_file() {
        let j = json("lr", "calibration", 3, 40);
        assert!(
            j.contains("\"label\":\"Detecting peaks: file 3 of 40\""),
            "{j}"
        );
        assert!(j.contains("\"done\":3,\"total\":40"), "{j}");
        // a phase with one job shows no «file 1 of 1», and a count above the total is held at the total
        assert!(
            json("lr", "xic", 0, 1).contains("\"label\":\"Aligning peaks and extracting XICs\"")
        );
        assert!((frac("lr", "calibration", 9, 4) - 0.08).abs() < 1e-9);
    }

    #[test]
    fn unknown_names_give_null() {
        assert_eq!(json("lr", "nope", 0, 0), "null");
        assert_eq!(json("py", "xic", 0, 0), "null");
    }
}
