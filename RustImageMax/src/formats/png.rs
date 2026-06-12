use crate::tile::pixel::{PixelData, PixelFormat};

pub struct PngDecoder;

impl PngDecoder {
    pub fn decode(data: &[u8]) -> Option<PixelData> {
        if data.len() < 24 || !data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return None;
        }
        None
    }
}

pub struct PngEncoder;

impl PngEncoder {
    pub fn encode(data: &PixelData) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let width = data.width();
        let height = data.height();
        write_png_chunk(&mut result, b"IHDR", &[
            0x00, 0x00, 0x00, (width as u32) as u8,
            0x00, 0x00, 0x00, (height as u32) as u8,
            8, 2, 0, 0, 0,
        ]);
        write_png_chunk(&mut result, b"IDAT", &[]);
        write_png_chunk(&mut result, b"IEND", &[]);
        result
    }
}

fn write_png_chunk(buf: &mut Vec<u8>, id: &[u8], data: &[u8]) {
    let len = data.len() as u32;
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(id);
    buf.extend_from_slice(data);
    buf.extend_from_slice(&0u32.to_be_bytes());
}

pub fn load(_path: &std::path::Path) -> Option<PixelData> {
    let data = std::fs::read(_path).ok()?;
    PngDecoder::decode(&data)
}

pub fn save(data: &PixelData, path: &std::path::Path) -> bool {
    let encoded = PngEncoder::encode(data);
    std::fs::write(path, encoded).is_ok()
}
