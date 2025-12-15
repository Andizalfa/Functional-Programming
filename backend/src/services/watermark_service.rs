use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;
use std::fs;

pub fn process_multiprocess(
    images: Vec<PathBuf>,
    watermark: PathBuf,
) -> Vec<PathBuf> {

    fs::create_dir_all("tmp").expect("Gagal membuat folder tmp");

    images
        .into_iter()
        .map(|img| {
            let output = PathBuf::from(format!("tmp/{}.png", Uuid::new_v4()));

            let status = Command::new("cargo")
                .args([
                    "run",
                    "--bin",
                    "watermark_worker",
                    img.to_str().unwrap(),
                    watermark.to_str().unwrap(),
                    output.to_str().unwrap(),
                ])
                .status()
                .expect("Gagal menjalankan worker process");

            if !status.success() {
                panic!("Worker process gagal untuk {:?}", img);
            }

            output
        })
        .collect()
}