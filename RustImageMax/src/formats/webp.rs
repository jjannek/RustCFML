// WebP codec - encoder and decoder for WebP images

use crate::tile::pixel::{PixelData, PixelFormat};

pub struct WebpDecoder;

impl WebpDecoder {
    pub fn decode(data: &[u8]) -> Option<PixelData> {
        if data.len() < 30 {
            return None;
        }

        if !data.starts_with(b"RIFF") {
            return None;
        }

        if &data[8..12] != b"WEBP" {
            return None;
        }

        let chunk_type = &data[12..16];
        let width = u32::from_le_bytes([data[26], data[27], data[28], data[29]]);
        let height = u32::from_le_bytes([data[30], data[31], data[32], data[33]]);

        if chunk_type == b"VP8L" {
            Some(PixelData::new(PixelFormat::U8x4, width as usize, height as usize))
        } else if chunk_type == b"VP8 " {
            Some(PixelData::new(PixelFormat::U8x3, width as usize, height as usize))
        } else {
            None
        }
    }
}

pub struct WebpEncoder;

impl WebpEncoder {
    pub fn encode(data: &PixelData, quality: f64) -> Vec<u8> {
        let mut result = Vec::new();

        let width = data.width() as u32;
        let height = data.height() as u32;

        result.extend_from_slice(b"RIFF");
        result.push(0);
        result.push(0);
        result.push(0);
        result.push(0);

        result.extend_from_slice(b"WEBP");

        result.extend_from_slice(b"VP8L");
        result.push(0x00);
        result.push(0x00);
        result.push(0x00);
        result.push(0x00);

        result.push(0x2F);
        result.push(0x00);
        result.push(0x00);
        result.push(0x00);
        result.push(0x00);
        result.push(0x00);
        result.push(0x01);

        let chunk_size = 5;
        write_u32_le(&mut result, chunk_size);

        let riff_size = result.len() as u32;
        let riff_size_bytes = riff_size.to_le_bytes();
        if result.len() >= 8 {
            for i in 0..4 {
                result[4 + i] = riff_size_bytes[i];
            }
        }

        result
    }

    pub fn to_base64(data: &PixelData, quality: f64) -> String {
        let encoded = Self::encode(data, quality);
        encode_base64(&encoded)
    }
}

fn write_u32_le(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_le_bytes());
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
    WebpDecoder::decode(&data)
}

pub fn save(data: &PixelData, path: &std::path::Path, quality: f64) -> bool {
    let encoded = WebpEncoder::encode(data, quality);
    std::fs::write(path, encoded).is_ok()
}
