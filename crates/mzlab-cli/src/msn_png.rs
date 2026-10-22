//! The PNG of a figure of the graph: the SVG of `msn_figura` rasterised by `resvg` (pure Rust), only when `mzlab-cli` is built with the cargo feature `png`
//! (not in the default build, never in a .wasm). Without it the answer is an error whose key says how to get it.

use mzlab_core::error::{Error, Result};

/// Pixels per unit of the SVG: the figure is drawn at twice its size so that the small text stays readable.
#[cfg(feature = "png")]
const SCALE: f32 = 2.0;

/// The PNG bytes of the SVG text.
#[cfg(feature = "png")]
pub fn png(svg: &str) -> Result<Vec<u8>> {
    let bad = |d: String| Error::new("err.msnfigura.png.render").with("detail", d);
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    opt.font_family = "sans-serif".into();
    let tree = resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| bad(e.to_string()))?;
    let size = tree
        .size()
        .to_int_size()
        .scale_by(SCALE)
        .ok_or_else(|| bad("size".into()))?;
    let mut pix = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| bad("size".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(SCALE, SCALE),
        &mut pix.as_mut(),
    );
    pix.encode_png().map_err(|e| bad(e.to_string()))
}

/// Without the feature: the key and how to build it.
#[cfg(not(feature = "png"))]
pub fn png(_svg: &str) -> Result<Vec<u8>> {
    Err(Error::new("err.msnfigura.png").with("build", "cargo build -p mzlab-cli --features png"))
}
