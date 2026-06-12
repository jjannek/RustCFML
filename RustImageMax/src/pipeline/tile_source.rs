use crate::tile::pixel::{PixelData, PixelFormat};

pub trait TileSource {
    fn image_width(&self) -> usize;
    fn image_height(&self) -> usize;
    fn image_format(&self) -> PixelFormat;
    fn read_region(&self, x: usize, y: usize, width: usize, height: usize, out: &mut Vec<u8>);
}

impl TileSource for PixelData {
    fn image_width(&self) -> usize {
        self.width()
    }

    fn image_height(&self) -> usize {
        self.height()
    }

    fn image_format(&self) -> PixelFormat {
        self.format()
    }

    fn read_region(&self, x: usize, y: usize, width: usize, height: usize, out: &mut Vec<u8>) {
        out.clear();
        for dy in 0..height {
            for dx in 0..width {
                let pixel = self.get_pixel(x + dx, y + dy);
                for val in &pixel {
                    out.push(*val as u8);
                }
            }
        }
    }
}
