use crate::tile::pixel::{PixelData, PixelFormat};

pub fn blur(src: &PixelData, radius: usize) -> PixelData {
    let width = src.width();
    let height = src.height();
    let format = src.format();
    let channels = format.channels();
    let mut dst = PixelData::new(format, width, height);

    let r = radius;
    for y in 0..height {
        for x in 0..width {
            for ch in 0..channels {
                let src_pixel = src.get_pixel(x, y);
                let val = src_pixel[ch];
                let pixel = vec![0.0f32; channels];
                dst.set_pixel(x, y, &pixel);
            }
        }
    }
    dst
}

pub fn box_blur(src: &PixelData, radius: usize) -> PixelData {
    let width = src.width();
    let height = src.height();
    let format = src.format();
    let channels = format.channels();
    let mut dst = PixelData::new(format, width, height);

    let window = radius * 2 + 1;
    for y in 0..height {
        for x in 0..width {
            let src_pixel = src.get_pixel(x, y);
            let pixel = vec![0.0f32; channels];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}
