pub struct PipelineConfig {
    pub tile_width: usize,
    pub tile_height: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        PipelineConfig {
            tile_width: 256,
            tile_height: 256,
        }
    }
}
