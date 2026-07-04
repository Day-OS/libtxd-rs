//! Indexed-color palette conversion helpers backed by `imagequant`.

use crate::error::{Result, TxdError};
use image::RgbaImage;

pub fn generate_palette(rgba: &RgbaImage, palette_size: u32) -> Result<(Vec<u8>, Vec<u8>)> {
    if palette_size != 16 && palette_size != 256 {
        return Err(TxdError::InvalidPalette);
    }
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return Err(TxdError::InvalidDimensions { width, height });
    }

    let pixels: Vec<imagequant::RGBA> = rgba
        .pixels()
        .map(|p| imagequant::RGBA::new(p[0], p[1], p[2], p[3]))
        .collect();

    let mut attr = imagequant::new();
    attr.set_max_colors(palette_size)?;
    attr.set_speed(5)?;

    let mut image = attr.new_image(pixels, width as usize, height as usize, 0.0)?;
    let mut result = attr.quantize(&mut image)?;
    let (palette_rgba, indexed) = result.remapped(&mut image)?;

    let mut palette = Vec::with_capacity(palette_size as usize * 4);
    for c in &palette_rgba {
        palette.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }
    palette.resize(palette_size as usize * 4, 0);

    Ok((palette, indexed))
}

pub fn convert_palette_to_rgba(
    indexed_data: &[u8],
    palette: &[u8],
    palette_size: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage> {
    if indexed_data.is_empty() || palette.is_empty() || width == 0 || height == 0 {
        return Err(TxdError::InvalidDimensions { width, height });
    }

    let mut output = vec![0u8; (width as usize) * (height as usize) * 4];
    for (dst, &raw_index) in output.chunks_exact_mut(4).zip(indexed_data) {
        let index = if (raw_index as u32) < palette_size {
            raw_index as usize
        } else {
            0
        };
        if let Some(entry) = palette.get(index * 4..index * 4 + 4) {
            dst.copy_from_slice(entry);
        }
    }
    RgbaImage::from_raw(width, height, output).ok_or(TxdError::InvalidDimensions { width, height })
}
