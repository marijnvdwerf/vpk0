//! Byte-identical round-trip tests for Pokemon Snap VPK0 segments.
//!
//! Reads the three VPK0-compressed segments from the Pokemon Snap ROM,
//! decompresses them, re-compresses using the Snap backend, and asserts
//! the output matches the original ROM bytes exactly.

use sha1::{Digest, Sha1};
use std::path::PathBuf;
use vpk0::LzssBackend;

const ROM_SHA1: &str = "edc7c49cc568c045fe48be0d18011c30f393cbaf";

struct Segment {
    name: &'static str,
    start: usize,
    end: usize,
}

const SEGMENTS: &[Segment] = &[
    Segment {
        name: "menu_images_A0F830",
        start: 0xA0F830,
        end: 0xA5CC46,
    },
    Segment {
        name: "rights_notice_AA0B80",
        start: 0xAA0B80,
        end: 0xAA18D3,
    },
    Segment {
        name: "tiny_AAA610",
        start: 0xAAA610,
        end: 0xAAA65B,
    },
];

fn find_rom() -> Option<PathBuf> {
    // Look in the sibling decomp repo first, then current dir
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("_ethanrepo/pokemonsnap.z64"),
        PathBuf::from("pokemonsnap.z64"),
    ];
    candidates.iter().find(|p| p.exists()).cloned()
}

fn load_rom() -> Vec<u8> {
    let path = find_rom().expect(
        "Pokemon Snap ROM not found. Place pokemonsnap.z64 next to the test or in ../_ethanrepo/",
    );
    let data = std::fs::read(&path).expect("failed to read ROM");
    let digest = Sha1::digest(&data);
    let hash = digest.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    assert_eq!(hash, ROM_SHA1, "ROM sha1 mismatch");
    data
}

#[test]
fn snap_recompress_bit_matches_rom() {
    let rom = load_rom();

    for seg in SEGMENTS {
        let blob = &rom[seg.start..seg.end];

        // Decompress original
        let decompressed = vpk0::decode_bytes(blob)
            .unwrap_or_else(|e| panic!("{}: decode failed: {}", seg.name, e));

        // Re-compress with Snap backend
        let recompressed = vpk0::Encoder::for_bytes(&decompressed)
            .two_sample()
            .lzss_backend(LzssBackend::Snap)
            .encode_to_vec()
            .unwrap_or_else(|e| panic!("{}: encode failed: {}", seg.name, e));

        assert_eq!(
            recompressed.len(),
            blob.len(),
            "{}: size mismatch (got {} expected {})",
            seg.name,
            recompressed.len(),
            blob.len()
        );
        assert_eq!(
            &recompressed[..],
            blob,
            "{}: byte mismatch at first differing position",
            seg.name,
        );
    }
}
