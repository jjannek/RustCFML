use crate::tile::pixel::PixelFormat;
use std::sync::Arc;

/// Image metadata
#[derive(Debug, Clone)]
pub struct ImageMeta {
    width: usize,
    height: usize,
    format: PixelFormat,
    /// Tile size used for demand-driven processing
    tile_width: usize,
    tile_height: usize,
}

impl ImageMeta {
    pub fn new(width: usize, height: usize, format: PixelFormat) -> Self {
        ImageMeta {
            width,
            height,
            format,
            tile_width: 256,
            tile_height: 256,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn tile_width(&self) -> usize {
        self.tile_width
    }

    pub fn tile_height(&self) -> usize {
        self.tile_height
    }

    pub fn num_tiles_x(&self) -> usize {
        (self.width + self.tile_width - 1) / self.tile_width
    }

    pub fn num_tiles_y(&self) -> usize {
        (self.height + self.tile_height - 1) / self.tile_height
    }

    /// Get the tile coordinates for a global pixel position
    pub fn tile_coords(&self, x: usize, y: usize) -> (usize, usize) {
        (x / self.tile_width, y / self.tile_height)
    }

    /// Get local coordinates within a tile
    pub fn local_coords(&self, x: usize, y: usize) -> (usize, usize) {
        (x % self.tile_width, y % self.tile_height)
    }
}

/// An image is a demand-driven grid of tiles.
/// Tiles are computed on-demand through the pipeline.
pub struct Image {
    meta: Arc<ImageMeta>,
}

impl Image {
    pub fn new(width: usize, height: usize, format: PixelFormat) -> Self {
        Image {
            meta: Arc::new(ImageMeta::new(width, height, format)),
        }
    }

    pub fn meta(&self) -> &ImageMeta {
        &self.meta
    }

    pub fn width(&self) -> usize {
        self.meta.width()
    }

    pub fn height(&self) -> usize {
        self.meta.height()
    }

    pub fn format(&self) -> PixelFormat {
        self.meta.format()
    }
}
