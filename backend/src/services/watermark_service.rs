use std::collections::HashMap;
use crate::utils::image_ops::process_photos_parallel;

// =====================================================================
// TYPE DEFINITIONS (IMMUTABLE DATA CONTRACT)
// =====================================================================

pub type FileBytes = (String, Vec<u8>);
pub type PayloadMap = HashMap<String, Vec<FileBytes>>;

// =====================================================================
// PUBLIC SERVICE FUNCTION
// - 100% IMMUTABLE
// - NO mut
// - NO stateful IO
// =====================================================================
pub fn process_payload(payload: PayloadMap) -> Vec<(String, Vec<u8>)> {

    let watermark = extract_watermark(&payload);
    let photos = extract_photos(&payload);

    match (photos, watermark) {
        (Some(p), Some(wm)) => process_photos_parallel(p, wm, 0.5),
        _ => Vec::new(),
    }
}

// =====================================================================
// EXTRACT WATERMARK IMAGE
// =====================================================================
fn extract_watermark(payload: &PayloadMap) -> Option<image::DynamicImage> {
    payload
        .get("watermark")
        .and_then(|v| v.first())
        .and_then(|(_, bytes)| image::load_from_memory(bytes).ok())
}

// =====================================================================
// EXTRACT PHOTOS LIST
// =====================================================================
fn extract_photos(payload: &PayloadMap) -> Option<Vec<FileBytes>> {
    payload
        .get("photos")
        .or(payload.get("photos[]"))
        .cloned()
}