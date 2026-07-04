//! Texture-encoding format selection: turning an `RgbaImage` into the raw
//! mipmap bytes and raster metadata a [`crate::texture::Texture`] stores
//! (RGBA32 passthrough, or DXT1/DXT3 compression via `Compression`'s methods).

use crate::error::Result;
use crate::texture::MipmapLevel;
use crate::types::{Compression, platform, raster_format};
use image::{
    RgbaImage,
    imageops::{self, FilterType},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFormat {
    Rgba32,
    Dxt1 { quality: f32 },
    Dxt3 { quality: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureCreateOptions {
    pub platform: u32,
    pub filter_flags: u32,
    pub format: TextureFormat,
    pub mipmaps: bool,
    pub mip_filter: FilterType,
}

impl Default for TextureCreateOptions {
    fn default() -> Self {
        Self {
            platform: platform::D3D9,
            filter_flags: 0x1106,
            format: TextureFormat::Rgba32,
            mipmaps: true,
            mip_filter: FilterType::Triangle,
        }
    }
}

pub fn image_mipmaps(image: RgbaImage, enabled: bool, filter: FilterType) -> Vec<RgbaImage> {
    let mut mipmaps = vec![image];
    if !enabled {
        return mipmaps;
    }

    while let Some(previous) = mipmaps.last() {
        let (width, height) = previous.dimensions();
        if width == 1 && height == 1 {
            break;
        }

        mipmaps.push(imageops::resize(
            previous,
            (width / 2).max(1),
            (height / 2).max(1),
            filter,
        ));
    }

    mipmaps
}

impl TextureFormat {
    pub(crate) fn compression(self) -> Compression {
        match self {
            TextureFormat::Rgba32 => Compression::None,
            TextureFormat::Dxt1 { .. } => Compression::Dxt1,
            TextureFormat::Dxt3 { .. } => Compression::Dxt3,
        }
    }

    pub(crate) fn encode_mipmap(self, image: &RgbaImage) -> Result<MipmapLevel> {
        let (width, height) = image.dimensions();
        let data = match self {
            TextureFormat::Rgba32 => image.clone().into_raw_bgra(),
            TextureFormat::Dxt1 { quality } => Compression::Dxt1.compress_to_dxt(image, quality)?,
            TextureFormat::Dxt3 { quality } => Compression::Dxt3.compress_to_dxt(image, quality)?,
        };

        Ok(MipmapLevel {
            width,
            height,
            data,
        })
    }

    pub(crate) fn raster_format(self, mipmap_count: usize) -> u32 {
        let format = match self {
            TextureFormat::Rgba32 | TextureFormat::Dxt1 { .. } | TextureFormat::Dxt3 { .. } => {
                raster_format::B8G8R8A8
            }
        };

        if mipmap_count > 1 {
            format | raster_format::MIPMAP
        } else {
            format
        }
    }

    pub(crate) fn depth(self) -> u8 {
        match self {
            TextureFormat::Rgba32 | TextureFormat::Dxt1 { .. } | TextureFormat::Dxt3 { .. } => 32,
        }
    }
}
