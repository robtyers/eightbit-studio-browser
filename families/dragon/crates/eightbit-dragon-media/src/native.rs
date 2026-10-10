//! Original native BASIC startup image, carried in `XRoar`'s segmented binary format.
use crate::{read_machine_code, read_tokenised_basic};
use eightbit_dragon_basic_core::{Dialect, detokenise};

#[derive(Debug, thiserror::Error)]
#[error("DSM2400: {0}")]
/// Invalid native run image; never includes source or firmware bytes.
pub struct NativeImageError(pub String);

/// Build a bounded RAM image from an already packaged cassette. Native BASIC
/// must have completed EXEC 48000 before the emulator loads this image.
/// The image contains only the programme and our own workspace initialiser.
pub fn create_native_run_image(
    cassette: &[u8],
    machine_code: bool,
) -> Result<Vec<u8>, NativeImageError> {
    let invalid =
        || NativeImageError("native run image exceeds the supported RAM workspace".into());
    let mut image = Vec::new();
    let entry = if machine_code {
        let code = read_machine_code(cassette).map_err(|_| invalid())?;
        if code.program.is_empty()
            || code.load_address < 0x0400
            || usize::from(code.load_address) + code.program.len() > 0xc000
            || code.execution_address < code.load_address
            || usize::from(code.execution_address)
                >= usize::from(code.load_address) + code.program.len()
        {
            return Err(invalid());
        }
        segment(&mut image, code.load_address, &code.program)?;
        code.execution_address
    } else {
        let basic = read_tokenised_basic(cassette).map_err(|_| invalid())?;
        let dialect = Dialect::for_id("dragon64-basic").map_err(|_| invalid())?;
        let listing = detokenise(&basic.program, &dialect).map_err(|_| invalid())?;
        let end = 0x1e01usize + basic.program.len();
        if !listing.warnings.is_empty() || basic.program.len() < 7 || end + 512 > 0xbffe {
            return Err(invalid());
        }
        let end = u16::try_from(end).map_err(|_| invalid())?;
        // Disable interrupts while establishing the native BASIC workspace.
        // This is the same documented workspace contract as our cartridge
        // loader, with native RAM limits and the relocated RUN entry point.
        let mut loader = vec![
            0x1a, 0x50, 0x4f, 0x1f, 0x8b, 0x10, 0xce, 0xbf, 0x36, 0x7f, 0x1e, 0x00,
        ];
        for (value, destinations) in [
            (0x1e01, &[0x19][..]),
            (end, &[0x1b, 0x1d, 0x1f][..]),
            (0xbffe, &[0x23, 0x27][..]),
            (0xbf36, &[0x21][..]),
            (0x1e00, &[0xa6][..]),
        ] {
            loader.push(0x8e);
            loader.extend(value.to_be_bytes());
            for destination in destinations {
                loader.extend([0x9f, *destination]);
            }
        }
        loader.extend([0x1c, 0xaf, 0x4f, 0x7e, 0xc4, 0x9f]);
        segment(&mut image, 0x0600, &loader)?;
        segment(&mut image, 0x1e01, &basic.program)?;
        0x0600
    };
    image.extend([0xff, 0, 0]);
    image.extend(entry.to_be_bytes());
    Ok(image)
}
fn segment(image: &mut Vec<u8>, address: u16, bytes: &[u8]) -> Result<(), NativeImageError> {
    let length = u16::try_from(bytes.len())
        .map_err(|_| NativeImageError("native image segment is too large".into()))?;
    image.push(0);
    image.extend(length.to_be_bytes());
    image.extend(address.to_be_bytes());
    image.extend(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_machine_code, create_tokenised_basic};
    use eightbit_dragon_basic_core::{analyse, tokenise};
    #[test]
    fn native_image_is_deterministic_original_data_with_checked_ram_bounds() {
        let dialect = Dialect::for_id("dragon64-basic").unwrap();
        let tokens = tokenise(&analyse("10 A=42\n20 END\n", &dialect).program, &dialect)
            .unwrap()
            .bytes;
        let cas = create_tokenised_basic("NATIVE", &tokens).unwrap();
        let image = create_native_run_image(&cas, false).unwrap();
        assert_eq!(image, create_native_run_image(&cas, false).unwrap());
        assert_eq!(&image[image.len() - 5..], &[0xff, 0, 0, 6, 0]);
        assert!(image.windows(tokens.len()).any(|bytes| bytes == tokens));
        for load in [0x0400, 0x9000, 0xbfff] {
            let cas = create_machine_code("CODE", &[0x39], load, load).unwrap();
            assert!(create_native_run_image(&cas, true).is_ok());
        }
        for (load, bytes, entry) in [
            (0x3ff, vec![0x39], 0x3ff),
            (0xbfff, vec![0x12, 0x39], 0xbfff),
            (0x9000, vec![0x39], 0x9001),
        ] {
            let cas = create_machine_code("CODE", &bytes, load, entry).unwrap();
            assert!(create_native_run_image(&cas, true).is_err());
        }
        for bytes in [&[][..], &[0x55][..], &cas[..cas.len() / 2]] {
            assert!(create_native_run_image(bytes, false).is_err());
        }
    }
}
