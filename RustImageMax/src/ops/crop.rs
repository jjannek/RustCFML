// Crop operations - extract rectangular regions

use crate::tile::pixel::PixelData;

/// Crop an image to the given region
pub fn crop(src: &PixelData, x: usize, y: usize, width: usize, height: usize) -> PixelData {
    let x = x.min(src.width());
    let y = y.min(src.height());
    let width = width.min(src.width().saturating_sub(x));
    let height = height.min(src.height().saturating_sub(y));

    if width == 0 || height == 0 {
        return PixelData::new(src.format(), 0, 0);
    }

    let mut dst = PixelData::new(src.format(), width, height);

    for dy in 0..height {
        for dx in 0..width {
            let src_pixel = src.get_pixel(x + dx, y + dy);
            if !src_pixel.is_empty() {
                dst.set_pixel(dx, dy, &src_pixel);
            }
        }
    }

    dst
}

/// Crop to content (remove transparent borders)
pub fn crop_to_content(src: &PixelData, alpha_threshold: f32) -> (PixelData, usize, usize, usize, usize) {
    let channels = src.format().channels();
    let has_alpha = channels >= 4;
    let mut min_x = src.width();
    let mut min_y = src.height();
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..src.height() {
        for x in 0..src.width() {
            let pixel = src.get_pixel(x, y);
            if pixel.len() >= channels {
                let alpha = if has_alpha {
                    pixel[3]
                } else {
                    255.0
                };
                if alpha > alpha_threshold {
                    found = true;
                    if x < min_x { min_x = x; }
                    if y < min_y { min_y = y; }
                    if x > max_x { max_x = x; }
                    if y > max_y { max_y = y; }
                }
            }
        }
    }

    if !found {
        let empty = PixelData::new(src.format(), 0, 0);
        return (empty, 0, 0, 0, 0);
    }

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    (crop(src, min_x, min_y, width, height), min_x, min_y, width, height)
}
