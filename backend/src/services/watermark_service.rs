use rayon::prelude::*;
use std::{path::PathBuf, process::Command};
use uuid::Uuid;

use crate::utils::error::AppError;

/* =========================================================
   SINGLE-PURPOSE HELPERS (fungsi tunggal)
   ========================================================= */

/// Membuat path output unik untuk hasil watermark.
fn output_path() -> PathBuf {
    PathBuf::from(format!("tmp/{}.png", Uuid::new_v4()))
}

/// Menyusun argumen CLI untuk menjalankan worker.
/// Mengembalikan iterator agar fleksibel & tidak terikat Vec.
fn build_worker_args(
    img: PathBuf,
    watermark: PathBuf,
    out: PathBuf,
) -> impl IntoIterator<Item = String> {
    [
        "run".to_string(),
        "--bin".to_string(),
        "watermark_worker".to_string(),
        img.to_string_lossy().to_string(),
        watermark.to_string_lossy().to_string(),
        out.to_string_lossy().to_string(),
    ]
}

/// Menjalankan OS process worker (effectful).
/// Dipanggil oleh `process_multiprocess`.
fn run_worker<I>(args: I) -> Result<(), AppError>
where
    I: IntoIterator<Item = String>,
{
    Command::new("cargo")
        .args(args)
        .status()
        .map_err(|e| AppError::Internal(format!("Gagal menjalankan worker process: {e}")))?
        .success()
        .then_some(())
        .ok_or_else(|| AppError::Internal("Worker process gagal".to_string()))
}

/* =========================================================
   ORCHESTRATOR (memanggil helper di atas)
   ========================================================= */

/// Memproses watermark untuk banyak gambar secara:
/// - PARALLEL → menggunakan rayon thread pool
/// - MULTIPROCESS → setiap gambar dijalankan di OS process terpisah
///
/// Alur per gambar:
/// 1) `output_path()` → buat path output unik
/// 2) `build_worker_args(...)` → susun argumen CLI
/// 3) `run_worker(args)` → jalankan worker process
pub fn process_multiprocess(
    images: Vec<PathBuf>,
    watermark: PathBuf,
) -> Result<Vec<PathBuf>, AppError> {
    images
        .into_par_iter() // <- ownership berpindah, aman & idiomatik
        .map(|img| {
            let out = output_path();
            let args = build_worker_args(img, watermark.clone(), out.clone());

            run_worker(args).map(|_| out)
        })
        .collect()
}
