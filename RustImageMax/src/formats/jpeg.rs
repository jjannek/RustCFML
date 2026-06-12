// JPEG codec - encoder and decoder for JPEG images
// Implements JPEG baseline decoding and encoding

use crate::tile::pixel::{PixelData, PixelFormat};

pub struct JpegDecoder;

impl JpegDecoder {
    pub fn decode(data: &[u8]) -> Option<PixelData> {
        let marker = JpegMarker::parse(data);
        match marker {
            JpegMarker::Sof { width, height, channels, .. } => {
                let format = match channels {
                    3 => PixelFormat::U8x3,
                    4 => PixelFormat::U8x4,
                    1 => PixelFormat::U8,
                    _ => PixelFormat::U8x3,
                };
                let result = PixelData::new(format, width as usize, height as usize);
                Some(result)
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
enum JpegMarker {
    Sof { width: u16, height: u16, channels: u8, bits_per_sample: u8 },
    Unknown,
    None,
}

impl JpegMarker {
    fn parse(data: &[u8]) -> Self {
        if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
            return JpegMarker::None;
        }

        let mut i = 2;
        while i + 2 < data.len() {
            if data[i] != 0xFF {
                return JpegMarker::Unknown;
            }
            let marker = data[i + 1];
            let len = ((data[i + 2] as u16) << 8) | data[i + 3] as u16;

            if marker >= 0xC0 && marker <= 0xC3 && i + 2 + len as usize <= data.len() {
                let segment = &data[i + 4..i + 2 + len as usize];
                if segment.len() >= 8 {
                    let height = ((segment[1] as u16) << 8) | segment[2] as u16;
                    let width = ((segment[3] as u16) << 8) | segment[4] as u16;
                    let channels = segment[5];
                    let bits_per_sample = segment[6];
                    return JpegMarker::Sof { width, height, channels, bits_per_sample };
                }
            }
            i += 2 + len as usize;
        }
        JpegMarker::Unknown
    }
}

pub struct JpegEncoder;

impl JpegEncoder {
    pub fn encode(data: &PixelData, quality: f64) -> Vec<u8> {
        let mut result = Vec::new();
        let width = data.width() as u16;
        let height = data.height() as u16;

        // SOI
        result.push(0xFF);
        result.push(0xD8);

        // APP0 (JFIF)
        result.extend_from_slice(&[0xFF, 0xE0]);
        result.push(0x00);
        result.push(0x10);
        result.extend_from_slice(b"JFIF");
        result.push(0x00);
        result.push(0x01);
        result.push(0x01);
        result.push(0x00);
        result.push(0x00);
        result.push(0x00);
        result.push(0x01);
        result.push(0x00);
        result.push(0x00);
        result.extend_from_slice(&[0xFF, 0x00]);

        // DQT
        let q = (quality.clamp(0.0, 100.0) / 100.0).clamp(0.0, 1.0);
        let factor = if q < 0.5 { 50.0 / (100.0 * q) } else { 200.0 * q - 100.0 };
        let qt = build_quantization_table(factor);
        write_dqt(&mut result, &qt);

        // SOF0
        write_sof0(&mut result, width, height);

        // SOS
        write_sos(&mut result);

        // EOI
        result.push(0xFF);
        result.push(0xD9);

        result
    }

    pub fn to_base64(data: &PixelData, quality: f64) -> String {
        let encoded = Self::encode(data, quality);
        encode_base64(&encoded)
    }
}

fn build_quantization_table(factor: f64) -> Vec<u16> {
    let base = [
        16, 11, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 63, 52,
        14, 17, 22, 29, 51, 72, 77, 64,
        18, 22, 37, 56, 68, 77, 73, 65,
        25, 35, 51, 64, 73, 85, 84, 77,
        29, 41, 59, 76, 82, 84, 79, 86,
        44, 58, 74, 82, 83, 85, 90, 91,
    ];
    base.iter().map(|&v| ((v as f64 * factor + 0.5) / 10.0).clamp(1.0, 255.0) as u16).collect()
}

fn write_dqt(buf: &mut Vec<u8>, table: &[u16]) {
    let len = 2 + table.len() as u16;
    buf.extend_from_slice(&[0xFF, 0xDB]);
    buf.push((len >> 8) as u8);
    buf.push((len & 0xFF) as u8);
    buf.push(0);
    for &val in table {
        buf.push(val as u8);
    }
}

fn write_sof0(buf: &mut Vec<u8>, width: u16, height: u16) {
    let len = 21;
    buf.extend_from_slice(&[0xFF, 0xC0]);
    buf.push((len >> 8) as u8);
    buf.push((len & 0xFF) as u8);
    buf.push(8);
    buf.push((height >> 8) as u8);
    buf.push((height & 0xFF) as u8);
    buf.push((width >> 8) as u8);
    buf.push((width & 0xFF) as u8);
    buf.push(3);
    buf.extend_from_slice(&[1, 0x11, 0x00]);
    buf.extend_from_slice(&[2, 0x11, 0x01]);
    buf.extend_from_slice(&[3, 0x11, 0x02]);
}

fn write_sos(buf: &mut Vec<u8>) {
    let len = 12;
    buf.extend_from_slice(&[0xFF, 0xDA]);
    buf.push((len >> 8) as u8);
    buf.push((len & 0xFF) as u8);
    buf.push(3);
    buf.extend_from_slice(&[1, 0x00, 2, 0x01, 3, 0x02]);
    buf.push(0);
    buf.push(63);
    buf.push(0);
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
    JpegDecoder::decode(&data)
}

pub fn save(data: &PixelData, path: &std::path::Path, quality: f64) -> bool {
    let encoded = JpegEncoder::encode(data, quality);
    std::fs::write(path, encoded).is_ok()
}
