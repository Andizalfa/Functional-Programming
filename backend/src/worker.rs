use std::{env, path::Path};

use backend::utils::error::AppError;
use backend::utils::image_ops::apply_watermark;

/* =========================================================
   SINGLE-PURPOSE HELPERS (fungsi tunggal)
   ========================================================= */

/// Parse argumen CLI:
/// watermark_worker <input_path> <watermark_path> <output_path>
fn parse_args(args: Vec<String>) -> Result<(String, String, String), AppError> {
    match args.as_slice() {
        [_bin, input, watermark, output] => Ok((input.clone(), watermark.clone(), output.clone())),
        _ => Err(AppError::BadRequest(
            "Usage: watermark_worker <input_path> <watermark_path> <output_path>".into(),
        )),
    }
}

/// Membaca file gambar dari path (input / watermark)
fn load_image(path: &str, label: &str) -> Result<image::DynamicImage, AppError> {
    image::open(path).map_err(|e| {
        AppError::BadRequest(format!("Gagal membuka {label} '{path}': {e}"))
    })
}

/// Pastikan folder output tersedia sebelum menyimpan file
fn ensure_parent_dir(path: &str) -> Result<(), AppError> {
    let p = Path::new(path);

    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::Internal(format!("Gagal membuat folder output '{:?}': {e}", parent))
        })?;
    }

    Ok(())
}

/// Simpan image ke output path
fn save_image(img: &image::DynamicImage, path: &str) -> Result<(), AppError> {
    img.save(path).map_err(|e| {
        AppError::Internal(format!("Gagal menyimpan output image '{path}': {e}"))
    })
}

/* =========================================================
   ORCHESTRATOR (fungsi yang memanggil helper di atas)
   ========================================================= */

/// Menjalankan workflow worker watermark.
///
/// Memanggil:
/// 1) `parse_args(...)` → validasi & ambil input_path, watermark_path, output_path
/// 2) `load_image(...)` → load base image dan watermark image
/// 3) `apply_watermark(...)` → proses watermark (pure-ish, return Result)
/// 4) `ensure_parent_dir(...)` → buat folder output jika belum ada
/// 5) `save_image(...)` → simpan hasil ke output_path
fn run() -> Result<(), AppError> {
    let (input_path, watermark_path, output_path) = parse_args(env::args().collect())?;

    let base = load_image(&input_path, "input image")?;
    let watermark = load_image(&watermark_path, "watermark image")?;

    let result = apply_watermark(&base, &watermark, 0.5, 20)?;

    ensure_parent_dir(&output_path)?;
    save_image(&result, &output_path)?;

    Ok(())
}

/* =========================================================
   ENTRYPOINT
   ========================================================= */

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
