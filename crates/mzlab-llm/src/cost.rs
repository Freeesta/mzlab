//! The estimated cost of a turn from the `usage` event and the price of the model (a hint for the user, never a bill).

use crate::event::Usage;
use crate::provider::Model;

/// In the currency of the model's price; `None` when the price is unknown (0). Cache reads and writes are priced as input (a lower bound; `spend` has the exact rates).
pub fn estimate(u: &Usage, m: &Model) -> Option<f64> {
    if m.price.input <= 0.0 || m.price.output <= 0.0 {
        return None;
    }
    Some(
        ((u.input + u.cache_read + u.cache_write_5m + u.cache_write_1h) as f64 * m.price.input
            + u.output as f64 * m.price.output)
            / 1e6,
    )
}
