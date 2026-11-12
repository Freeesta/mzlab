//! exp/ln/pow/trig with the same bits on every target. A native build calls the `libm` crate (the system library differs in the last bit);
//! the `.wasm` keeps std, whose wasm32 maths is the same libm code (2·10⁶ random inputs per function give identical bits), so the module
//! does not carry a second copy next to the one `sci-form` and `nalgebra` already pull in (cap of the chem module, docs/agenti/parita.md).

macro_rules! same_bits {
    ($($name:ident => $std:ident ($($a:ident),+);)+) => {$(
        #[cfg(target_arch = "wasm32")]
        #[inline]
        pub(crate) fn $name($($a: f64),+) -> f64 {
            same_bits!(@std $std $($a),+)
        }
        #[cfg(not(target_arch = "wasm32"))]
        #[inline]
        pub(crate) fn $name($($a: f64),+) -> f64 {
            libm::$name($($a),+)
        }
    )+};
    (@std $std:ident $x:ident) => { $x.$std() };
    (@std $std:ident $x:ident, $y:ident) => { $x.$std($y) };
}

same_bits! {
    exp => exp(x);
    log => ln(x);
    pow => powf(x, y);
    sin => sin(x);
    cos => cos(x);
    tanh => tanh(x);
    cbrt => cbrt(x);
}
