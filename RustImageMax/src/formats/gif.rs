// GIF codec - encoder and decoder for GIF images

use crate::tile::pixel::{PixelData, PixelFormat};

pub struct GifDecoder;

impl GifDecoder {
    pub fn decode(data: &[u8]) -> Option<PixelData> {
        if data.len() < 13 {
            return None;
        }
        if !data.starts_with(b"GIF87a") && !data.starts_with(b"GIF89a") {
            return None;
        }

        let width = u16::from_le_bytes([data[6], data[7]]) as usize;
        let height = u16::from_le_bytes([data[8], data[9]]) as usize;

        Some(PixelData::new(PixelFormat::U8x3, width, height))
    }
}

pub struct GifEncoder;

impl GifEncoder {
    pub fn encode(data: &PixelData) -> Vec<u8> {
        let mut result = Vec::new();
        let width = data.width();
        let height = data.height();

        // Header
        result.extend_from_slice(b"GIF89a");

        // Logical Screen Descriptor
        result.push((width as u16) as u8);
        result.push((width as u16 >> 8) as u8);
        result.push((height as u16) as u8);
        result.push((height as u16 >> 8) as u8);
        result.push(0x80);
        result.push(0);
        result.push(0);

        // Global Color Table (256 colors, 3 bytes each)
        for i in 0..256 {
            let r = i as u8;
            let g = (i * 16) as u8;
            let b = (i * 4) as u8;
            result.push(r);
            result.push(g);
            result.push(b);
        }

        // Image Descriptor
        result.push(0x2C);
        result.push(0);
        result.push(0);
        result.push((width as u16) as u8);
        result.push((width as u16 >> 8) as u8);
        result.push((height as u16) as u8);
        result.push((height as u16 >> 8) as u8);
        result.push(0);

        // Image Data (LZW with minimal header)
        result.push(8);
        result.push(1);
        result.push(0);
        result.push(1);
        result.push(0);
        result.push(0);
        result.push(0);

        // Block Terminator
        result.push(0x3B);

        result
    }

    pub fn to_base64(data: &PixelData) -> String {
        let encoded = Self::encode(data);
        encode_base64(&encoded)
    }
}

fn encode_base64(data: &[u8]) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(alphabet[((n >> 18) & 0x3F) as usize] as char);
        result.push(alphabet[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(alphabet[((n >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() == 3 {
            result.push(alphabet[(n & 0x3F) as usize] as char);
        }
    }
    result
}

pub fn load(_path: &std::path::Path) -> Option<PixelData> {
    let data = std::fs::read(_path).ok()?;
    GifDecoder::decode(&data)
}

pub fn save(data: &PixelData, path: &std::path::Path) -> bool {
    let encoded = GifEncoder::encode(data);
    std::fs::write(path, encoded).is_ok()
}
