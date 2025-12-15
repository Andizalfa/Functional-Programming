// Mengimpor DynamicImage untuk merepresentasikan gambar dengan berbagai format
// GenericImageView untuk mendapatkan dimensi dan pixel dari gambar
// ImageBuffer untuk membuat buffer gambar baru
// Rgba untuk merepresentasikan pixel dengan 4 channel (Red, Green, Blue, Alpha)
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, imageops::FilterType};

// =====================================================================
// PUBLIC PURE API
// =====================================================================
// - Pure function
// - Deterministic
// - Aman dipanggil di worker process mana pun
// =====================================================================

// Fungsi publik untuk menerapkan watermark ke gambar dasar
// Parameter:
// - base: referensi ke gambar dasar
// - watermark: referensi ke gambar watermark
// - opacity: tingkat transparansi watermark (0.0-1.0)
// - margin: jarak dari tepi gambar dalam pixel
// Mengembalikan gambar baru dengan watermark yang sudah diterapkan
pub fn apply_watermark(
    base: &DynamicImage,           // Referensi ke gambar dasar yang akan diberi watermark
    watermark: &DynamicImage,      // Referensi ke gambar watermark yang akan ditempelkan
    opacity: f32,                  // Tingkat opacity watermark (0.0 = transparan, 1.0 = solid)
    margin: u32,                   // Margin dari tepi kanan bawah dalam pixel
) -> DynamicImage {                // Mengembalikan gambar baru bertipe DynamicImage
    // Mendapatkan lebar (bw) dan tinggi (bh) dari gambar dasar
    let (bw, bh) = base.dimensions();
    
    // Resize watermark agar proporsional dengan gambar dasar
    // Watermark akan berukuran 20% dari lebar gambar dasar
    let watermark_resized = resize_watermark(watermark, bw);
    
    // Mendapatkan lebar (ww) dan tinggi (wh) dari gambar watermark yang sudah diresize
    let (ww, wh) = watermark_resized.dimensions();
    
    // Menghitung posisi watermark di pojok kanan bawah dengan mempertimbangkan margin
    let position = compute_position(bw, bh, ww, wh, margin);

    // Membuat ImageBuffer baru dengan lebar bw dan tinggi bh
    // Setiap pixel dihasilkan dari fungsi closure yang menerima koordinat (x, y)
    let output = ImageBuffer::from_fn(bw, bh, |x, y| {
        // Untuk setiap pixel pada posisi (x, y), panggil fungsi blend_pixel
        // untuk mencampur pixel gambar dasar dengan watermark
        blend_pixel(base, &watermark_resized, position, opacity, x, y)
    });

    // Mengkonversi ImageBuffer menjadi DynamicImage dengan format RGBA8
    DynamicImage::ImageRgba8(output)
}

//
// =====================================================================
// HELPER PURE FUNCTIONS (PRIVATE)
// =====================================================================
//

// -----------------------------------------------------
// Resize watermark agar proporsional dengan gambar base
// -----------------------------------------------------
// Fungsi untuk meresize watermark agar ukurannya proporsional
// Watermark akan berukuran 20% dari lebar gambar dasar
fn resize_watermark(watermark: &DynamicImage, base_width: u32) -> DynamicImage {
    let (wm_width, wm_height) = watermark.dimensions();
    
    // Hitung lebar baru watermark (20% dari lebar base)
    let new_width = (base_width as f32 * 0.20) as u32;
    
    // Hitung tinggi baru dengan mempertahankan aspect ratio
    let aspect_ratio = wm_height as f32 / wm_width as f32;
    let new_height = (new_width as f32 * aspect_ratio) as u32;
    
    // Resize watermark menggunakan filter Lanczos3 untuk kualitas terbaik
    watermark.resize(new_width, new_height, FilterType::Lanczos3)
}

// -----------------------------------------------------
// Hitung posisi watermark (pojok kanan bawah)
// -----------------------------------------------------
// Fungsi untuk menghitung posisi x dan y dari watermark
// agar berada di pojok kanan bawah dengan margin yang ditentukan
fn compute_position(
    base_width: u32,       // Lebar gambar dasar
    base_height: u32,      // Tinggi gambar dasar
    wm_width: u32,         // Lebar watermark
    wm_height: u32,        // Tinggi watermark
    margin: u32,           // Margin dari tepi
) -> (u32, u32) {          // Mengembalikan tuple (x, y) posisi watermark
    (
        // Menghitung posisi X: lebar gambar - (lebar watermark + margin)
        // saturating_sub mencegah underflow (hasil tidak akan negatif)
        base_width.saturating_sub(wm_width + margin),
        // Menghitung posisi Y: tinggi gambar - (tinggi watermark + margin)
        // saturating_sub mencegah underflow (hasil tidak akan negatif)
        base_height.saturating_sub(wm_height + margin),
    )
}

// -----------------------------------------------------
// Blend satu pixel (PURE, no side effect)
// -----------------------------------------------------
// Fungsi untuk mencampur (blend) satu pixel antara gambar dasar dan watermark
fn blend_pixel(
    base: &DynamicImage,           // Referensi ke gambar dasar
    watermark: &DynamicImage,      // Referensi ke gambar watermark
    (wx, wy): (u32, u32),         // Posisi watermark (koordinat kiri atas)
    opacity: f32,                  // Tingkat opacity watermark
    x: u32,                        // Koordinat X pixel yang sedang diproses
    y: u32,                        // Koordinat Y pixel yang sedang diproses
) -> Rgba<u8> {                    // Mengembalikan pixel RGBA yang sudah di-blend

    // Mendapatkan pixel dari gambar dasar pada posisi (x, y)
    let base_px = base.get_pixel(x, y);
    // Mendapatkan dimensi watermark (lebar dan tinggi)
    let (ww, wh) = watermark.dimensions();

    // Mengecek apakah pixel (x, y) berada di dalam area watermark
    // dengan memeriksa apakah x berada di antara wx dan wx+ww
    // dan y berada di antara wy dan wy+wh
    let is_inside_watermark =
        x >= wx && x < wx + ww &&  // Cek apakah x berada di dalam range horizontal watermark
        y >= wy && y < wy + wh;    // Cek apakah y berada di dalam range vertikal watermark

    // Jika pixel tidak berada di dalam area watermark
    if !is_inside_watermark {
        // Kembalikan pixel dari gambar dasar tanpa perubahan
        return base_px;
    }

    // Mendapatkan pixel watermark pada posisi relatif (x-wx, y-wy)
    let wm_px = watermark.get_pixel(x - wx, y - wy);
    // Menghitung nilai alpha final dengan mengalikan alpha channel watermark dengan opacity
    // wm_px.0[3] adalah alpha channel (indeks 3), dibagi 255.0 untuk normalisasi ke 0.0-1.0
    let alpha = (wm_px.0[3] as f32 * opacity) / 255.0;

    // Membuat pixel RGBA baru dengan mencampur channel warna
    Rgba([
        // Blend channel merah (indeks 0)
        blend_channel(base_px.0[0], wm_px.0[0], alpha),
        // Blend channel hijau (indeks 1)
        blend_channel(base_px.0[1], wm_px.0[1], alpha),
        // Blend channel biru (indeks 2)
        blend_channel(base_px.0[2], wm_px.0[2], alpha),
        // Alpha channel selalu 255 (fully opaque)
        255,
    ])
}

// -----------------------------------------------------
// Blend satu channel warna (PURE matematis)
// -----------------------------------------------------
// Fungsi untuk mencampur satu channel warna menggunakan alpha blending
fn blend_channel(base: u8, wm: u8, alpha: f32) -> u8 {
    // Formula alpha blending: result = base * (1 - alpha) + watermark * alpha
    // base: nilai channel dari gambar dasar (0-255)
    // wm: nilai channel dari watermark (0-255)
    // alpha: tingkat transparansi (0.0-1.0)
    // Hasilnya dikonversi kembali ke u8 (0-255)
    (base as f32 * (1.0 - alpha) + wm as f32 * alpha) as u8
}