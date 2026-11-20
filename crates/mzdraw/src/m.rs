//! Maths that must give the same bits in the `.wasm`, the app and the CLI: the trigonometry comes from the `libm` crate on every
//! target (the platform's own differs in the last digit), the rest is exact IEEE arithmetic.

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub fn hypot(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
