//! End-to-end TXD round-trip against a real multi-texture asset.

use libtxd::dictionary::TextureDictionary;
use std::path::PathBuf;

#[test]
fn skin_txd_round_trips_through_a_real_export() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_assets")
        .join("skin.txd");

    let original = TextureDictionary::from_path(&source_path)
        .unwrap_or_else(|err| panic!("failed to load {source_path:?}: {err}"));
    assert_eq!(original.textures.len(), 5);

    let export_path = std::env::temp_dir().join("libtxd_e2e_skin_export.txd");
    original
        .save(&export_path)
        .unwrap_or_else(|err| panic!("failed to save {export_path:?}: {err}"));

    let reexported = TextureDictionary::from_path(&export_path)
        .unwrap_or_else(|err| panic!("failed to reload exported file {export_path:?}: {err}"));
    std::fs::remove_file(&export_path).unwrap();

    assert_eq!(reexported.version, original.version);
    assert_eq!(reexported.game_version, original.game_version);
    assert_eq!(reexported.textures.len(), original.textures.len());

    for (original_tex, reexported_tex) in original.textures.iter().zip(&reexported.textures) {
        assert_eq!(reexported_tex.name, original_tex.name);
        assert_eq!(
            reexported_tex.mipmaps.len(),
            original_tex.mipmaps.len(),
            "{} mipmap count",
            original_tex.name
        );
        for (i, (om, rm)) in original_tex
            .mipmaps
            .iter()
            .zip(&reexported_tex.mipmaps)
            .enumerate()
        {
            assert_eq!(rm.width, om.width, "{} mip{i} width", original_tex.name);
            assert_eq!(rm.height, om.height, "{} mip{i} height", original_tex.name);
            assert_eq!(rm.data, om.data, "{} mip{i} pixel data", original_tex.name);
        }
        assert_eq!(
            reexported_tex, original_tex,
            "{} differs after round-trip",
            original_tex.name
        );
    }

    assert_eq!(reexported, original);
}
