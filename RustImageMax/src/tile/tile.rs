use super::pixel::{PixelData, PixelFormat};

/// A tile is a rectangular region of an image.
/// It's the basic unit of demand-driven processing.
#[derive(Debug, Clone)]
pub struct Tile {
    /// Pixel data
    data: PixelData,
    /// Position of this tile in the parent image (top-left corner)
    x: usize,
    y: usize,
}

impl Tile {
    pub fn new(format: PixelFormat, width: usize, height: usize, x: usize, y: usize) -> Self {
        Tile {
            data: PixelData::new(format, width, height),
            x,
            y,
        }
    }

    pub fn from_data(data: PixelData, x: usize, y: usize) -> Self {
        Tile { data, x, y }
    }

    /// Position in parent image
    pub fn x(&self) -> usize {
        self.x
    }

    pub fn y(&self) -> usize {
        self.y
    }

    pub fn width(&self) -> usize {
        self.data.width()
    }

    pub fn height(&self) -> usize {
        self.data.height()
    }

    pub fn format(&self) -> PixelFormat {
        self.data.format()
    }

    pub fn data(&self) -> &PixelData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut PixelData {
        &mut self.data
    }

    /// Get pixel value at local coordinates
    pub fn get_pixel(&self, x: usize, y: usize) -> Vec<f32> {
        self.data.get_pixel(x, y)
    }

    /// Set pixel value at local coordinates
    pub fn set_pixel(&mut self, x: usize, y: usize, values: &[f32]) {
        self.data.set_pixel(x, y, values);
    }

    /// Get pixel at global coordinates (accounting for tile position)
    pub fn get_global_pixel(&self, gx: usize, gy: usize) -> Option<Vec<f32>> {
        let lx = gx.checked_sub(self.x)?;
        let ly = gy.checked_sub(self.y)?;
        if lx < self.width() && ly < self.height() {
            Some(self.get_pixel(lx, ly))
        } else {
            None
        }
    }
}
