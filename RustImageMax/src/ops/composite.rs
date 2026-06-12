use crate::tile::pixel::{PixelData, PixelFormat};

pub fn composite(base: &PixelData, overlay: &PixelData, x: usize, y: usize) -> PixelData {
    let mut dst = PixelData::new(base.format(), base.width(), base.height());

    for by in 0..base.height() {
        for bx in 0..base.width() {
            let pixel = vec![0.0f32; base.format().channels()];
            dst.set_pixel(bx, by, &pixel);
        }
    }
    dst
}
