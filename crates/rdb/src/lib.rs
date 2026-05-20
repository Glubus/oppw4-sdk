mod address;
mod bytes;
mod catalog;
mod hash;
mod rdb;
mod scan;

pub use address::{parse_block_tail, parse_payload_tail, RdbAddressSuffix, RdbPayloadTail};
pub use catalog::{parse_name_hash_catalog, NameHashEntry};
pub use hash::parse_prefixed_hex_hash;
pub use rdb::{parse_rdb, RdbBlock, RdbError, RdbHeader, RdbIndex};
pub use scan::{
    scan_archive_names_with_catalog, scan_virtualized_names, scan_virtualized_names_with_catalog,
    ArchiveScan, ArchiveScanCounts, VirtualizedFile,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(buf: &mut [u8], offset: usize, value: u32) {
        buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn fixture() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x20];
        bytes[0..8].copy_from_slice(b"_DRK0000");
        put_u32(&mut bytes, 0x08, 0x20);
        put_u32(&mut bytes, 0x10, 2);
        bytes[0x18..0x1d].copy_from_slice(b"data/");

        let block0_offset = bytes.len();
        let mut block0 = vec![0u8; 0x44];
        block0[0..8].copy_from_slice(b"IDRK0000");
        put_u32(&mut block0, 0x08, 0x44);
        put_u32(&mut block0, 0x10, 0x0c);
        put_u32(&mut block0, 0x18, 0x100);
        put_u32(&mut block0, 0x20, 0);
        put_u32(&mut block0, 0x24, 0x0011c397);
        put_u32(&mut block0, 0x28, 0x56efe45c);
        put_u32(&mut block0, 0x2c, 0x00020000);
        block0[0x38..0x44].copy_from_slice(b"11c3971@138\0");
        bytes.extend_from_slice(&block0);

        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        assert_eq!(block0_offset, 0x20);

        let mut block1 = vec![0u8; 0x43];
        block1[0..8].copy_from_slice(b"IDRK0000");
        put_u32(&mut block1, 0x08, 0x43);
        put_u32(&mut block1, 0x10, 0x0b);
        put_u32(&mut block1, 0x18, 0xa0);
        put_u32(&mut block1, 0x20, 0);
        put_u32(&mut block1, 0x24, 0x009f7b2b);
        put_u32(&mut block1, 0x28, 0x56efe45c);
        put_u32(&mut block1, 0x2c, 0x00020000);
        block1[0x38..0x43].copy_from_slice(b"9f7b2b3@d8\0");
        bytes.extend_from_slice(&block1);
        bytes
    }

    #[test]
    fn parses_root_header() {
        let parsed = parse_rdb(&fixture()).unwrap();

        assert_eq!(parsed.header.first_block_offset, 0x20);
        assert_eq!(parsed.header.declared_count, 2);
        assert_eq!(parsed.header.data_prefix, "data/");
    }

    #[test]
    fn parses_aligned_idrk_blocks() {
        let parsed = parse_rdb(&fixture()).unwrap();

        assert_eq!(parsed.blocks.len(), 2);
        assert_eq!(parsed.blocks[0].offset, 0x20);
        assert_eq!(parsed.blocks[0].length, 0x44);
        assert_eq!(parsed.blocks[0].kind, *b"IDRK");
        assert_eq!(parsed.blocks[0].primary_hash, 0x0011c397);
        assert_eq!(&parsed.blocks[0].payload[..8], &[0; 8]);
        assert_eq!(&parsed.blocks[0].payload[8..], b"11c3971@138\0");

        assert_eq!(parsed.blocks[1].offset, 0x64);
        assert_eq!(parsed.blocks[1].length, 0x43);
        assert_eq!(parsed.blocks[1].primary_hash, 0x009f7b2b);
    }

    #[test]
    fn rejects_bad_root_magic() {
        let mut bytes = fixture();
        bytes[0] = b'X';

        assert_eq!(parse_rdb(&bytes), Err(RdbError::InvalidRootMagic));
    }

    #[test]
    fn rejects_truncated_block() {
        let mut bytes = fixture();
        bytes.truncate(0x50);

        assert_eq!(
            parse_rdb(&bytes),
            Err(RdbError::TruncatedBlock {
                offset: 0x20,
                length: 0x44
            })
        );
    }

    #[test]
    fn parses_prefixed_hex_hash_names() {
        assert_eq!(parse_prefixed_hex_hash("0x3b359352.g1m"), Some(0x3b359352));
        assert_eq!(parse_prefixed_hex_hash("0X4CE275FB.g1m"), Some(0x4ce275fb));
        assert_eq!(parse_prefixed_hex_hash("0x93dfb06c"), Some(0x93dfb06c));
        assert_eq!(parse_prefixed_hex_hash("MDLC038_Zoro_Wa.g1m"), None);
        assert_eq!(parse_prefixed_hex_hash("0x.g1m"), None);
    }

    #[test]
    fn parses_payload_tail() {
        let payload = [
            b"\0\0\0\0\xff\xff\xff\xff".as_slice(),
            b"1268d0@11c28b#8\0".as_slice(),
        ]
        .concat();

        assert_eq!(
            parse_payload_tail(&payload),
            Some(RdbPayloadTail {
                raw: "1268d0@11c28b#8".to_string(),
                part_a: 0x1268d0,
                part_b: 0x11c28b,
                suffix: Some(RdbAddressSuffix {
                    marker: '#',
                    value: 0x8,
                }),
            })
        );
    }

    #[test]
    fn parses_real_sequence_editor_index() {
        let parsed = parse_rdb(include_bytes!("../fixtures/rdb/SequenceEditor.rdb")).unwrap();

        assert_eq!(parsed.header.first_block_offset, 0x20);
        assert_eq!(parsed.header.data_prefix, "data/");
        assert!(!parsed.blocks.is_empty());
        assert_eq!(parsed.blocks.len(), parsed.header.declared_count as usize);
        assert!(parsed.blocks.iter().all(|block| block.kind == *b"IDRK"));
    }

    #[test]
    fn parses_known_real_payload_tail() {
        let parsed = parse_rdb(include_bytes!("../fixtures/rdb/CharacterEditor.rdb")).unwrap();
        let block = parsed
            .blocks
            .iter()
            .find(|block| block.primary_hash == 0x3b359352)
            .unwrap();

        assert_eq!(
            parse_payload_tail(&block.payload),
            Some(RdbPayloadTail {
                raw: "0@268005#9".to_string(),
                part_a: 0,
                part_b: 0x268005,
                suffix: Some(RdbAddressSuffix {
                    marker: '#',
                    value: 0x9,
                }),
            })
        );
        assert_eq!(parse_block_tail(block), parse_payload_tail(&block.payload));
    }

    #[test]
    fn parses_named_asset_address_forms() {
        assert_eq!(
            parse_payload_tail(b"\0\0\0\0c423de@300&0\0"),
            Some(RdbPayloadTail {
                raw: "c423de@300&0".to_string(),
                part_a: 0xc423de,
                part_b: 0x300,
                suffix: Some(RdbAddressSuffix {
                    marker: '&',
                    value: 0,
                }),
            })
        );
        assert_eq!(
            parse_payload_tail(b"\0\0\0\0b61b29a@ec\0"),
            Some(RdbPayloadTail {
                raw: "b61b29a@ec".to_string(),
                part_a: 0xb61b29a,
                part_b: 0xec,
                suffix: None,
            })
        );
    }

    #[test]
    fn scans_virtualized_hash_file_names() {
        let parsed = parse_rdb(include_bytes!("../fixtures/rdb/CharacterEditor.rdb")).unwrap();
        let scanned = scan_virtualized_names(
            &parsed,
            ["0x3b359352.g1m", "0xffffffff.g1m", "MDLC038_Zoro_Wa.g1m"],
        );

        assert_eq!(scanned.len(), 3);
        assert_eq!(scanned[0].file_name, "0x3b359352.g1m");
        assert_eq!(scanned[0].hash, Some(0x3b359352));
        assert_eq!(scanned[0].block.unwrap().primary_hash, 0x3b359352);
        assert_eq!(scanned[1].hash, Some(0xffffffff));
        assert!(scanned[1].block.is_none());
        assert_eq!(scanned[2].hash, None);
        assert!(scanned[2].block.is_none());
    }

    #[test]
    fn parses_exported_name_hash_catalog_lines() {
        let bytes = [
            b"noise,MPLC009_Ace.g1m".as_slice(),
            b"\r\n".as_slice(),
            b"0xa8bd2e49,MDLC038_Zoro_Wa.g1m".as_slice(),
            b"\r\n".as_slice(),
            b"0x1ad9ce2c,CE1_0080_STYLE_IMPACT_HAO.g1e".as_slice(),
            b"\r\n".as_slice(),
            b"0x1ad9ce2c,broken.g1m".as_slice(),
            &[0, 0],
            b"0xnothex".as_slice(),
        ]
        .concat();
        let catalog = parse_name_hash_catalog(&bytes);

        assert_eq!(
            catalog,
            vec![
                NameHashEntry {
                    name: "MDLC038_Zoro_Wa.g1m".to_string(),
                    hash: 0xa8bd2e49,
                },
                NameHashEntry {
                    name: "CE1_0080_STYLE_IMPACT_HAO.g1e".to_string(),
                    hash: 0x1ad9ce2c,
                },
            ]
        );
    }

    #[test]
    fn parses_embedded_name_hash_catalog_fallback() {
        let bytes = [
            b"MPLC009_Ace.g1m".as_slice(),
            b"\r\n".as_slice(),
            b"0xa8bd2e49".as_slice(),
            &[0, 0],
            b"MDLC038_Zoro_Wa.g1m".as_slice(),
            b"\r\n".as_slice(),
            b"0x1ad9ce2c".as_slice(),
        ]
        .concat();
        let catalog = parse_name_hash_catalog(&bytes);

        assert_eq!(
            catalog,
            vec![
                NameHashEntry {
                    name: "MPLC009_Ace.g1m".to_string(),
                    hash: 0xa8bd2e49,
                },
                NameHashEntry {
                    name: "MDLC038_Zoro_Wa.g1m".to_string(),
                    hash: 0x1ad9ce2c,
                },
            ]
        );
    }

    #[test]
    fn parses_embedded_hash_before_name_catalog_fallback() {
        let bytes = [
            &[0xff, 0],
            b"0x359b9672,800_294_face_law_dressrosa_External_00.g1t".as_slice(),
            &[0, 0],
            b"0x3bff0f13,801_294_chara_law_dressrosa_External_00.g1t".as_slice(),
            &[0, 0],
        ]
        .concat();
        let catalog = parse_name_hash_catalog(&bytes);

        assert_eq!(
            catalog,
            vec![
                NameHashEntry {
                    name: "800_294_face_law_dressrosa_External_00.g1t".to_string(),
                    hash: 0x359b9672,
                },
                NameHashEntry {
                    name: "801_294_chara_law_dressrosa_External_00.g1t".to_string(),
                    hash: 0x3bff0f13,
                },
            ]
        );
    }

    #[test]
    fn scans_virtualized_catalog_file_names() {
        let parsed = parse_rdb(include_bytes!("../fixtures/rdb/CharacterEditor.rdb")).unwrap();
        let catalog = vec![NameHashEntry {
            name: "MDLC038_Zoro_Wa.g1m".to_string(),
            hash: 0x1ad9ce2c,
        }];
        let scanned = scan_virtualized_names_with_catalog(
            &parsed,
            ["MDLC038_Zoro_Wa.g1m", "Unknown_Name.g1m"],
            &catalog,
        );

        assert_eq!(scanned[0].hash, Some(0x1ad9ce2c));
        assert_eq!(scanned[0].block.unwrap().primary_hash, 0x1ad9ce2c);
        assert_eq!(scanned[1].hash, None);
        assert!(scanned[1].block.is_none());
    }

    #[test]
    fn counts_archive_scan_results() {
        let parsed = parse_rdb(include_bytes!("../fixtures/rdb/CharacterEditor.rdb")).unwrap();
        let scan = scan_archive_names_with_catalog(
            "CharacterEditor",
            &parsed,
            ["0x3b359352.g1m", "0xffffffff.g1m", "Unknown_Name.g1m"],
            &[],
        );

        assert_eq!(scan.archive_name, "CharacterEditor");
        assert_eq!(
            scan.counts(),
            ArchiveScanCounts {
                total: 3,
                matched: 1,
                hash_missing: 1,
                unresolved_names: 1,
            }
        );
    }
}
