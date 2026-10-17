//! Serialization of the 2D optical DAD / PDA absorbance matrix into the mzLab map format.

use super::dad::DadData;

/// Encodes the 2D absorbance matrix of `dad` as JSON for `/api/map?kind=pda`.
pub fn dad_to_map_json(dad: &DadData) -> String {
    let (rt0, rt1) = (
        dad.time_min.first().copied().unwrap_or(0.0),
        dad.time_min.last().copied().unwrap_or(dad.duration_min),
    );
    let nrt = dad.points();
    let nmz = dad.channels();
    let mut bytes = Vec::with_capacity(dad.matrix_mau.len().saturating_mul(4));
    for &val in &dad.matrix_mau {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    use base64::Engine;
    let b64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    format!(
        "{{\"data\":\"{b64_data}\",\"rt0\":{rt0},\"rt1\":{rt1},\"nrt\":{nrt},\"mz0\":{},\"mz1\":{},\"nmz\":{nmz},\"dmz\":{},\"level\":1,\"region\":false,\"agg\":\"mean\",\"log\":false,\"bg\":false,\"hr\":false,\"kind\":\"pda\"}}",
        dad.wl_start_nm, dad.wl_stop_nm, dad.wl_step_nm
    )
}
