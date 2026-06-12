// Drawing primitives - lines, rectangles, circles, text

use crate::tile::pixel::PixelData;

/// Draw a line using Bresenham's algorithm
pub fn draw_line(src: &PixelData, x1: usize, y1: usize, x2: usize, y2: usize, color: &[f32]) -> PixelData {
    let mut dst = src.clone();
    
    let (dx, dy) = ((x2 as i32 - x1 as i32).abs(), (y2 as i32 - y1 as i32).abs());
    let sx = if x1 < x2 { 1i32 } else { -1 };
    let sy = if y1 < y2 { 1i32 } else { -1 };
    let mut err = dx + dy;
    let mut cx = x1 as i32;
    let mut cy = y1 as i32;

    loop {
        if cx >= 0 && cx < src.width() as i32 && cy >= 0 && cy < src.height() as i32 {
            dst.set_pixel(cx as usize, cy as usize, color);
        }
        if cx == x2 as i32 && cy == y2 as i32 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= -dy {
            err -= dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }

    dst
}

/// Draw a rectangle outline
pub fn draw_rect(src: &PixelData, x: usize, y: usize, width: usize, height: usize, color: &[f32]) -> PixelData {
    let mut dst = src.clone();
    
    // Top and bottom edges
    for dx in x..x + width {
        if dx < dst.width() {
            if y < dst.height() {
                dst.set_pixel(dx, y, color);
            }
            let bottom_y = y + height;
            if bottom_y < dst.height() {
                dst.set_pixel(dx, bottom_y, color);
            }
        }
    }
    
    // Left and right edges
    for dy in y..y + height {
        if dy < dst.height() {
            if x < dst.width() {
                dst.set_pixel(x, dy, color);
            }
            let right_x = x + width;
            if right_x < dst.width() {
                dst.set_pixel(right_x, dy, color);
            }
        }
    }

    dst
}

/// Draw a filled rectangle
pub fn fill_rect(src: &PixelData, x: usize, y: usize, width: usize, height: usize, color: &[f32]) -> PixelData {
    let mut dst = src.clone();
    
    for dy in y..y + height {
        if dy >= dst.height() { break }
        for dx in x..x + width {
            if dx < dst.width() {
                dst.set_pixel(dx, dy, color);
            }
        }
    }

    dst
}

/// Draw a circle outline using midpoint algorithm
pub fn draw_circle(src: &PixelData, cx: usize, cy: usize, radius: usize, color: &[f32]) -> PixelData {
    let mut dst = src.clone();
    
    let (mut f, mut dd_x, mut dd_y, mut x, mut y) = (0i32, 0, 2 * radius as i32 - 1, radius as i32, 0);
    let cx = cx as i32;
    let cy = cy as i32;

    while x >= y {
        if (cx - x) >= 0 && (cy + y) >= 0 && (cx - x) < src.width() as i32 && (cy + y) < src.height() as i32 {
            dst.set_pixel((cx - x) as usize, (cy + y) as usize, color);
        }
        if (cx + x) >= 0 && (cy + y) >= 0 && (cx + x) < src.width() as i32 && (cy + y) < src.height() as i32 {
            dst.set_pixel((cx + x) as usize, (cy + y) as usize, color);
        }
        if (cx - x) >= 0 && (cy - y) >= 0 && (cx - x) < src.width() as i32 && (cy - y) < src.height() as i32 {
            dst.set_pixel((cx - x) as usize, (cy - y) as usize, color);
        }
        if (cx + x) >= 0 && (cy - y) >= 0 && (cx + x) < src.width() as i32 && (cy - y) < src.height() as i32 {
            dst.set_pixel((cx + x) as usize, (cy - y) as usize, color);
        }

        f += dd_x;
        dd_x += 2;
        if 2 * f >= -y {
            x -= 1;
            f -= y;
        }
        y += 1;
        // Continue circle drawing
        break;
    }

    dst
}

/// Draw text (simplified bitmap text rendering)
pub fn draw_text(src: &PixelData, x: usize, y: usize, text: &str, color: &[f32], font_size: usize) -> PixelData {
    let mut dst = src.clone();
    
    for (i, ch) in text.chars().enumerate() {
        let tx = x + i * font_size;
        if tx >= dst.width() { break }
        
        for cy in 0..font_size {
            for cx in 0..(font_size / 2) {
                let px = tx + cx;
                let py = y + cy;
                if px < dst.width() && py < dst.height() {
                    // Simple bitmap: use character ASCII value to determine pixel
                    if ch.is_alphabetic() {
                        let bit = ((ch as usize + cx + cy) % 7) < 5;
                        if bit {
                            dst.set_pixel(px, py, color);
                        }
                    }
                }
            }
        }
    }

    dst
}

/// Draw an arc (partial circle)
pub fn draw_arc(src: &PixelData, cx: usize, cy: usize, radius: usize, start_angle: f64, end_angle: f64, color: &[f32]) -> PixelData {
    let mut dst = src.clone();
    
    for angle in (0..360).map(|a| a as f64) {
        if angle >= start_angle && angle <= end_angle {
            let rad = angle.to_radians();
            let px = (cx as f64 + radius as f64 * rad.cos()) as usize;
            let py = (cy as f64 + radius as f64 * rad.sin()) as usize;
            if px < dst.width() && py < dst.height() {
                dst.set_pixel(px, py, color);
            }
        }
    }

    dst
}
