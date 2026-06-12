use crate::pipeline::tile_source::TileSource;

pub struct TileProcessor<S: TileSource> {
    source: S,
    tile_width: usize,
    tile_height: usize,
}

impl<S: TileSource> TileProcessor<S> {
    pub fn new(source: S, tile_width: usize, tile_height: usize) -> Self {
        TileProcessor {
            source,
            tile_width,
            tile_height,
        }
    }

    pub fn process_tile(&self, tx: usize, ty: usize) -> Option<Vec<u8>> {
        let bounds = self.tile_bounds(tx, ty);
        let format = self.source.image_format();
        let width = bounds.2;
        let height = bounds.3;
        
        if width == 0 || height == 0 {
            return None;
        }

        let mut tile_data = vec![0u8; width * height * format.pixel_size()];
        self.source.read_region(bounds.0, bounds.1, width, height, &mut tile_data);
        Some(tile_data)
    }

    fn tile_bounds(&self, tx: usize, ty: usize) -> (usize, usize, usize, usize) {
        let img_w = self.source.image_width();
        let img_h = self.source.image_height();

        let x = tx * self.tile_width;
        let y = ty * self.tile_height;
        let w = self.tile_width.min(img_w.saturating_sub(x));
        let h = self.tile_height.min(img_h.saturating_sub(y));

        (x, y, w, h)
    }
}
