//! EPD1 frame encoding for the Stencild panel (800×600 landscape, 4bpp).

pub const MAGIC: &[u8; 4] = b"EPD1";
pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
pub const PAYLOAD: usize = (WIDTH as usize * HEIGHT as usize) / 2;

/// Pack an 8-bit grayscale image (WIDTH×HEIGHT, row-major) into the panel's
/// 4bpp format: even-x pixel in the low nibble, odd-x in the high nibble.
#[allow(dead_code)] // part of the public encoding API (used by tests and apps)
pub fn pack_4bpp(gray: &[u8]) -> Vec<u8> {
    assert_eq!(gray.len(), (WIDTH * HEIGHT) as usize, "grayscale size");
    let mut out = vec![0u8; PAYLOAD];
    for (i, px) in gray.iter().enumerate() {
        let v = px >> 4;
        if i & 1 == 0 {
            out[i >> 1] |= v;
        } else {
            out[i >> 1] |= v << 4;
        }
    }
    out
}

/// Build a full EPD1 frame: magic, LE payload size, waveform, invert flag, payload.
pub fn encode(payload: &[u8], waveform: u8, invert: u8) -> Vec<u8> {
    let mut frame = Vec::with_capacity(10 + payload.len());
    frame.extend_from_slice(MAGIC);
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.push(waveform);
    frame.push(invert);
    frame.extend_from_slice(payload);
    frame
}

/// Convenience: grayscale image -> packed payload -> EPD1 frame.
#[allow(dead_code)] // see pack_4bpp
pub fn encode_gray(gray: &[u8], waveform: u8, invert: bool) -> Vec<u8> {
    let payload = pack_4bpp(gray);
    encode(&payload, waveform, invert as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_header_is_epd1_and_declares_payload() {
        let payload = vec![0x55u8; PAYLOAD];
        let frame = encode(&payload, 2, 0);
        assert_eq!(&frame[..4], MAGIC);
        assert_eq!(u32::from_le_bytes(frame[4..8].try_into().unwrap()), PAYLOAD as u32);
        assert_eq!(frame[8], 2);
        assert_eq!(frame[9], 0);
        assert_eq!(frame.len(), 10 + PAYLOAD);
    }

    #[test]
    fn packing_puts_even_pixel_in_low_nibble() {
        let mut gray = vec![0u8; (WIDTH * HEIGHT) as usize];
        gray[0] = 0xF0; // pixel 0 -> low nibble F
        gray[1] = 0x00; // pixel 1 -> high nibble 0
        gray[2] = 0x00; // pixel 2 -> low nibble 0
        gray[3] = 0x30; // pixel 3 -> high nibble 3
        let packed = pack_4bpp(&gray);
        assert_eq!(packed[0], 0x0F);
        assert_eq!(packed[1], 0x30);
    }

    #[test]
    fn invert_flag_roundtrips() {
        let frame = encode_gray(&vec![0u8; (WIDTH * HEIGHT) as usize], 1, true);
        assert_eq!(frame[8], 1);
        assert_eq!(frame[9], 1);
    }
}
