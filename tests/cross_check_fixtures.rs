//! Cross-checks the Rust reader/converter against values captured from the
//! original C++ parser on the same real TXD assets.
use libtxd::dictionary::TextureDictionary;
use libtxd::texture::Texture;
use libtxd::types::{Compression, platform};
use std::path::PathBuf;

fn checksum(data: &[u8]) -> u32 {
    data.iter()
        .fold(0u32, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u32))
}

fn load_all_textures(path: &std::path::Path) -> (u32, Vec<Texture>) {
    let dict = TextureDictionary::from_path(path)
        .unwrap_or_else(|err| panic!("TextureDictionary failed to parse {path:?}: {err}"));
    (dict.version, dict.textures)
}

struct Expected {
    name: &'static str,
    platform: u32,
    filter_flags: u32,
    raster_format: u32,
    depth: u8,
    has_alpha: bool,
    compression: Compression,
    palette_size: u32,
    mips: &'static [(u32, u32, usize, u32)],
}

fn assert_matches(actual: &Texture, expected: &Expected) {
    assert_eq!(actual.name, expected.name, "name");
    assert_eq!(
        actual.platform, expected.platform,
        "platform ({})",
        expected.name
    );
    assert_eq!(
        actual.filter_flags, expected.filter_flags,
        "filter_flags ({})",
        expected.name
    );
    assert_eq!(
        actual.raster_format, expected.raster_format,
        "raster_format ({})",
        expected.name
    );
    assert_eq!(actual.depth, expected.depth, "depth ({})", expected.name);
    assert_eq!(
        actual.has_alpha, expected.has_alpha,
        "has_alpha ({})",
        expected.name
    );
    assert_eq!(
        actual.compression, expected.compression,
        "compression ({})",
        expected.name
    );
    assert_eq!(
        actual.palette_size, expected.palette_size,
        "palette_size ({})",
        expected.name
    );
    assert_eq!(
        actual.mipmaps.len(),
        expected.mips.len(),
        "mipmap count ({})",
        expected.name
    );

    for (i, (mip, &(w, h, size, sum))) in actual.mipmaps.iter().zip(expected.mips).enumerate() {
        assert_eq!(mip.width, w, "{} mip{i} width", expected.name);
        assert_eq!(mip.height, h, "{} mip{i} height", expected.name);
        assert_eq!(mip.data.len(), size, "{} mip{i} data len", expected.name);
        assert_eq!(
            checksum(&mip.data),
            sum,
            "{} mip{i} checksum",
            expected.name
        );
    }
}

#[test]
fn skin_txd_matches_cpp_reference_output() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_assets")
        .join("skin.txd");
    let (version, textures) = load_all_textures(&path);

    assert_eq!(version, 0x1803FFFF);
    assert_eq!(textures.len(), 5);

    let expected = [
        Expected {
            name: "cred",
            platform: platform::D3D9,
            filter_flags: 0x1106,
            raster_format: 0x8600,
            depth: 32,
            has_alpha: false,
            compression: Compression::None,
            palette_size: 0,
            mips: &[
                (512, 512, 1048576, 4221767057),
                (256, 256, 262144, 1527370737),
                (128, 128, 65536, 2697684202),
                (64, 64, 16384, 688018072),
                (32, 32, 4096, 1809197649),
                (16, 16, 1024, 893111327),
                (8, 8, 256, 2294064577),
                (4, 4, 64, 4096239539),
                (2, 2, 16, 3819747484),
                (1, 1, 4, 1654532),
            ],
        },
        Expected {
            name: "headmommy",
            platform: platform::D3D9,
            filter_flags: 0x1106,
            raster_format: 0x8500,
            depth: 32,
            has_alpha: false,
            compression: Compression::None,
            palette_size: 0,
            mips: &[
                (512, 512, 1048576, 3301852513),
                (256, 256, 262144, 2096509680),
                (128, 128, 65536, 3172588345),
                (64, 64, 16384, 942433175),
                (32, 32, 4096, 4159270867),
                (16, 16, 1024, 1017785631),
                (8, 8, 256, 2312529488),
                (4, 4, 64, 3010165836),
                (2, 2, 16, 3154641779),
                (1, 1, 4, 1580573),
            ],
        },
        Expected {
            name: "Tex_0008",
            platform: platform::D3D9,
            filter_flags: 0x1106,
            raster_format: 0x8600,
            depth: 32,
            has_alpha: false,
            compression: Compression::None,
            palette_size: 0,
            mips: &[
                (128, 256, 131072, 1090424199),
                (64, 128, 32768, 1100794084),
                (32, 64, 8192, 590844497),
                (16, 32, 2048, 20907439),
                (8, 16, 512, 1874020589),
                (4, 8, 128, 3251284602),
                (2, 4, 32, 2291823776),
                (1, 2, 8, 1267245896),
                (1, 1, 4, 2875467),
            ],
        },
        Expected {
            name: "body",
            platform: platform::D3D9,
            filter_flags: 0x1106,
            raster_format: 0x8500,
            depth: 32,
            has_alpha: false,
            compression: Compression::None,
            palette_size: 0,
            mips: &[
                (1024, 2048, 8388608, 1794907743),
                (512, 1024, 2097152, 3847573903),
                (256, 512, 524288, 1297081738),
                (128, 256, 131072, 3684043026),
                (64, 128, 32768, 646881684),
                (32, 64, 8192, 613595665),
                (16, 32, 2048, 995089924),
                (8, 16, 512, 1923963775),
                (4, 8, 128, 103287669),
                (2, 4, 32, 2771070956),
                (1, 2, 8, 2511981939),
                (1, 1, 4, 5072537),
            ],
        },
        Expected {
            name: "body2",
            platform: platform::D3D9,
            filter_flags: 0x1106,
            raster_format: 0x8600,
            depth: 32,
            has_alpha: false,
            compression: Compression::None,
            palette_size: 0,
            mips: &[
                (400, 800, 1280000, 2819083737),
                (200, 400, 320000, 377106709),
                (100, 200, 80000, 1203371538),
                (50, 100, 20000, 1839211229),
                (25, 50, 5000, 4117774921),
                (12, 25, 1200, 542048258),
                (6, 12, 288, 2012409166),
                (3, 6, 72, 1834287957),
                (1, 3, 12, 3045445578),
                (1, 1, 4, 4412385),
            ],
        },
    ];

    for (actual, expected) in textures.iter().zip(expected.iter()) {
        assert_matches(actual, expected);
    }
}

#[test]
fn converter_matches_cpp_reference_output() {
    let skin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_assets")
        .join("skin.txd");
    let (_, skin_textures) = load_all_textures(&skin_path);

    let expected_rgba_checksums = [
        ("cred", 3880515793u32),
        ("headmommy", 2133579105),
        ("Tex_0008", 3753449110),
        ("body", 376023967),
        ("body2", 3522762073),
    ];

    for (texture, (name, expected_checksum)) in skin_textures.iter().zip(expected_rgba_checksums) {
        assert_eq!(texture.name, name);
        let rgba = texture.to_rgba8(0).unwrap();
        assert_eq!(
            checksum(rgba.as_raw()),
            expected_checksum,
            "{name} rgba8 checksum"
        );
    }
}
