use crate::tile::pixel::{PixelData, PixelFormat};

pub fn grayscale(src: &PixelData) -> PixelData {
    let width = src.width();
    let height = src.height();
    let format = src.format();
    let channels = format.channels();
    let mut dst = PixelData::new(format, width, height);

    for y in 0..height {
        for x in 0..width {
            let src_pixel = src.get_pixel(x, y);
            if channels >= 3 {
                let r = src_pixel[0];
                let g = src_pixel[1];
                let b = src_pixel[2];
                let gray = 0.299 * r + 0.587 * g + 0.114 * b;
                let pixel = vec![gray; channels];
                dst.set_pixel(x, y, &pixel);
            } else {
                let pixel = vec![0.0f32; channels];
                dst.set_pixel(x, y, &pixel);
            }
        }
    }
    dst
}
