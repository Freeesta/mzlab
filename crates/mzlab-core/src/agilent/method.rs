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

/// One rate of the oven program.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OvenRamp {
    pub rate_c_min: f64,
    pub to_c: f64,
    pub hold_min: f64,
}

/// The GC part of the method report, in the units the report uses (what is not in the report stays `None`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GcMethod {
    pub oven_initial_c: Option<f64>,
    pub oven_hold_min: Option<f64>,
    pub oven_ramps: Vec<OvenRamp>,
    pub equilibration_min: Option<f64>,
    pub run_time_min: Option<f64>,
    /// "Front SSZ Inlet He": the inlet and its gas as the report names them.
    pub inlet: String,
    pub inlet_mode: String,
    pub inlet_heater_c: Option<f64>,
    pub inlet_pressure_psi: Option<f64>,
    pub inlet_total_flow_ml_min: Option<f64>,
    pub column_flow_ml_min: Option<f64>,
    pub column_control_mode: String,
    pub column_velocity_cm_s: Option<f64>,
    pub column_holdup_min: Option<f64>,
    /// The column description line (maker, model, and its serial number: an identity field) and the line under it.
    pub column_info: String,
    pub column_name: String,
    pub column_dimensions: String,
    pub transfer_line_c: Option<f64>,
    pub injection_volume_ul: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcqMethod {
    /// The GC part (oven, inlet, column, transfer line).
    pub gc: GcMethod,
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
        gc: parse_gc(&text),
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

/// "Key      value" lines of the GC report: the key and the value are apart by two spaces or more.
fn spaced(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_end();
    let i = t.find("  ")?;
    let (k, v) = t.split_at(i);
    (!k.trim().is_empty() && !v.trim().is_empty()).then(|| (k.trim(), v.trim()))
}

/// The first number of a value ("270 °C", "On    270 °C" → 270).
fn first_number(v: &str) -> Option<f64> {
    v.split_whitespace().find_map(|w| w.parse().ok())
}

/// Reads the GC part of the report: the lines before `MS Information`, by section.
pub fn parse_gc(text: &str) -> GcMethod {
    let mut g = GcMethod::default();
    let mut sec = String::new();
    let mut ramp = OvenRamp::default();
    let mut ramp_open = false;
    let mut after_column = 0;
    for line in text.lines().take_while(|l| !l.contains("MS Information")) {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let Some((k, v)) = spaced(line) else {
            match t {
                "Oven" | "Column" | "MSD Transfer Line" | "ALS" => sec = t.to_string(),
                _ if t.starts_with("Front SSZ Inlet") || t.starts_with("Back SSZ Inlet") => {
                    sec = "Inlet".into();
                    g.inlet = t.to_string();
                }
                _ if sec == "Column" && after_column == 1 => {
                    g.column_name = t.to_string();
                    after_column = 2;
                }
                _ => {}
            }
            continue;
        };
        match (sec.as_str(), k) {
            ("Oven", "(Initial)") => g.oven_initial_c = first_number(v),
            ("Oven", "Hold Time") => g.oven_hold_min = first_number(v),
            ("Oven", "Equilibration Time") => g.equilibration_min = first_number(v),
            ("Oven", k) if k.ends_with("Rate") => {
                if ramp_open {
                    g.oven_ramps.push(std::mem::take(&mut ramp));
                }
                ramp.rate_c_min = first_number(v).unwrap_or(0.0);
                ramp_open = true;
            }
            ("Oven", k) if k.ends_with("Value") => ramp.to_c = first_number(v).unwrap_or(0.0),
            ("Oven", k) if k.ends_with("Hold Time") => {
                ramp.hold_min = first_number(v).unwrap_or(0.0)
            }
            ("Inlet", "Mode") => g.inlet_mode = v.to_string(),
            ("Inlet", "Heater") => g.inlet_heater_c = first_number(v.trim_start_matches("On")),
            ("Inlet", "Pressure") => {
                g.inlet_pressure_psi = first_number(v.trim_start_matches("On"))
            }
            ("Inlet", "Total Flow") => {
                g.inlet_total_flow_ml_min = first_number(v.trim_start_matches("On"))
            }
            ("ALS", "Injection Volume") => g.injection_volume_ul = first_number(v),
            ("Column", "(Initial)") if g.column_flow_ml_min.is_none() => {
                g.column_flow_ml_min = first_number(v)
            }
            ("Column", "Column Information") => {
                g.column_info = v.to_string();
                after_column = 1;
            }
            ("Column", "Dimensions") => g.column_dimensions = v.to_string(),
            ("Column", "Average Velocity") => g.column_velocity_cm_s = first_number(v),
            ("Column", "Holdup Time") => g.column_holdup_min = first_number(v),
            ("Column", "Control Mode") => g.column_control_mode = v.to_string(),
            ("Column", "Pressure") | ("Column", "Flow") => {}
            ("MSD Transfer Line", "(Initial)") => g.transfer_line_c = first_number(v),
            _ => {}
        }
    }
    if ramp_open {
        g.oven_ramps.push(ramp);
    }
    g.run_time_min = text.lines().find_map(|l| {
        spaced(l)
            .filter(|(k, _)| *k == "Run Time")
            .and_then(|(_, v)| first_number(v))
    });
    g
}
