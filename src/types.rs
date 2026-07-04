//! Shared TXD wire types and constants.

use crate::error::{Result, TxdError};
use binrw::{BinRead, BinWrite, BinWriterExt};
use image::RgbaImage;
use std::io::{Read, Seek, SeekFrom, Write};
use texpresso::{Algorithm as DxtAlgorithm, Format as DxtFormat, Params as DxtParams};

pub mod platform {
    pub const OGL: u32 = 2;
    pub const PS2: u32 = 4;
    pub const XBOX: u32 = 5;
    pub const D3D8: u32 = 8;
    pub const D3D9: u32 = 9;
    pub const PS2_FOURCC: u32 = 0x0032_5350; // b"PS2\0"
}

pub mod chunk_type {
    pub const STRUCT: u32 = 0x01;
    pub const STRING: u32 = 0x02;
    pub const EXTENSION: u32 = 0x03;
    pub const TEXTURE_NATIVE: u32 = 0x15;
    pub const TEX_DICTIONARY: u32 = 0x16;
    pub const SKY_MIPMAP: u32 = 0x110;
}

pub mod raster_format {
    pub const DEFAULT: u32 = 0x0000;
    pub const A1R5G5B5: u32 = 0x0100;
    pub const R5G6B5: u32 = 0x0200;
    pub const R4G4B4A4: u32 = 0x0300;
    pub const LUM8: u32 = 0x0400;
    pub const B8G8R8A8: u32 = 0x0500;
    pub const B8G8R8: u32 = 0x0600;
    pub const R5G5B5: u32 = 0x0A00;

    pub const AUTOMIPMAP: u32 = 0x1000;
    pub const PAL8: u32 = 0x2000;
    pub const PAL4: u32 = 0x4000;
    pub const MIPMAP: u32 = 0x8000;

    pub const MASK: u32 = 0x0F00;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    #[default]
    None = 0,
    Dxt1 = 1,
    Dxt3 = 3,
}

impl Compression {
    pub(crate) fn next_mipmap_dimension(self, value: u32) -> u32 {
        let next = (value / 2).max(1);
        if self != Compression::None && next < 4 && next != 0 {
            4
        } else {
            next
        }
    }

    pub(crate) fn from_d3d9(fourcc: [u8; 4], flags: u8) -> Self {
        if flags & 0x8 == 0 || &fourcc[..3] != b"DXT" {
            return Self::None;
        }

        match fourcc[3] {
            b'1' => Self::Dxt1,
            b'3' => Self::Dxt3,
            _ => Self::None,
        }
    }

    fn dxt_format(self) -> Result<DxtFormat> {
        match self {
            Compression::Dxt1 => Ok(DxtFormat::Bc1),
            Compression::Dxt3 => Ok(DxtFormat::Bc2),
            Compression::None => Err(TxdError::UnsupportedCompression),
        }
    }

    pub fn compressed_size(self, width: u32, height: u32) -> Result<usize> {
        Ok(self
            .dxt_format()?
            .compressed_size(width as usize, height as usize))
    }

    pub fn decompress_dxt(
        self,
        compressed_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<RgbaImage> {
        if compressed_data.is_empty() || width == 0 || height == 0 {
            return Err(TxdError::InvalidDimensions { width, height });
        }
        let format = self.dxt_format()?;

        let mut output = vec![0u8; (width as usize) * (height as usize) * 4];
        format.decompress(
            compressed_data,
            width as usize,
            height as usize,
            &mut output,
        );
        RgbaImage::from_raw(width, height, output)
            .ok_or(TxdError::InvalidDimensions { width, height })
    }

    pub fn compress_to_dxt(self, rgba: &RgbaImage, quality: f32) -> Result<Vec<u8>> {
        let format = self.dxt_format()?;
        let (width, height) = rgba.dimensions();
        if width == 0 || height == 0 {
            return Err(TxdError::InvalidDimensions { width, height });
        }

        let params = DxtParams {
            algorithm: if quality >= 0.5 {
                DxtAlgorithm::ClusterFit
            } else {
                DxtAlgorithm::RangeFit
            },
            ..Default::default()
        };

        let mut output = vec![0u8; format.compressed_size(width as usize, height as usize)];
        format.compress(
            rgba.as_raw(),
            width as usize,
            height as usize,
            params,
            &mut output,
        );
        Ok(output)
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameVersion {
    Gta3V1 = 0x0000_0302,
    Gta3V2 = 0x0000_0304,
    Gta3V3 = 0x0000_0310,
    Gta3V4 = 0x0800_FFFF,
    VcPs2 = 0x0C02_FFFF,
    VcPc = 0x1003_FFFF,
    Sa = 0x1803_FFFF,
    Unknown = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, BinRead, BinWrite)]
#[brw(little)]
pub struct ChunkHeader {
    pub chunk_type: u32,
    pub length: u32,
    pub version: u32,
}

impl ChunkHeader {
    pub fn read<R: Read + Seek>(stream: &mut R) -> Result<Self> {
        Ok(<Self as BinRead>::read(stream)?)
    }

    pub fn write<W: Write + Seek>(&self, stream: &mut W) -> Result<()> {
        Ok(<Self as BinWrite>::write(self, stream)?)
    }
}

pub(crate) fn write_chunk<W, F>(
    stream: &mut W,
    chunk_type: u32,
    version: u32,
    write_body: F,
) -> Result<u32>
where
    W: Write + Seek,
    F: FnOnce(&mut W) -> Result<()>,
{
    let start = stream.stream_position()?;
    ChunkHeader {
        chunk_type,
        length: 0,
        version,
    }
    .write(stream)?;

    write_body(stream)?;

    let end = stream.stream_position()?;
    stream.seek(SeekFrom::Start(start + 4))?;
    stream.write_le(&((end - start - 12) as u32))?;
    stream.seek(SeekFrom::Start(end))?;

    Ok((end - start) as u32)
}
