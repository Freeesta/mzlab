//! The estimated cost of a turn from the `usage` event and the price of the model (a hint for the user, never a bill).

use crate::event::Usage;
use crate::provider::Model;

/// In the currency of the model's price; `None` when the price is unknown (0). Cached input is priced as input.
pub fn estimate(u: &Usage, m: &Model) -> Option<f64> {
    if m.price.input <= 0.0 || m.price.output <= 0.0 {
        return None;
    }
    Some(((u.input + u.cache_read) as f64 * m.price.input + u.output as f64 * m.price.output) / 1e6)
}
