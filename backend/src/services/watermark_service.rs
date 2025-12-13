use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

pub fn process_multiprocess(
    images: Vec<PathBuf>,
    watermark: PathBuf,
) -> Vec<PathBuf> {

    images
        .into_iter()
        .map(|img| {
            let output = PathBuf::from(format!("tmp/{}.png", Uuid::new_v4()));

            Command::new("cargo")
                .args([
                    "run",
                    "--bin",
                    "watermark_worker",
                    img.to_str().unwrap(),
                    watermark.to_str().unwrap(),
                    output.to_str().unwrap(),
                ])
                .spawn()
                .expect("Gagal spawn worker process");

            output
        })
        .collect()
}