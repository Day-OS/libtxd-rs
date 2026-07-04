//! Texture model and D3D native TXD read/write support.

use crate::encode::{TextureCreateOptions, image_mipmaps};
use crate::error::{Result, TxdError};
use crate::palette::convert_palette_to_rgba;
use crate::types::{ChunkHeader, Compression, chunk_type, platform, raster_format, write_chunk};
use binrw::{BinRead, BinReaderExt, BinWrite, BinWriterExt};
use image::{Rgba, RgbaImage};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MipmapLevel {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub platform: u32,
    pub name: String,
    pub mask_name: String,
    pub filter_flags: u32,
    pub raster_format: u32,
    pub depth: u8,
    pub has_alpha: bool,
    pub compression: Compression,
    pub mipmaps: Vec<MipmapLevel>,
    pub palette: Vec<u8>,
    pub palette_size: u32,
}

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct D3dTexturePrefix {
    platform: u32,
    filter_flags: u32,
    name: [u8; 32],
    mask_name: [u8; 32],
    raster_format: u32,
}

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct D3dTextureTail {
    width: u16,
    height: u16,
    depth: u8,
    mipmap_count: u8,
    raster_type: u8,
    compression_or_alpha: u8,
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            platform: platform::D3D8,
            name: String::new(),
            mask_name: String::new(),
            filter_flags: 0,
            raster_format: raster_format::DEFAULT,
            depth: 32,
            has_alpha: false,
            compression: Compression::None,
            mipmaps: Vec::new(),
            palette: Vec::new(),
            palette_size: 0,
        }
    }
}

impl Texture {
    pub fn from_d3d_reader<R: Read + Seek>(stream: &mut R) -> Result<Option<Self>> {
        Self::read_d3d_texture(stream)
    }

    fn read_d3d_texture<R: Read + Seek>(stream: &mut R) -> Result<Option<Self>> {
        let header = ChunkHeader::read(stream)?;
        if header.chunk_type != chunk_type::TEXTURE_NATIVE {
            return Ok(None);
        }

        let section_start = stream.stream_position()?;
        let section_end = section_start + header.length as u64;

        let Some(texture) = Self::read_d3d_struct(stream)? else {
            return Ok(None);
        };

        stream.seek(SeekFrom::Start(section_end))?;
        Ok(Some(texture))
    }

    fn read_d3d_struct<R: Read + Seek>(stream: &mut R) -> Result<Option<Self>> {
        let struct_header = ChunkHeader::read(stream)?;
        if struct_header.chunk_type != chunk_type::STRUCT {
            return Ok(None);
        }

        let struct_start = stream.stream_position()?;
        let struct_end = struct_start + struct_header.length as u64;

        let prefix = D3dTexturePrefix::read(stream)?;
        if !prefix.is_d3d_platform() {
            return Ok(None);
        }

        let mut fourcc = [0u8; 4];
        let mut has_alpha = false;
        if prefix.platform == platform::D3D9 {
            stream.read_exact(&mut fourcc)?;
        } else {
            let alpha_val: u32 = stream.read_le()?;
            has_alpha = alpha_val == 1;
        }

        let tail = D3dTextureTail::read(stream)?;
        let compression = if prefix.platform == platform::D3D9 {
            has_alpha = tail.compression_or_alpha & 0x1 != 0;
            Compression::from_d3d9(fourcc, tail.compression_or_alpha)
        } else {
            match tail.compression_or_alpha {
                1 => Compression::Dxt1,
                3 => Compression::Dxt3,
                _ => Compression::None,
            }
        };
        let palette_size = prefix.palette_size();
        let palette = read_palette(stream, palette_size)?;
        let mipmaps = tail.read_mipmaps(stream, compression)?;

        stream.seek(SeekFrom::Start(struct_end))?;
        Ok(Some(Self {
            platform: prefix.platform,
            name: fixed_name_to_string(&prefix.name),
            mask_name: fixed_name_to_string(&prefix.mask_name),
            filter_flags: prefix.filter_flags,
            raster_format: prefix.raster_format,
            depth: tail.depth,
            has_alpha,
            compression,
            mipmaps,
            palette,
            palette_size,
        }))
    }

    pub fn write_d3d<W: Write + Seek>(&self, stream: &mut W, version: u32) -> Result<u32> {
        write_chunk(stream, chunk_type::TEXTURE_NATIVE, version, |stream| {
            self.write_d3d_struct(stream, version)?;

            ChunkHeader {
                chunk_type: chunk_type::EXTENSION,
                length: 0,
                version,
            }
            .write(stream)
        })
    }

    fn write_d3d_struct<W: Write + Seek>(&self, stream: &mut W, version: u32) -> Result<u32> {
        write_chunk(stream, chunk_type::STRUCT, version, |stream| {
            D3dTexturePrefix {
                platform: self.platform,
                filter_flags: self.filter_flags,
                name: fixed_name_bytes(&self.name),
                mask_name: fixed_name_bytes(&self.mask_name),
                raster_format: self.raster_format,
            }
            .write(stream)?;

            if self.platform == platform::D3D8 {
                stream.write_le(&(if self.has_alpha { 1u32 } else { 0 }))?;
            } else if let Some(fourcc) = self.d3d9_fourcc() {
                stream.write_all(fourcc)?;
            } else {
                stream.write_le(&(if self.has_alpha { 0x15u32 } else { 0x16 }))?;
            }

            let (width, height) = self
                .mipmaps
                .first()
                .map(|m| (m.width, m.height))
                .unwrap_or_default();
            D3dTextureTail {
                width: width as u16,
                height: height as u16,
                depth: self.depth,
                mipmap_count: self.mipmaps.len() as u8,
                raster_type: 0x4,
                compression_or_alpha: self.compression_or_alpha_byte(),
            }
            .write(stream)?;

            if self.palette_size > 0 && !self.palette.is_empty() {
                stream.write_all(&self.palette)?;
            }

            for mipmap in &self.mipmaps {
                stream.write_le(&(mipmap.data.len() as u32))?;
                stream.write_all(&mipmap.data)?;
            }

            Ok(())
        })
    }

    fn compression_or_alpha_byte(&self) -> u8 {
        if self.platform == platform::D3D8 {
            self.compression as u8
        } else {
            (if self.compression != Compression::None {
                8
            } else {
                0
            }) | (if self.has_alpha { 1 } else { 0 })
        }
    }

    fn d3d9_fourcc(&self) -> Option<&'static [u8; 4]> {
        match self.compression {
            Compression::Dxt1 => Some(b"DXT1"),
            Compression::Dxt3 => Some(b"DXT3"),
            Compression::None => None,
        }
    }
}

impl D3dTexturePrefix {
    fn is_d3d_platform(&self) -> bool {
        self.platform == platform::D3D8 || self.platform == platform::D3D9
    }

    fn palette_size(&self) -> u32 {
        if self.raster_format & raster_format::PAL8 != 0 {
            256
        } else if self.raster_format & raster_format::PAL4 != 0 {
            16
        } else {
            0
        }
    }
}

impl D3dTextureTail {
    fn read_mipmaps<R: Read + Seek>(
        &self,
        stream: &mut R,
        compression: Compression,
    ) -> Result<Vec<MipmapLevel>> {
        let mut mipmaps = Vec::with_capacity(self.mipmap_count as usize);
        let mut width = self.width as u32;
        let mut height = self.height as u32;

        for level in 0..self.mipmap_count as u32 {
            if level > 0 {
                width = compression.next_mipmap_dimension(width);
                height = compression.next_mipmap_dimension(height);
            }

            let mip_size: u32 = stream.read_le()?;
            if mip_size == 0 {
                width = 0;
                height = 0;
            }

            let mut data = vec![0u8; mip_size as usize];
            stream.read_exact(&mut data)?;
            mipmaps.push(MipmapLevel {
                width,
                height,
                data,
            });
        }

        Ok(mipmaps)
    }
}

fn read_palette<R: Read>(stream: &mut R, palette_size: u32) -> Result<Vec<u8>> {
    if palette_size == 0 {
        return Ok(Vec::new());
    }

    let mut palette = vec![0u8; (palette_size * 4) as usize];
    stream.read_exact(&mut palette)?;
    Ok(palette)
}

fn fixed_name_to_string(bytes: &[u8]) -> String {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

fn fixed_name_bytes<const N: usize>(value: &str) -> [u8; N] {
    let mut buf = [0u8; N];
    let bytes = value.as_bytes();
    let n = bytes.len().min(N - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

impl Texture {
    pub fn from_rgba(
        name: impl Into<String>,
        rgba: RgbaImage,
        options: TextureCreateOptions,
    ) -> Result<Self> {
        let (width, height) = rgba.dimensions();
        if width == 0 || height == 0 {
            return Err(TxdError::InvalidDimensions { width, height });
        }

        let images = image_mipmaps(rgba, options.mipmaps, options.mip_filter);
        let has_alpha = images[0].pixels().any(|pixel| pixel[3] < 255);
        let compression = options.format.compression();
        let mipmaps = images
            .iter()
            .map(|image| options.format.encode_mipmap(image))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            platform: options.platform,
            name: name.into(),
            mask_name: String::new(),
            filter_flags: options.filter_flags,
            raster_format: options.format.raster_format(mipmaps.len()),
            depth: options.format.depth(),
            has_alpha,
            compression,
            mipmaps,
            palette: Vec::new(),
            palette_size: 0,
        })
    }

    pub fn can_convert(&self) -> bool {
        self.is_palette_format()
            || matches!(
                self.compression,
                Compression::None | Compression::Dxt1 | Compression::Dxt3
            )
    }

    pub fn to_rgba8(&self, mipmap_index: usize) -> Result<RgbaImage> {
        let mipmap = self
            .mipmaps
            .get(mipmap_index)
            .ok_or(TxdError::MissingMipmap(mipmap_index))?;
        if mipmap.width == 0 || mipmap.height == 0 || mipmap.data.is_empty() {
            return Err(TxdError::InvalidDimensions {
                width: mipmap.width,
                height: mipmap.height,
            });
        }

        if self.is_palette_format() {
            if self.palette_size == 0 || (self.palette.len() as u32) < self.palette_size * 4 {
                return Ok(RgbaImage::from_pixel(
                    mipmap.width,
                    mipmap.height,
                    Rgba([0, 0, 0, 0]),
                ));
            }
            return convert_palette_to_rgba(
                &mipmap.data,
                &self.palette,
                self.palette_size,
                mipmap.width,
                mipmap.height,
            );
        }

        match self.compression {
            Compression::Dxt1 | Compression::Dxt3 => {
                self.compression
                    .decompress_dxt(&mipmap.data, mipmap.width, mipmap.height)
            }
            Compression::None => Ok(self.convert_uncompressed(mipmap)),
        }
    }

    fn is_palette_format(&self) -> bool {
        self.raster_format & raster_format::PAL8 != 0
            || self.raster_format & raster_format::PAL4 != 0
    }

    fn convert_uncompressed(&self, mipmap: &MipmapLevel) -> RgbaImage {
        let format_mask = self.raster_format & raster_format::MASK;
        let bpp = match (self.depth / 8) as usize {
            0 => 4,
            n => n,
        };

        let mut output = RgbaImage::new(mipmap.width, mipmap.height);
        for (dst, src) in output.pixels_mut().zip(mipmap.data.chunks_exact(bpp)) {
            *dst = Rgba(decode_uncompressed_pixel(format_mask, src));
        }
        output
    }
}

fn decode_uncompressed_pixel(format_mask: u32, pixel_data: &[u8]) -> [u8; 4] {
    match format_mask {
        raster_format::B8G8R8A8 => [pixel_data[2], pixel_data[1], pixel_data[0], pixel_data[3]],
        raster_format::B8G8R8 => [pixel_data[2], pixel_data[1], pixel_data[0], 255],
        raster_format::R5G6B5 => {
            let pixel = u16::from_le_bytes([pixel_data[0], pixel_data[1]]);
            [
                (((pixel >> 11) & 0x1F) << 3) as u8,
                (((pixel >> 5) & 0x3F) << 2) as u8,
                ((pixel & 0x1F) << 3) as u8,
                255,
            ]
        }
        raster_format::A1R5G5B5 => {
            let pixel = u16::from_le_bytes([pixel_data[0], pixel_data[1]]);
            [
                (((pixel >> 10) & 0x1F) << 3) as u8,
                (((pixel >> 5) & 0x1F) << 3) as u8,
                ((pixel & 0x1F) << 3) as u8,
                if (pixel >> 15) & 0x1 != 0 { 255 } else { 0 },
            ]
        }
        raster_format::R4G4B4A4 => {
            let pixel = u16::from_le_bytes([pixel_data[0], pixel_data[1]]);
            [
                (((pixel >> 12) & 0xF) << 4) as u8,
                (((pixel >> 8) & 0xF) << 4) as u8,
                (((pixel >> 4) & 0xF) << 4) as u8,
                ((pixel & 0xF) << 4) as u8,
            ]
        }
        raster_format::LUM8 => [pixel_data[0], pixel_data[0], pixel_data[0], 255],
        _ => [0, 0, 0, 255],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_from_rgba_builds_mipmapped_texture() {
        let source = RgbaImage::from_fn(4, 2, |x, y| Rgba([x as u8 * 40, y as u8 * 80, 120, 255]));

        let texture =
            Texture::from_rgba("sample", source.clone(), TextureCreateOptions::default()).unwrap();
        let decoded = texture.to_rgba8(0).unwrap();

        assert_eq!(texture.name, "sample");
        assert_eq!(texture.compression, Compression::None);
        assert_eq!(texture.depth, 32);
        assert_eq!(texture.mipmaps.len(), 3);
        assert_eq!(texture.mipmaps[1].width, 2);
        assert_eq!(texture.mipmaps[1].height, 1);
        assert_eq!(decoded, source);
    }
}
