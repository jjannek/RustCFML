use std::fmt;

/// Supported pixel formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    U8,      // 8-bit unsigned (grayscale)
    U8x3,    // RGB
    U8x4,    // RGBA
    U16,     // 16-bit unsigned (grayscale)
    U16x3,   // 16-bit RGB
    U16x4,   // 16-bit RGBA
    F32,     // 32-bit float
    F32x3,   // 32-bit RGB float
    F32x4,   // 32-bit RGBA float
}

impl PixelFormat {
    /// Number of channels (bands)
    pub fn channels(&self) -> usize {
        match self {
            PixelFormat::U8 | PixelFormat::U16 | PixelFormat::F32 => 1,
            PixelFormat::U8x3 | PixelFormat::U16x3 | PixelFormat::F32x3 => 3,
            PixelFormat::U8x4 | PixelFormat::U16x4 | PixelFormat::F32x4 => 4,
        }
    }

    /// Size of one sample in bytes
    pub fn sample_size(&self) -> usize {
        match self {
            PixelFormat::U8 => 1,
            PixelFormat::U8x3 | PixelFormat::U8x4 => 1,
            PixelFormat::U16 => 2,
            PixelFormat::U16x3 | PixelFormat::U16x4 => 2,
            PixelFormat::F32 => 4,
            PixelFormat::F32x3 | PixelFormat::F32x4 => 4,
        }
    }

    /// Size of one pixel in bytes
    pub fn pixel_size(&self) -> usize {
        self.channels() * self.sample_size()
    }
}

/// A single pixel value as a slice of f32 samples
#[derive(Clone, Debug)]
pub struct Pixel<'a>(&'a [f32]);

impl<'a> Pixel<'a> {
    pub fn new(samples: &'a [f32]) -> Self {
        Pixel(samples)
    }

    pub fn samples(&self) -> &[f32] {
        self.0
    }

    pub fn channel(&self, i: usize) -> f32 {
        self.0[i]
    }
}

/// Owned pixel data
#[derive(Clone, Debug)]
pub struct PixelData {
    data: Vec<u8>,
    format: PixelFormat,
    width: usize,
    height: usize,
}

impl PixelData {
    pub fn new(format: PixelFormat, width: usize, height: usize) -> Self {
        let size = width * height * format.pixel_size();
        PixelData {
            data: vec![0u8; size],
            format,
            width,
            height,
        }
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Get pixel value as f32 array
    pub fn get_pixel(&self, x: usize, y: usize) -> Vec<f32> {
        let channels = self.format.channels();
        let sample_size = self.format.sample_size();
        let start = (y * self.width + x) * self.format.pixel_size();
        let end = start + self.format.pixel_size();

        self.data[start..end]
            .chunks(sample_size)
            .map(|chunk| match self.format {
                PixelFormat::U8 | PixelFormat::U8x3 | PixelFormat::U8x4 => {
                    chunk[0] as f32
                }
                PixelFormat::U16 | PixelFormat::U16x3 | PixelFormat::U16x4 => {
                    let val = u16::from_le_bytes([chunk[0], chunk[1]]);
                    val as f32
                }
                PixelFormat::F32 | PixelFormat::F32x3 | PixelFormat::F32x4 => {
                    f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                }
            })
            .collect()
    }

    /// Set pixel value
    pub fn set_pixel(&mut self, x: usize, y: usize, values: &[f32]) {
        let sample_size = self.format.sample_size();
        let start = (y * self.width + x) * self.format.pixel_size();

        for (i, &val) in values.iter().enumerate() {
            let offset = start + i * sample_size;
            match self.format {
                PixelFormat::U8 | PixelFormat::U8x3 | PixelFormat::U8x4 => {
                    self.data[offset] = val.clamp(0.0, 255.0) as u8;
                }
                PixelFormat::U16 | PixelFormat::U16x3 | PixelFormat::U16x4 => {
                    let val = (val.clamp(0.0, 65535.0) as u16).to_le_bytes();
                    self.data[offset] = val[0];
                    self.data[offset + 1] = val[1];
                }
                PixelFormat::F32 | PixelFormat::F32x3 | PixelFormat::F32x4 => {
                    let bytes = val.to_le_bytes();
                    for (b, &byte) in bytes.iter().enumerate() {
                        self.data[offset + b] = byte;
                    }
                }
            }
        }
    }
}
