//! Opt-in supplied-ROM qualification. Only original programme observations are exported.
#[path = "support/reference.rs"]
mod reference;
use std::{io::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let packed = reference::read()?;
    let firmware = reference::validate(&packed)?;
    let output = std::env::var_os("EIGHTBIT_C64_CAPTURE_DIR").map(PathBuf::from);
    if let Some(path) = &output {
        if !path.is_absolute() {
            return Err("Capture folder must be absolute and new".into());
        }
        std::fs::create_dir(path)?;
    }
    let report = reference::qualify(firmware, |name, pixels, audio| {
        if let Some(path) = &output {
            // Original illustrative palette; no copied emulator palette/calibration claim.
            let palette = [
                [0, 0, 0],
                [255, 255, 255],
                [165, 50, 50],
                [96, 200, 190],
                [160, 72, 180],
                [75, 162, 65],
                [60, 55, 160],
                [218, 211, 103],
                [174, 106, 45],
                [104, 69, 26],
                [215, 111, 111],
                [65, 65, 65],
                [120, 120, 120],
                [143, 215, 128],
                [137, 131, 226],
                [184, 184, 184],
            ];
            let mut image = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path.join(format!("{name}.ppm")))?;
            write!(image, "P6\n384 272\n255\n")?;
            for pixel in pixels {
                image.write_all(&palette[usize::from(*pixel & 15)])?;
            }
            if !audio.is_empty() {
                let bytes = u32::try_from(audio.len() * 2)?;
                let mut wav = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path.join(format!("{name}.wav")))?;
                wav.write_all(b"RIFF")?;
                wav.write_all(&(36 + bytes).to_le_bytes())?;
                wav.write_all(b"WAVEfmt ")?;
                wav.write_all(&16u32.to_le_bytes())?;
                wav.write_all(&1u16.to_le_bytes())?;
                wav.write_all(&1u16.to_le_bytes())?;
                wav.write_all(&44100u32.to_le_bytes())?;
                wav.write_all(&88200u32.to_le_bytes())?;
                wav.write_all(&2u16.to_le_bytes())?;
                wav.write_all(&16u16.to_le_bytes())?;
                wav.write_all(b"data")?;
                wav.write_all(&bytes.to_le_bytes())?;
                for sample in audio {
                    wav.write_all(&sample.to_le_bytes())?;
                }
            }
        }
        Ok(())
    })?;
    println!("{report}");
    Ok(())
}
