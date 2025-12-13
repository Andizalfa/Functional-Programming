use std::{env, fs};
use image::DynamicImage;
use crate::utils::image_ops::apply_watermark;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input_path = &args[1];
    let watermark_path = &args[2];
    let output_path = &args[3];

    let base = image::open(input_path).unwrap();
    let watermark = image::open(watermark_path).unwrap();

    let result = apply_watermark(&base, &watermark, 0.5, 20);

    result.save(output_path).unwrap();
}