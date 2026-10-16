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
#[path = "progress_tests.rs"]
mod tests;
