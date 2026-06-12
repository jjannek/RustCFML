pub use crate::tile::pixel::{PixelData, PixelFormat};

pub struct ImageProcessor {
    data: PixelData,
}

impl ImageProcessor {
    pub fn new(data: PixelData) -> Self {
        ImageProcessor { data }
    }

    pub fn from_data(data: PixelData) -> Self {
        ImageProcessor { data }
    }

    pub fn width(&self) -> usize {
        self.data.width()
    }

    pub fn height(&self) -> usize {
        self.data.height()
    }

    pub fn data(&self) -> &PixelData {
        &self.data
    }

    pub fn into_data(self) -> PixelData {
        self.data
    }

    pub fn resize(mut self, width: usize, height: usize) -> Self {
        self.data = crate::ops::resize::resize(&self.data, width, height);
        self
    }

    pub fn blur(mut self, radius: usize) -> Self {
        self.data = crate::ops::blur::blur(&self.data, radius);
        self
    }

    pub fn sharpen(mut self, weight: f32) -> Self {
        self.data = crate::ops::sharpen::sharpen(&self.data, weight);
        self
    }

    pub fn grayscale(mut self) -> Self {
        self.data = crate::ops::grayscale::grayscale(&self.data);
        self
    }

    pub fn rotate(mut self, angle_degrees: f64) -> Self {
        self.data = crate::ops::rotate::rotate(&self.data, angle_degrees);
        self
    }

    pub fn flip_horizontal(mut self) -> Self {
        self.data = crate::ops::flip::flip_horizontal(&self.data);
        self
    }

    pub fn flip_vertical(mut self) -> Self {
        self.data = crate::ops::flip::flip_vertical(&self.data);
        self
    }

    pub fn crop(mut self, x: usize, y: usize, width: usize, height: usize) -> Self {
        self.data = crate::ops::crop::crop(&self.data, x, y, width, height);
        self
    }

    pub fn negative(mut self) -> Self {
        self.data = crate::ops::negative::negative(&self.data);
        self
    }

    pub fn composite(mut self, other: PixelData, x: usize, y: usize) -> Self {
        self.data = crate::ops::composite::composite(&self.data, &other, x, y);
        self
    }
}
