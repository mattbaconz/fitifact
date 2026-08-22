use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use fitifact::image::{sample_jpeg_rgb, sample_png_rgb};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

fn main() {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image");
    fs::create_dir_all(&output).expect("create fixture directory");

    write(&output, "compatible-jpeg.jpg", sample_jpeg_rgb(8, 8));
    write(&output, "mismatch-png.png", sample_png_rgb(8, 8));

    let transparent = RgbaImage::from_fn(64, 64, |x, y| {
        let alpha = ((x + y) * 2).min(255) as u8;
        Rgba([32, 112, 78, alpha])
    });
    write_png(
        &output,
        "transparent-png.png",
        DynamicImage::ImageRgba8(transparent),
    );

    let crop_grid = RgbImage::from_fn(640, 360, |x, y| {
        if x % 80 < 3 || y % 60 < 3 {
            Rgb([24, 31, 27])
        } else if x < 160 {
            Rgb([204, 91, 69])
        } else if x >= 480 {
            Rgb([218, 166, 64])
        } else {
            Rgb([58, 132, 92])
        }
    });
    write_png(&output, "crop-grid.png", DynamicImage::ImageRgb8(crop_grid));

    let oversized = RgbImage::from_pixel(6_001, 4_000, Rgb([40, 96, 70]));
    write_png(
        &output,
        "oversized-pixels.png",
        DynamicImage::ImageRgb8(oversized),
    );

    write(
        &output,
        "malformed-image.jpg",
        b"\xff\xd8\xff\xe0\0\x10JFIF\0fitifact-truncated".to_vec(),
    );

    let mut webp = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::from_pixel(8, 8, Rgb([200, 40, 40])))
        .write_to(&mut webp, ImageFormat::WebP)
        .expect("encode WebP fixture");
    write(&output, "still-webp.webp", webp.into_inner());
}

fn write_png(output: &std::path::Path, name: &str, image: DynamicImage) {
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("encode PNG fixture");
    write(output, name, bytes.into_inner());
}

fn write(output: &std::path::Path, name: &str, bytes: Vec<u8>) {
    fs::write(output.join(name), bytes).expect("write fixture");
}
