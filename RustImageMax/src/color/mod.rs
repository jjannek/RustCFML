use crate::tile::pixel::{PixelData, PixelFormat};

pub fn brightness(src: &PixelData, delta: f32) -> PixelData {
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

pub fn contrast(src: &PixelData, factor: f32) -> PixelData {
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

pub fn saturation(src: &PixelData, factor: f32) -> PixelData {
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

pub fn hue_rotate(src: &PixelData, _hue: f32) -> PixelData {
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
