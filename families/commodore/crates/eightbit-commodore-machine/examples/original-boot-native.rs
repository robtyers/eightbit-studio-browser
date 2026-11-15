//! Explicit source-bound original image, never downloaded or product-selected.
#[path = "support/boot.rs"]
mod boot;
#[path = "support/reference.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (bytes, expected) = boot::read()?;
    let output =
        std::env::var_os("EIGHTBIT_C64_ORIGINAL_CAPTURE_DIR").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        if !path.is_absolute() {
            return Err("Capture folder must be absolute and new".into());
        }
        std::fs::create_dir(path)?;
    }
    println!(
        "{}",
        boot::qualify(&bytes, &expected, |pixels| {
            use std::io::Write;
            if let Some(path) = &output {
                let palette = [
                    [0, 0, 0],
                    [255, 255, 255],
                    [165, 50, 50],
                    [96, 200, 190],
                    [160, 72, 180],
                    [75, 162, 65],
                    [60, 55, 160],
                    [218, 211, 103],
                    [170, 100, 40],
                    [100, 60, 20],
                    [230, 130, 130],
                    [70, 70, 70],
                    [120, 120, 120],
                    [140, 220, 130],
                    [150, 180, 255],
                    [190, 190, 190],
                ];
                let mut file = std::fs::File::create_new(path.join("original-foundation.ppm"))?;
                file.write_all(b"P6\n384 272\n255\n")?;
                for index in pixels {
                    file.write_all(
                        palette
                            .get(usize::from(*index))
                            .ok_or("Invalid original palette index")?,
                    )?;
                }
            }
            Ok(())
        })?
    );
    Ok(())
}
