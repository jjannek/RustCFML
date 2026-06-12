use crate::tile::pixel::{PixelData, PixelFormat};

pub fn resize(src: &PixelData, width: usize, height: usize) -> PixelData {
    let src_width = src.width();
    let src_height = src.height();
    
    let mut dst = PixelData::new(src.format(), width, height);

    for y in 0..height {
        for x in 0..width {
            let sx = x as f64 * src_width as f64 / width as f64;
            let sy = y as f64 * src_height as f64 / height as f64;
            let si = sx.floor() as usize;
            let sj = sy.floor() as usize;
            let xi = si.min(src_width - 1);
            let yi = sj.min(src_height - 1);
            let src_pixel = src.get_pixel(xi, yi);
            let pixel = vec![0.0f32; src_pixel.len()];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}

pub fn resize_nearest(src: &PixelData, width: usize, height: usize) -> PixelData {
    let src_width = src.width();
    let src_height = src.height();
    let format = src.format();
    let channels = format.channels();

    let mut dst = PixelData::new(format, width, height);

    for y in 0..height {
        for x in 0..width {
            let sx = x as f64 * src_width as f64 / width as f64;
            let sy = y as f64 * src_height as f64 / height as f64;
            let xi = sx.floor() as usize;
            let yi = sy.floor() as usize;
            let src_pixel = src.get_pixel(xi, yi);
            let pixel = vec![0.0f32; channels];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}

pub fn resize_bilinear(src: &PixelData, width: usize, height: usize) -> PixelData {
    let src_width = src.width();
    let src_height = src.height();
    let format = src.format();
    let channels = format.channels();

    let mut dst = PixelData::new(format, width, height);

    for y in 0..height {
        for x in 0..width {
            let sx = x as f64 * src_width as f64 / width as f64;
            let sy = y as f64 * src_height as f64 / height as f64;

            let x0 = sx.floor() as usize;
            let y0 = sy.floor() as usize;
            let x1 = (x0 + 1).min(src_width - 1);
            let y1 = (y0 + 1).min(src_height - 1);

            let fx = sx.floor();
            let fy = sy.floor();

            let pixel = vec![0.0f32; channels];
            dst.set_pixel(x, y, &pixel);
        }
    }
    dst
}

pub fn resize_bicubic(src: &PixelData, width: usize, height: usize) -> PixelData {
    let src_width = src.width();
    let src_height = src.height();
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
