// Mengimpor PathBuf untuk merepresentasikan path file sistem
use std::path::PathBuf;
// Mengimpor Command untuk menjalankan proses eksternal
use std::process::Command;
// Mengimpor Uuid untuk menghasilkan identifier unik untuk nama file output
use uuid::Uuid;
// Mengimpor fs untuk operasi file system
use std::fs;

// Fungsi publik untuk memproses banyak gambar dengan watermark secara multiprocess
// Parameter:
// - images: vector berisi path gambar-gambar yang akan diberi watermark
// - watermark: path file watermark yang akan ditempelkan
// Mengembalikan vector berisi tuple (path file hasil, durasi pemrosesan dalam detik)
pub fn process_multiprocess(
    images: Vec<PathBuf>,      // Vector berisi path gambar-gambar input
    watermark: PathBuf,        // Path file watermark
) -> Vec<(PathBuf, f64)> {     // Mengembalikan vector tuple (path output, durasi detik)

    // Membuat folder tmp jika belum ada
    fs::create_dir_all("tmp").expect("Gagal membuat folder tmp");

    // Mengkonversi vector images menjadi iterator untuk diproses satu per satu
    images
        .into_iter()
        // Memetakan setiap gambar (img) ke operasi pemrosesan
        .map(|img| {
            // Mulai tracking waktu untuk foto ini
            let start = std::time::Instant::now();

            // Membuat path output unik menggunakan UUID random
            // Format: tmp/<uuid-random>.png
            let output = PathBuf::from(format!("tmp/{}.png", Uuid::new_v4()));

            // Membuat command baru untuk menjalankan cargo
            let status = Command::new("cargo")
                // Menambahkan argumen-argumen untuk cargo command
                .args([
                    "run",                              // Subcommand cargo untuk menjalankan binary
                    "--bin",                            // Flag untuk menentukan binary tertentu
                    "watermark_worker",                 // Nama binary worker yang akan dijalankan
                    img.to_str().unwrap(),             // Path gambar input (dikonversi ke string)
                    watermark.to_str().unwrap(),       // Path watermark (dikonversi ke string)
                    output.to_str().unwrap(),          // Path output (dikonversi ke string)
                ])
                // Wait (tunggu) proses hingga selesai dan dapatkan status
                .status()
                // Jika gagal menjalankan command, panic dengan pesan error
                .expect("Gagal menjalankan worker process");

            // Cek apakah proses berhasil
            if !status.success() {
                panic!("Worker process gagal untuk {:?}", img);
            }

            // Hitung durasi pemrosesan foto ini
            let duration = start.elapsed();
            let duration_secs = duration.as_secs_f64();

            // Mengembalikan tuple (path output, durasi) untuk setiap gambar yang diproses
            (output, duration_secs)
        })
        // Mengumpulkan semua hasil map menjadi vector tuple (PathBuf, f64)
        .collect()
}