//! Pressure channels of the vendor readers (Thermo analog traces, SCIEX device traces): which channel is a pressure and how many pascal one unit of it is,
//! so that both write it in pascal as a `pressure array` (MS:1000821, as msconvert does).

/// A channel is a pressure when its name says so or its unit is a pressure unit (compared in lower case).
pub(crate) fn is_pressure(name: &str, unit: &str) -> bool {
    name.as_bytes()
        .windows(8)
        .any(|w| w.eq_ignore_ascii_case(b"pressure"))
        || pascal_per(unit).is_some()
}

/// Pascal in one unit of a pressure channel, by the name the file gives (compared in lower case); `None` for any other unit.
pub(crate) fn pascal_per(unit: &str) -> Option<f64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "psi" => Some(6_894.757_293_168),
        "bar" => Some(1e5),
        "mbar" => Some(100.0),
        "pa" | "pascal" => Some(1.0),
        "kpa" => Some(1e3),
        "mpa" => Some(1e6),
        _ => None,
    }
}
