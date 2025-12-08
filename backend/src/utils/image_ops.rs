use image::{DynamicImage, GenericImageView, GenericImage, ImageOutputFormat, imageops::FilterType};
use rayon::prelude::*;
use std::io::Cursor;



// =====================================================================
// 1) Hitung posisi watermark
// =====================================================================
pub fn compute_position(
    base_width: u32,
    base_height: u32,
    wm_width: u32,
    wm_height: u32,
    margin: u32,
) -> (u32, u32) {
    let x = base_width.saturating_sub(wm_width + margin);
    let y = base_height.saturating_sub(wm_height + margin);
    (x, y)
}



// =====================================================================
// 2) Watermark Overlay (alpha blending manual)
// =====================================================================
pub fn apply_watermark(
    mut base: DynamicImage,
    watermark: &DynamicImage,
    opacity: f32,
    margin: u32,
) -> DynamicImage {

    let (bw, bh) = base.dimensions();
    let (ww, wh) = watermark.dimensions();

    // Posisi dipisahkan ke function compute_position()
    let (x, y) = compute_position(bw, bh, ww, wh, margin);

    for i in 0..ww {
        for j in 0..wh {
            let mut wp = watermark.get_pixel(i, j);
            let mut bp = base.get_pixel(x + i, y + j);

            // Sesuaikan alpha watermark
            wp.0[3] = ((wp.0[3] as f32) * opacity) as u8;

            let alpha = wp.0[3] as f32 / 255.0;

            // Blending manual (RGB saja)
            bp.0[0] = (bp.0[0] as f32 * (1.0 - alpha) + wp.0[0] as f32 * alpha) as u8;
            bp.0[1] = (bp.0[1] as f32 * (1.0 - alpha) + wp.0[1] as f32 * alpha) as u8;
            bp.0[2] = (bp.0[2] as f32 * (1.0 - alpha) + wp.0[2] as f32 * alpha) as u8;

            base.put_pixel(x + i, y + j, bp);
        }
    }

    base
}



// =====================================================================
// 3) Parallel processing untuk banyak foto
// =====================================================================
pub fn process_photos_parallel(
    photos: Vec<(String, Vec<u8>)>,
    wm_img: DynamicImage,
    opacity: f32,
) -> Vec<(String, Vec<u8>)> 
{
    photos
        .into_par_iter()
        .map(|(filename, bytes)| {

            // Load gambar
            let mut img = image::load_from_memory(&bytes).unwrap();
            let (w, h) = img.dimensions();

            // Resize watermark agar proporsional
            let wm_small = wm_img.resize(w / 4, h / 4, FilterType::Lanczos3);

            // Apply watermark
            let img = apply_watermark(img, &wm_small, opacity, 20);

            // Encode hasil ke PNG
            let mut out_bytes = Vec::new();
            img.write_to(
                &mut Cursor::new(&mut out_bytes),
                ImageOutputFormat::Png,
            )
            .unwrap();

            (filename, out_bytes)
        })
        .collect()
}
