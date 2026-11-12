//! The rows of the gradient table of a Waters `_INLET.INF`.
use super::*;

#[test]
fn a_gradient_row_gives_time_flow_and_percent_b() {
    assert_eq!(
        gradient_row("1. Initial 0.250 65.0 35.0 Initial"),
        Some((0.0, Some(0.25), Some(35.0)))
    );
    assert_eq!(
        gradient_row("2. 1.00 0.300 60.0 40.0 6"),
        Some((1.0, Some(0.3), Some(40.0)))
    );
    assert_eq!(gradient_row("Time Flow %A %B Curve"), None);
    assert_eq!(gradient_row("2. 1.00 0.300"), None);
}
