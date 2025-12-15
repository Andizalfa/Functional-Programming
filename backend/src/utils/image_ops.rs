use image::{
    imageops::FilterType, DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgba,
};

use crate::utils::error::AppError;

/* =========================================================
   CONSTANTS
   ========================================================= */

/// Rasio lebar watermark terhadap lebar gambar dasar
const WATERMARK_RATIO: f32 = 0.20;

/* =========================================================
   SINGLE-PURPOSE PURE HELPERS
   ========================================================= */

/// Clamp nilai float ke rentang 0.0..=1.0
fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Pastikan nilai minimal 1 (menghindari ukuran 0px)
fn max1(v: u32) -> u32 {
    if v == 0 { 1 } else { v }
}

/// Hitung posisi watermark di pojok kanan bawah dengan margin
fn bottom_right_position(
    base_w: u32,
    base_h: u32,
    wm_w: u32,
    wm_h: u32,
    margin: u32,
) -> (u32, u32) {
    (
        base_w.saturating_sub(wm_w.saturating_add(margin)),
        base_h.saturating_sub(wm_h.saturating_add(margin)),
    )
}

/// Blend satu channel warna
fn blend_channel(base: u8, wm: u8, alpha: f32) -> u8 {
    let b = base as f32;
    let w = wm as f32;

    ((b * (1.0 - alpha) + w * alpha).round())
        .clamp(0.0, 255.0) as u8
}

/// Blend dua pixel RGBA berdasarkan alpha watermark
fn blend_rgba(base: Rgba<u8>, wm: Rgba<u8>, opacity: f32) -> Rgba<u8> {
    let alpha = (wm[3] as f32 / 255.0) * opacity;

    Rgba([
        blend_channel(base[0], wm[0], alpha),
        blend_channel(base[1], wm[1], alpha),
        blend_channel(base[2], wm[2], alpha),
        255,
    ])
}

/// Ambil pixel watermark jika (x,y) berada di area watermark
fn watermark_pixel_at(
    watermark: &DynamicImage,
    (wx, wy): (u32, u32),
    x: u32,
    y: u32,
) -> Option<Rgba<u8>> {
    let (ww, wh) = watermark.dimensions();

    let inside = x >= wx
        && y >= wy
        && x < wx.saturating_add(ww)
        && y < wy.saturating_add(wh);

    inside.then(|| watermark.get_pixel(x - wx, y - wy).to_rgba())
}

/// Tentukan pixel output pada (x,y)
fn blend_at(
    base: &DynamicImage,
    watermark: &DynamicImage,
    pos: (u32, u32),
    opacity: f32,
    x: u32,
    y: u32,
) -> Rgba<u8> {
    let base_px = base.get_pixel(x, y).to_rgba();

    match watermark_pixel_at(watermark, pos, x, y) {
        None => base_px,
        Some(wm_px) => blend_rgba(base_px, wm_px, opacity),
    }
}

/// Resize watermark relatif terhadap lebar gambar dasar
fn resize_watermark_relative(
    watermark: &DynamicImage,
    base_width: u32,
    ratio: f32,
) -> Result<DynamicImage, AppError> {
    let (wm_w, wm_h) = watermark.dimensions();

    if wm_w == 0 || wm_h == 0 || base_width == 0 {
        return Err(AppError::BadRequest(
            "Ukuran gambar watermark atau base tidak valid".into(),
        ));
    }

    let new_w = max1((base_width as f32 * ratio).round() as u32);
    let new_h = max1(((new_w as f32) * (wm_h as f32 / wm_w as f32)).round() as u32);

    Ok(watermark.resize(new_w, new_h, FilterType::Lanczos3))
}

/* =========================================================
   ORCHESTRATOR
   ========================================================= */

/// Fungsi utama untuk memberi watermark pada gambar.
///
/// Memanggil helper:
/// 1) `clamp01` → normalisasi opacity
/// 2) `resize_watermark_relative` → resize watermark
/// 3) `bottom_right_position` → hitung posisi watermark
/// 4) `blend_at` → menghitung pixel output
pub fn apply_watermark(
    base: &DynamicImage,
    watermark: &DynamicImage,
    opacity: f32,
    margin: u32,
) -> Result<DynamicImage, AppError> {
    let opacity = clamp01(opacity);

    let (bw, bh) = base.dimensions();
    let resized = resize_watermark_relative(watermark, bw, WATERMARK_RATIO)?;

    let (ww, wh) = resized.dimensions();
    let pos = bottom_right_position(bw, bh, ww, wh, margin);

    let out = ImageBuffer::from_fn(bw, bh, |x, y| {
        blend_at(base, &resized, pos, opacity, x, y)
    });

    Ok(DynamicImage::ImageRgba8(out))
}
