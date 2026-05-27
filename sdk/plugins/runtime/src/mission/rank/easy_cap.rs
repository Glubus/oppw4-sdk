use std::sync::OnceLock;

use plugin_sdk::OwnedHostApi;

use crate::runtime::probe::PLUGIN_ID;

const SCORE_RANK_CAP_SIGNATURE: &[u8] = &[
    0x39, 0x74, 0x24, 0x38, 0x75, 0x21, 0x41, 0x83, 0xfe, 0x04, 0x75, 0x1b, 0xc7, 0x87, 0xa4, 0x02,
    0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xc7, 0x87, 0x28, 0x03, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
];
const SCORE_RANK_CAP_MASK: &[u8] = &[1; SCORE_RANK_CAP_SIGNATURE.len()];
const SCORE_RANK_CAP_JUMP_OFFSET: usize = 4;

const FINAL_RANK_CAP_SIGNATURE: &[u8] = &[
    0x75, 0x52, 0x83, 0x7b, 0x10, 0x05, 0x75, 0x0e, 0xc7, 0x43, 0x10, 0x03, 0x00, 0x00, 0x00, 0xc7,
    0x43, 0x38, 0x01, 0x00, 0x00, 0x00, 0x83, 0x7b, 0x0c, 0x05, 0x75, 0x0e, 0xc7, 0x43, 0x0c, 0x03,
    0x00, 0x00, 0x00,
];
const FINAL_RANK_CAP_MASK: &[u8] = &[1; FINAL_RANK_CAP_SIGNATURE.len()];
const FINAL_RANK_CAP_JUMP_OFFSET: usize = 0;

const GLOBAL_RANK_PRIMARY_CAP_SIGNATURE: &[u8] = &[
    0x83, 0x7c, 0x24, 0x28, 0x00, 0xb8, 0x03, 0x00, 0x00, 0x00, 0x75, 0x16, 0x83, 0xfe, 0x05, 0x0f,
    0x44, 0xf0, 0x83, 0xfd, 0x05, 0x0f, 0x44, 0xe8, 0x3b, 0xf1, 0x0f, 0x44, 0xf0, 0x3b, 0xe9, 0x0f,
    0x44, 0xe8,
];
const GLOBAL_RANK_PRIMARY_CAP_MASK: &[u8] = &[1; GLOBAL_RANK_PRIMARY_CAP_SIGNATURE.len()];
const GLOBAL_RANK_PRIMARY_CAP_JUMP_OFFSET: usize = 10;

const GLOBAL_RANK_MODE4_CAP_SIGNATURE: &[u8] = &[
    0x83, 0x7c, 0x24, 0x28, 0x00, 0x75, 0x0a, 0x3b, 0xc1, 0xbb, 0x03, 0x00, 0x00, 0x00, 0x0f, 0x44,
    0xc3,
];
const GLOBAL_RANK_MODE4_CAP_MASK: &[u8] = &[1; GLOBAL_RANK_MODE4_CAP_SIGNATURE.len()];
const GLOBAL_RANK_MODE4_CAP_JUMP_OFFSET: usize = 5;

const GLOBAL_RANK_MODE5_CAP_SIGNATURE: &[u8] = &[
    0x83, 0x7c, 0x24, 0x28, 0x00, 0x75, 0x14, 0x3b, 0xd9, 0x75, 0x10, 0x8d, 0x59, 0xff,
];
const GLOBAL_RANK_MODE5_CAP_MASK: &[u8] = &[1; GLOBAL_RANK_MODE5_CAP_SIGNATURE.len()];
const GLOBAL_RANK_MODE5_CAP_JUMP_OFFSET: usize = 5;

const ORIGINAL_JNZ: u8 = 0x75;
const PATCHED_JMP: u8 = 0xeb;

static PATCH_SITES: OnceLock<EasyRankCapSites> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EasyRankCapSites {
    final_rank: usize,
    score_rank: usize,
    global_rank_primary: usize,
    global_rank_mode4: usize,
    global_rank_mode5: usize,
}

pub(crate) fn set_easy_s_rankable(host: &OwnedHostApi, enabled: bool) {
    if !enabled || PATCH_SITES.get().is_some() {
        return;
    }

    match patch_easy_rank_cap(host) {
        Ok(sites) => {
            let _ = PATCH_SITES.set(sites);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_runtime set_easy_s_rankable installed final_rank_site=0x{:x} score_rank_site=0x{:x} global_rank_primary_site=0x{:x} global_rank_mode4_site=0x{:x} global_rank_mode5_site=0x{:x}",
                    sites.final_rank,
                    sites.score_rank,
                    sites.global_rank_primary,
                    sites.global_rank_mode4,
                    sites.global_rank_mode5
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_runtime set_easy_s_rankable failed: {error}"),
            );
        }
    }
}

fn patch_easy_rank_cap(host: &OwnedHostApi) -> Result<EasyRankCapSites, String> {
    Ok(EasyRankCapSites {
        final_rank: patch_jump(
            host,
            FINAL_RANK_CAP_SIGNATURE,
            FINAL_RANK_CAP_MASK,
            FINAL_RANK_CAP_JUMP_OFFSET,
            "final rank cap",
        )?,
        score_rank: patch_jump(
            host,
            SCORE_RANK_CAP_SIGNATURE,
            SCORE_RANK_CAP_MASK,
            SCORE_RANK_CAP_JUMP_OFFSET,
            "score rank cap",
        )?,
        global_rank_primary: patch_jump(
            host,
            GLOBAL_RANK_PRIMARY_CAP_SIGNATURE,
            GLOBAL_RANK_PRIMARY_CAP_MASK,
            GLOBAL_RANK_PRIMARY_CAP_JUMP_OFFSET,
            "global rank primary cap",
        )?,
        global_rank_mode4: patch_jump(
            host,
            GLOBAL_RANK_MODE4_CAP_SIGNATURE,
            GLOBAL_RANK_MODE4_CAP_MASK,
            GLOBAL_RANK_MODE4_CAP_JUMP_OFFSET,
            "global rank mode4 cap",
        )?,
        global_rank_mode5: patch_jump(
            host,
            GLOBAL_RANK_MODE5_CAP_SIGNATURE,
            GLOBAL_RANK_MODE5_CAP_MASK,
            GLOBAL_RANK_MODE5_CAP_JUMP_OFFSET,
            "global rank mode5 cap",
        )?,
    })
}

fn patch_jump(
    host: &OwnedHostApi,
    signature: &[u8],
    mask: &[u8],
    jump_offset: usize,
    label: &str,
) -> Result<usize, String> {
    let site = host
        .memory()
        .scan(signature, mask)
        .map_err(|error| format!("{label} scan failed: {error}"))?;
    if site == 0 {
        return Err(format!("{label} signature not found"));
    }

    let jump_address = site + jump_offset;
    let mut current = [0u8; 1];
    host.memory()
        .read(jump_address, &mut current)
        .map_err(|error| format!("{label} read failed address=0x{jump_address:x}: {error}"))?;
    if current[0] != ORIGINAL_JNZ && current[0] != PATCHED_JMP {
        return Err(format!(
            "{label} unexpected jump byte address=0x{jump_address:x} value=0x{:02x}",
            current[0]
        ));
    }
    if current[0] == PATCHED_JMP {
        return Ok(site);
    }

    host.memory()
        .write(jump_address, &[PATCHED_JMP])
        .map_err(|error| format!("{label} write failed address=0x{jump_address:x}: {error}"))?;
    Ok(site)
}
