//! `acqmeth.txt`: the instrument method the run was acquired with, a UTF-16 report. The MS part (mode, solvent delay, scan range, SIM groups) is read
//! into fields; the rest of the report (oven, inlet, column) is kept as text for the callers that want it.

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimGroup {
    pub id: String,
    pub start_min: f64,
    pub resolution: Option<f64>,
    /// (m/z, dwell in ms).
    pub ions: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcqMethod {
    /// The whole report, decoded.
    pub text: String,
    /// "Scan" or "SIM" as the method writes it.
    pub mode: String,
    pub solvent_delay_min: Option<f64>,
    pub tune_file: String,
    pub start_min: Option<f64>,
    pub low_mass: Option<f64>,
    pub high_mass: Option<f64>,
    pub threshold: Option<f64>,
    pub groups: Vec<SimGroup>,
}

/// UTF-16 with or without BOM (little endian when it has none), or plain UTF-8.
pub fn decode(b: &[u8]) -> String {
    let (le, body) = match b {
        [0xff, 0xfe, r @ ..] => (true, r),
        [0xfe, 0xff, r @ ..] => (false, r),
        [a, 0, ..] if *a != 0 => (true, b),
        _ => return String::from_utf8_lossy(b).into_owned(),
    };
    let u: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| {
            if le {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            }
        })
        .collect();
    String::from_utf16_lossy(&u)
}

fn kv(line: &str) -> Option<(&str, &str)> {
    let (k, v) = line.split_once(':')?;
    Some((k.trim(), v.trim()))
}

fn number(v: &str) -> Option<f64> {
    v.split_whitespace().next()?.parse().ok()
}

/// The `(mass, dwell)` pairs on a line of a SIM group.
fn pairs(line: &str) -> Vec<(f64, f64)> {
    line.split('(')
        .skip(1)
        .filter_map(|p| {
            let (a, b) = p.split(')').next()?.split_once(',')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .collect()
}

pub fn parse(bytes: &[u8]) -> AcqMethod {
    let text = decode(bytes);
    let mut m = AcqMethod {
        text: text.clone(),
        ..Default::default()
    };
    let mut in_ms = false;
    for line in text.lines() {
        if line.contains("MS Information") {
            in_ms = true;
        }
        if !in_ms {
            continue;
        }
        let t = line.trim();
        if t.starts_with("Group ") && t.contains("Group ID") {
            m.groups.push(SimGroup {
                id: kv(&t[t.find("Group ID").unwrap_or(0)..])
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default(),
                ..Default::default()
            });
            continue;
        }
        let Some(g) = m.groups.last_mut().filter(|_| {
            t.contains("Group Start Time")
                || t.contains("Resolution")
                || t.starts_with('(')
                || t.contains("Dwell In Group")
        }) else {
            if let Some((k, v)) = kv(t) {
                match k {
                    "Acquisition Mode" => m.mode = v.to_string(),
                    "Solvent Delay (minutes)" => m.solvent_delay_min = number(v),
                    "Tune file" => m.tune_file = v.to_string(),
                    "Start Time" => m.start_min = number(v),
                    "Low Mass" => m.low_mass = number(v),
                    "High Mass" => m.high_mass = number(v),
                    "Threshold" => m.threshold = number(v),
                    _ => {}
                }
            }
            continue;
        };
        if t.starts_with("Resolution") {
            g.resolution = kv(t).and_then(|(_, v)| number(v));
        } else if t.starts_with("Group Start Time") {
            g.start_min = kv(t).and_then(|(_, v)| number(v)).unwrap_or(0.0);
        } else {
            g.ions.extend(pairs(t));
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(t: &str) -> Vec<u8> {
        let mut b = vec![0xff, 0xfe];
        t.encode_utf16().for_each(|u| b.extend(u.to_le_bytes()));
        b
    }

    #[test]
    fn sim_groups_and_scan_range() {
        let sim = "oven\nHold Time 1 min\nMS Information\nAcquisition Mode         : SIM\nSolvent Delay (minutes)  : 4\nTune file                : D:\\x\\bfb.u\n\
[SIM Parameters]\nGroup 1               Group ID                  : A\nResolution                : 1\nGroup Start Time          : 4\nNumber of Ions            : 3\n\
Dwell In Group            :(  Mass,  Dwell) (  Mass,  Dwell)\n                           (  95.10,70  )  (  108.10,70  )\n                           (  135.10,50  )\n\n\
Group 2               Group ID                  : B\nResolution                : 1\nGroup Start Time          : 8.5\n                           (  111.00,70  )\n\n[MSZones]\nMS Source : 230 C\n";
        let m = parse(&utf16(sim));
        assert_eq!(
            (m.mode.as_str(), m.solvent_delay_min, m.tune_file.as_str()),
            ("SIM", Some(4.0), "D:\\x\\bfb.u")
        );
        assert_eq!(m.groups.len(), 2);
        assert_eq!(
            (
                m.groups[0].id.as_str(),
                m.groups[0].start_min,
                m.groups[0].resolution
            ),
            ("A", 4.0, Some(1.0))
        );
        assert_eq!(
            m.groups[0].ions,
            vec![(95.1, 70.0), (108.1, 70.0), (135.1, 50.0)]
        );
        assert_eq!(
            (m.groups[1].start_min, m.groups[1].ions.clone()),
            (8.5, vec![(111.0, 70.0)])
        );
        let scan = "MS Information\nAcquisition Mode         : Scan\nSolvent Delay (minutes)  : 3\n[Scan Parameters]\nStart Time               : 3\nLow Mass                 : 50\nHigh Mass                : 500\nThreshold                : 100\n";
        let m = parse(scan.as_bytes());
        assert_eq!(
            (
                m.mode.as_str(),
                m.start_min,
                m.low_mass,
                m.high_mass,
                m.threshold,
                m.groups.len()
            ),
            ("Scan", Some(3.0), Some(50.0), Some(500.0), Some(100.0), 0)
        );
    }
}
