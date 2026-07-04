# libtxd

A Rust port of the TXD (RenderWare texture dictionary) parsing, writing, and
conversion logic from [txdedit](https://github.com/vaibhavpandeyvpz/txdedit).

## What it does

- Reads and writes D3D8/D3D9 `.txd` texture dictionaries (`TextureDictionary`).
- Converts textures to and from `RgbaImage` (`Texture::from_rgba`, `Texture::to_rgba8`).
- Encodes RGBA32 (raw) and DXT1/DXT3 (via `texpresso`) mipmap chains, with
  automatic mipmap generation.
- Decodes legacy packed pixel formats (R5G6B5, A1R5G5B5, R4G4B4A4, LUM8, ...)
  and PAL4/PAL8 indexed-palette textures (palette quantization via `imagequant`).
- Detects the source game version (GTA III, Vice City, San Andreas) from a
  dictionary's chunk version.

## Layout

| Module | Responsibility |
| --- | --- |
| `texture` | `Texture`/`MipmapLevel` model, D3D8/D3D9 binary read/write, RGBA conversion |
| `dictionary` | `TextureDictionary` container: load/save, lookup, add/remove |
| `encode` | `TextureFormat`/`TextureCreateOptions` for building new textures |
| `types` | Shared wire constants, `Compression`, `GameVersion`, chunk I/O helpers |
| `palette` | Indexed-color palette generation/decoding (`imagequant`) |
| `error` | `TxdError` |

## Testing

`tests/cross_check_fixtures.rs` decodes real `.txd` fixtures and checks the
output against known-good checksums, and `tests/e2e_roundtrip.rs` verifies a
full read → convert → write round trip.
