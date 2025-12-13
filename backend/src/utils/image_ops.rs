use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

// =====================================================================
// PUBLIC PURE API
// =====================================================================
// - Pure function
// - Deterministic
// - Aman dipanggil di worker process mana pun
// =====================================================================
pub fn apply_watermark(
    base: &DynamicImage,
    watermark: &DynamicImage,
    opacity: f32,
    margin: u32,
) -> DynamicImage {
    let (bw, bh) = base.dimensions();
    let (ww, wh) = watermark.dimensions();
    let position = compute_position(bw, bh, ww, wh, margin);

    let output = ImageBuffer::from_fn(bw, bh, |x, y| {
        blend_pixel(base, watermark, position, opacity, x, y)
    });

    DynamicImage::ImageRgba8(output)
}

//
// =====================================================================
// HELPER PURE FUNCTIONS (PRIVATE)
// =====================================================================
//

// -----------------------------------------------------
// Hitung posisi watermark (pojok kanan bawah)
// -----------------------------------------------------
fn compute_position(
    base_width: u32,
    base_height: u32,
    wm_width: u32,
    wm_height: u32,
    margin: u32,
) -> (u32, u32) {
    (
        base_width.saturating_sub(wm_width + margin),
        base_height.saturating_sub(wm_height + margin),
    )
}

// -----------------------------------------------------
// Blend satu pixel (PURE, no side effect)
// -----------------------------------------------------
fn blend_pixel(
    base: &DynamicImage,
    watermark: &DynamicImage,
    (wx, wy): (u32, u32),
    opacity: f32,
    x: u32,
    y: u32,
) -> Rgba<u8> {

    let base_px = base.get_pixel(x, y);
    let (ww, wh) = watermark.dimensions();

    let is_inside_watermark =
        x >= wx && x < wx + ww &&
        y >= wy && y < wy + wh;

    if !is_inside_watermark {
        return base_px;
    }

    let wm_px = watermark.get_pixel(x - wx, y - wy);
    let alpha = (wm_px.0[3] as f32 * opacity) / 255.0;

    Rgba([
        blend_channel(base_px.0[0], wm_px.0[0], alpha),
        blend_channel(base_px.0[1], wm_px.0[1], alpha),
        blend_channel(base_px.0[2], wm_px.0[2], alpha),
        255,
    ])
}

// -----------------------------------------------------
// Blend satu channel warna (PURE matematis)
// -----------------------------------------------------
fn blend_channel(base: u8, wm: u8, alpha: f32) -> u8 {
    (base as f32 * (1.0 - alpha) + wm as f32 * alpha) as u8
}