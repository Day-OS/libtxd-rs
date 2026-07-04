//! Texture dictionary container and TXD read/write support.

use crate::error::{Result, TxdError};
use crate::texture::Texture;
use crate::types::{ChunkHeader, GameVersion, chunk_type, platform, write_chunk};
use binrw::BinWriterExt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct TextureDictionary {
    pub textures: Vec<Texture>,
    pub version: u32,
    pub game_version: GameVersion,
}

impl Default for TextureDictionary {
    fn default() -> Self {
        Self {
            textures: Vec::new(),
            version: GameVersion::Sa as u32,
            game_version: GameVersion::Sa,
        }
    }
}

impl TextureDictionary {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        Self::from_reader(&mut file)
    }

    pub fn from_reader<R: Read + Seek>(stream: &mut R) -> Result<Self> {
        let header = ChunkHeader::read(stream)?;
        if header.chunk_type != chunk_type::TEX_DICTIONARY {
            return Err(TxdError::NotTextureDictionary);
        }

        let mut dictionary = Self {
            version: header.version,
            ..Self::default()
        };
        dictionary.read_body(stream, header.length)?;
        Ok(dictionary)
    }

    pub fn find_texture(&self, name: &str) -> Option<&Texture> {
        self.textures
            .iter()
            .rfind(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn find_texture_mut(&mut self, name: &str) -> Option<&mut Texture> {
        self.textures
            .iter_mut()
            .rfind(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn add_texture(&mut self, texture: Texture) {
        self.textures.push(texture);
    }

    pub fn remove_texture(&mut self, index: usize) -> Option<Texture> {
        (index < self.textures.len()).then(|| self.textures.remove(index))
    }

    pub fn remove_texture_by_name(&mut self, name: &str) -> Option<Texture> {
        self.textures
            .iter()
            .rposition(|t| t.name.eq_ignore_ascii_case(name))
            .map(|pos| self.textures.remove(pos))
    }

    pub fn set_version(&mut self, version: u32) {
        self.version = version;
        self.game_version = Self::detect_game_version(version, &self.textures);
    }

    fn read_body<R: Read + Seek>(&mut self, stream: &mut R, length: u32) -> Result<()> {
        let section_end = stream.stream_position()? + length as u64;

        while stream.stream_position()? < section_end {
            let child_header = match ChunkHeader::read(stream) {
                Ok(h) => h,
                Err(_) => break,
            };
            let child_start = stream.stream_position()?;
            let child_end = child_start + child_header.length as u64;

            if child_header.chunk_type == chunk_type::TEXTURE_NATIVE {
                // The texture reader consumes the TEXTURENATIVE header itself.
                stream.seek(SeekFrom::Start(child_start - 12))?;
                if let Some(texture) = Texture::from_d3d_reader(stream)? {
                    self.textures.push(texture);
                }
            }
            stream.seek(SeekFrom::Start(child_end))?;
        }

        self.game_version = Self::detect_game_version(self.version, &self.textures);
        Ok(())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut file = File::create(path)?;
        self.save_to(&mut file)
    }

    pub fn save_to<W: Write + Seek>(&self, stream: &mut W) -> Result<()> {
        write_chunk(stream, chunk_type::TEX_DICTIONARY, self.version, |stream| {
            ChunkHeader {
                chunk_type: chunk_type::STRUCT,
                length: 4,
                version: self.version,
            }
            .write(stream)?;

            stream.write_le(&(self.textures.len() as u16))?;
            stream.write_le(&0u16)?;

            for texture in &self.textures {
                texture.write_d3d(stream, self.version)?;
            }

            ChunkHeader {
                chunk_type: chunk_type::EXTENSION,
                length: 0,
                version: self.version,
            }
            .write(stream)
        })?;
        Ok(())
    }

    fn detect_game_version(version_value: u32, textures: &[Texture]) -> GameVersion {
        if version_value == GameVersion::Gta3V1 as u32
            || version_value == GameVersion::Gta3V2 as u32
            || version_value == GameVersion::Gta3V3 as u32
        {
            return GameVersion::Gta3V1;
        } else if version_value == GameVersion::Gta3V4 as u32 {
            return GameVersion::Gta3V4;
        } else if version_value == GameVersion::VcPs2 as u32 {
            let has_d3d = textures
                .iter()
                .any(|t| t.platform == platform::D3D8 || t.platform == platform::D3D9);
            let has_ps2 = textures
                .iter()
                .any(|t| t.platform == platform::PS2 || t.platform == platform::PS2_FOURCC);

            return if has_d3d && !has_ps2 {
                GameVersion::Gta3V4
            } else if has_ps2 && !has_d3d {
                GameVersion::VcPs2
            } else {
                GameVersion::Gta3V4
            };
        }

        let lower16 = (version_value & 0xFFFF) as u16;
        let upper16 = ((version_value >> 16) & 0xFFFF) as u16;

        if upper16 == 0x0C02 && lower16 == 0xFFFF {
            GameVersion::VcPs2
        } else if upper16 == 0x1003 && lower16 == 0xFFFF {
            GameVersion::VcPc
        } else if upper16 == 0x1803 && lower16 == 0xFFFF {
            GameVersion::Sa
        } else {
            GameVersion::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::MipmapLevel;

    fn sample_texture(name: &str) -> Texture {
        let mut tex = Texture {
            name: name.to_string(),
            ..Default::default()
        };
        tex.mipmaps.push(MipmapLevel {
            width: 4,
            height: 4,
            data: vec![0x11; 4 * 4 * 4],
        });
        tex
    }

    #[test]
    fn duplicate_name_lookup_and_removal_use_latest_match() {
        let mut dict = TextureDictionary::default();
        dict.add_texture(sample_texture("same"));

        let mut replacement = sample_texture("SAME");
        replacement.filter_flags = 7;
        dict.add_texture(replacement);

        assert_eq!(dict.find_texture("same").unwrap().filter_flags, 7);
        assert!(dict.remove_texture_by_name("same").is_some());
        assert_eq!(dict.textures.len(), 1);
        assert_eq!(dict.textures[0].filter_flags, 0);
    }
}
