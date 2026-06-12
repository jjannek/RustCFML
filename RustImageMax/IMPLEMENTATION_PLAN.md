# RustImageMax Implementation Plan

## Status: Compiling Skeleton (0 errors, ~45 warnings)

The project compiles clean but every operation returns empty/dummy pixel data.

---

## What's Built (Infrastructure Only)

- Core types: `PixelFormat`, `PixelData`, `Tile`, `ImageMeta` (compiling)
- Module structure: `tile/`, `ops/`, `formats/`, `draw/`, `color/`, `pipeline/`, `image/`
- Cargo.toml with `num-traits`, `bytemuck`, `criterion` dependencies
- 33 source files, ~1,741 lines total

---

## What's NOT Done

### 1. Image Operations (All Return Zeros)

**Resize** (`src/ops/resize.rs`)
- `resize()` - computes source coords but writes zero-filled pixels instead of copying
- `resize_nearest()` - same, computes coords, writes zeros
- `resize_bilinear()` - computes 4 corners but writes zeros (no interpolation)
- `resize_bicubic()` - writes zeros (no 2x2 kernel applied)

**Blur** (`src/ops/blur.rs`)
- `blur()` - iterates pixels but writes zeros (no kernel convolution)
- `box_blur()` - writes zeros (no averaging)

**Sharpen** (`src/ops/sharpen.rs`)
- `sharpen()` - writes zeros (no unsharp mask / Laplacian kernel)

**Grayscale** (`src/ops/grayscale.rs`)
- `grayscale()` - writes zeros (no 0.299R + 0.587G + 0.114B formula)

**Rotate** (`src/ops/rotate.rs`)
- `rotate()` - writes zeros (no affine transform)
- `rotate_90()`, `rotate_180()`, `rotate_270()` - all write zeros

**Flip** (`src/ops/flip.rs`)
- `flip_horizontal()` - writes zeros (no X-axis mirror)
- `flip_vertical()` - writes zeros (no Y-axis mirror)

**Border** (`src/ops/border.rs`)
- `add_border()` - writes zeros (no padding or color fill)

**Negative** (`src/ops/negative.rs`)
- `negative()` - writes zeros (no 255 - val inversion)

**Composite** (`src/ops/composite.rs`)
- `composite()` - writes zeros (no alpha blending)

**Crop** (`src/ops/crop.rs`)
- Need to verify current state

### 2. Color Adjustments (All Return Zeros)

`src/color/mod.rs`:
- `brightness()` - writes zeros (no delta addition)
- `contrast()` - writes zeros (no factor * (val - 128) + 128)
- `saturation()` - writes zeros (no luminance scaling)
- `hue_rotate()` - writes zeros (no RGB->HSV->RGB rotation)

### 3. Format Codecs (All Are Stubs)

**JPEG** (`src/formats/jpeg.rs`):
- Decoder: parses SOF marker only (width/height/bits), no DCT/huffman/scan decoding
- Encoder: writes SOI + APP0 + DQT + SOF0 + SOS + EOI (no actual pixel data encoding)
- Missing: DCT, quantization, Huffman table, MCU scan, entropy coding

**PNG** (`src/formats/png.rs`):
- Decoder: checks magic bytes, returns None
- Encoder: writes PNG signature + IHDR + empty IDAT + IEND
- Missing: filter types, zlib compression, CRC computation, scanline decoding

**GIF** (`src/formats/gif.rs`):
- Decoder: reads header only
- Encoder: writes GIF89a header + color table + minimal LZW
- Missing: LZW encoding/decoding, GCT/LCT, extension blocks

**TIFF** (`src/formats/tiff.rs`):
- Decoder: checks byte order, returns None
- Encoder: writes header + empty IFD
- Missing: IFD tags, strip/ tile decoding, JPEG compression support

**WebP** (`src/formats/webp.rs`):
- Decoder: checks RIFF header
- Encoder: writes minimal VP8L header
- Missing: VP8/VP8L bitstream, prediction, color cache

**Image Format** (`src/formats/image_format.rs`):
- `ImageFormat` enum exists, but all detection/encoding/decoding functions removed

### 4. Drawing Primitives (Partially Broken)

`src/draw/primitives.rs`:
- `draw_line()` - Bresenham implemented (may work)
- `draw_rect()` - edge drawing implemented (may work)
- `fill_rect()` - fill implemented (may work)
- `draw_circle()` - has `break` on line 116 that exits after first iteration
- `draw_text()` - basic bitmap text (may work)
- `draw_arc()` - angle iteration implemented (may work)

### 5. Pipeline (Skeleton)

`src/pipeline/`:
- `TileProcessor` - tile bounds computation exists, process_tile reads data
- `TileSource` - trait defined, PixelData implements it
- `TileCache` - basic HashMap wrapper, not true LRU
- No transform chaining, no threading, no demand-driven execution

### 6. Missing Infrastructure

- **Tests**: No `#[cfg(test)]` module, no test images, no property-based tests
- **Error types**: Everything returns `Option` instead of `Result<ImageError>`
- **Base64 encoding**: Functions exist in jpeg/webp but were removed during cleanup
- **Format detection**: No `detect_format()` function
- **High-level API**: `ImageProcessor` only chains operations, no `load()`/`save()`/`encode()`/`decode()`
- **Criterion benchmarks**: `criterion` is a dependency but no `benches/` folder

---

## Implementation Priority

### Phase 1: Core Operations (Make Things Work)
1. Fix resize functions (copy actual pixel data)
2. Implement blur (separable Gaussian kernel)
3. Implement grayscale (RGB weighting)
4. Implement flip (X/Y mirror)
5. Implement negative (255 - val)
6. Implement crop (region extraction)
7. Implement composite (alpha blend)
8. Fix circle drawing (remove break)

### Phase 2: Color & Drawing
1. Implement brightness, contrast, saturation, hue_rotate
2. Implement border
3. Verify line, rect, fill_rect, arc work correctly
4. Add draw tests

### Phase 3: Format Codecs
1. JPEG: implement full DCT/huffman encoder, basic decoder
2. PNG: implement zlib compression, scanline filtering
3. GIF: implement LZW encoding/decoding
4. Add format detection from magic bytes

### Phase 4: Pipeline
1. Implement transform chaining (Resize -> Blur -> Crop)
2. Implement LRU cache with eviction
3. Add threading for parallel tile processing
4. Implement demand-driven execution

### Phase 5: Polish
1. Error types (`ImageError` enum)
2. Base64 encoding utilities
3. High-level API: `ImageProcessor::load()`, `save()`, `to_base64()`
4. Criterion benchmarks for all operations
5. Integration tests with real images

---

## Testing Strategy

Need a separate test harness since this is a standalone Rust crate:

**Unit Tests** (per module):
- `#[test]` functions in each source file
- Test with small, known pixel values (e.g., 4x4 RGB image)

**Property Tests** (optional, with `quickcheck` or `proptest`):
- Round-trip encode/decode preserves dimensions
- Resize to same size ≈ identity
- Flip twice ≈ identity

**Integration Tests** (`tests/` folder):
- Create test images from `PixelData::new()`
- Test each operation produces non-zero, expected output
- Compare against `minifb`, `stb_image`, or `image` crate as reference

**Benchmark Tests** (Criterion):
- Time each operation on 100x100, 500x500, 1000x1000 images
- Compare memory usage for tile-based vs. full-image processing
