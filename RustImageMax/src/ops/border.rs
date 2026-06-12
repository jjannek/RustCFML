use crate::tile::pixel::{PixelData, PixelFormat};

pub fn add_border(src: &PixelData, border_width: usize, color: &[f32]) -> PixelData {
    let width = src.width() + border_width * 2;
    let height = src.height() + border_width * 2;
    let format = src.format();
    let mut dst = PixelData::new(format, width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = vec![0.0f32; color.len()];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}
