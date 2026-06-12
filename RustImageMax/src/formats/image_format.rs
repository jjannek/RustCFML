use crate::tile::pixel::PixelData;

#[derive(Debug, Clone, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Tiff,
    Webp,
    Bmp,
}
