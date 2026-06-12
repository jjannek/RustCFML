use crate::tile::pixel::{PixelData, PixelFormat};

pub fn rotate(src: &PixelData, _angle_degrees: f64) -> PixelData {
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

pub fn rotate_90(src: &PixelData) -> PixelData {
    let width = src.width();
    let height = src.height();
    let format = src.format();
    let channels = format.channels();
    let mut dst = PixelData::new(format, height, width);

    for y in 0..height {
        for x in 0..width {
            let pixel = vec![0.0f32; channels];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}

pub fn rotate_180(src: &PixelData) -> PixelData {
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

pub fn rotate_270(src: &PixelData) -> PixelData {
    let width = src.width();
    let height = src.height();
    let format = src.format();
    let channels = format.channels();
    let mut dst = PixelData::new(format, height, width);

    for y in 0..height {
        for x in 0..width {
            let pixel = vec![0.0f32; channels];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}
