// Access patterns for tile processing
// Determines how tiles are fetched and processed

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessPattern {
    /// Sequential top-to-bottom, left-to-right
    /// Best for single-pass operations
    #[default]
    Sequential,
    
    /// Random access - tiles fetched as needed
    /// Best for compositing and overlay
    Random,
    
    /// Sequential horizontal (left-to-right, row by row)
    /// Best for convolution operations
    SequentialHorizontal,
    
    /// Sequential vertical (top-to-bottom, column by column)
    /// Best for line-based operations
    SequentialVertical,
    
    /// Demand-driven with horizontal threading
    /// Tiles computed in parallel, results streamed
    DemandDriven,
}

impl AccessPattern {
    pub fn supports_threading(&self) -> bool {
        matches!(self, AccessPattern::Random | AccessPattern::DemandDriven)
    }
}
