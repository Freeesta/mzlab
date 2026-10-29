//! `Windowed::read_at` with an offset near the end of `u64` is an error, not an overflow.

use mzlab_core::idxread::ReadAt;
use mzlab_core::window::Windowed;

#[test]
fn an_offset_near_the_end_of_u64_is_an_error_not_a_wrap() {
    let mut src = vec![0u8; 100];
    let mut w = Windowed::new(&mut src, 100);
    assert!(w.read_at(u64::MAX - 2, &mut [0u8; 10]).is_err());
}
