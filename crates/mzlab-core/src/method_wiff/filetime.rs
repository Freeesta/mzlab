//! The FILETIME of a property set (100 ns ticks since 1601) as a local `YYYY-MM-DDThh:mm:ss`: the creation time of a `.wiff`.

pub(super) fn filetime(ft: u64) -> String {
    let secs = (ft / 10_000_000) as i64 - 11_644_473_600;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let (doy, y) = (doe - (365 * yoe + yoe / 4 - yoe / 100), yoe + era * 400);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (
        doy - (153 * mp + 2) / 5 + 1,
        if mp < 10 { mp + 3 } else { mp - 9 },
    );
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        if m <= 2 { y + 1 } else { y },
        m,
        d,
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}
