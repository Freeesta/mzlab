//! Serialization of the 2D optical DAD / PDA absorbance matrix into the mzLab map format.

use base64::Engine;

use super::dad::DadData;
use crate::json::Value;
use crate::jsonb::{text, Obj};

/// Encodes the 2D absorbance matrix of `dad` as JSON for `/api/map?kind=pda`.
pub fn dad_to_map_json(dad: &DadData) -> String {
    let (rt0, rt1) = (
        dad.time_min.first().copied().unwrap_or(0.0),
        dad.time_min.last().copied().unwrap_or(dad.duration_min),
    );
    let nrt = dad.points();
    let nmz = dad.channels();
    let b64_data = base64::engine::general_purpose::STANDARD.encode(
        dad.matrix_mau
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>(),
    );
    let o = Obj::new()
        .n("rt0", rt0)
        .n("rt1", rt1)
        .i("nrt", nrt as i64)
        .n("mz0", dad.wl_start_nm)
        .n("mz1", dad.wl_stop_nm)
        .i("nmz", nmz as i64)
        .n("dmz", dad.wl_step_nm)
        .i("level", 1)
        .b("region", false)
        .put("agg", text("mean"))
        .b("log", false)
        .b("bg", false)
        .b("hr", false)
        .put("kind", text("pda"))
        .put("data", Value::Str(b64_data));
    crate::jsonb::write(&o.done())
}
