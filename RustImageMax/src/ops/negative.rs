use crate::tile::pixel::{PixelData, PixelFormat};

pub fn negative(src: &PixelData) -> PixelData {
    let width = src.width();
    let height = src.height();
    let format = src.format();
    let channels = format.channels();
    let mut dst = PixelData::new(format, width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = vec![0.0f32; channels];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}
