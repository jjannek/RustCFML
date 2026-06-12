use crate::tile::pixel::{PixelData, PixelFormat};

pub struct TiffDecoder;

impl TiffDecoder {
    pub fn decode(data: &[u8]) -> Option<PixelData> {
        if data.len() < 8 {
            return None;
        }
        None
    }
}

pub struct TiffEncoder;

impl TiffEncoder {
    pub fn encode(data: &PixelData) -> Vec<u8> {
        let mut result = Vec::new();
        let width = data.width();
        let height = data.height();

        result.extend_from_slice(b"II");
        result.extend_from_slice(&42u16.to_le_bytes());
        result.extend_from_slice(&8u32.to_le_bytes());
        write_ifd(&mut result, width, height);
        result
    }
}

fn write_u16(buf: &mut Vec<u8>, val: u16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

fn write_u32(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_le_bytes());
}

fn write_ifd(buf: &mut Vec<u8>, width: usize, height: usize) {
    write_u16(buf, 0);
    write_u32(buf, 0);
}

pub fn load(_path: &std::path::Path) -> Option<PixelData> {
    let data = std::fs::read(_path).ok()?;
    TiffDecoder::decode(&data)
}

pub fn save(data: &PixelData, path: &std::path::Path) -> bool {
    let encoded = TiffEncoder::encode(data);
    std::fs::write(path, encoded).is_ok()
}
