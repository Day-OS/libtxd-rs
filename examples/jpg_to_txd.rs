//! Converts `test_assets/apple.jpg` into a `.txd` dictionary holding the same
//! texture at several power-of-two sizes, the dimensions GTA SA expects.
//!
//! Run with: `cargo run --example jpg_to_txd`

use image::imageops::{self, FilterType};
use libtxd::dictionary::TextureDictionary;
use libtxd::encode::{TextureCreateOptions, TextureFormat};
use libtxd::texture::Texture;

const TEXTURE_SIZES: [u32; 5] = [32, 64, 128, 256, 512];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = image::open("test_assets/apple.jpg")?.into_rgba8();
    let options = TextureCreateOptions {
        format: TextureFormat::Dxt1 { quality: 1.0 },
        ..TextureCreateOptions::default()
    };

    let mut dictionary = TextureDictionary::default();
    for size in TEXTURE_SIZES {
        let rgba = imageops::resize(&source, size, size, FilterType::Lanczos3);
        let name = format!("apple_{size}");
        let texture = Texture::from_rgba(name, rgba, options)?;
        dictionary.add_texture(texture);
    }
    dictionary.save("apple.txd")?;

    println!("wrote apple.txd with {} textures", TEXTURE_SIZES.len());
    Ok(())
}
