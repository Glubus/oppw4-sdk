use super::snapshot::ResultStateSnapshot;

pub(super) fn calculate_hash(snapshot: &ResultStateSnapshot) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    hash_u32(&mut hash, snapshot.state);
    hash_u32(&mut hash, snapshot.unlock_flags);
    hash_u32(&mut hash, snapshot.mission_id);
    hash_blocks(&mut hash, snapshot);
    hash_u32(&mut hash, snapshot.soul_counter);
    hash_u32(&mut hash, snapshot.crew_unlock_count);
    hash_pairs(&mut hash, &snapshot.crew_unlocks);
    hash_events(&mut hash, &snapshot.events);
    hash
}

fn hash_blocks(hash: &mut u64, snapshot: &ResultStateSnapshot) {
    for value in snapshot
        .difficulty_or_rank
        .iter()
        .chain(snapshot.result_score.iter())
        .chain(snapshot.result_reward.iter())
        .chain(snapshot.result_copy.iter())
        .chain(snapshot.crew_points.iter())
        .chain(snapshot.source_rewards.iter())
        .chain(snapshot.character_rewards.iter())
        .chain(snapshot.character_totals.iter())
    {
        hash_u32(hash, *value);
    }
}

fn hash_pairs(hash: &mut u64, values: &[[u32; 2]]) {
    for pair in values {
        hash_u32(hash, pair[0]);
        hash_u32(hash, pair[1]);
    }
}

fn hash_events(hash: &mut u64, values: &[[u32; 5]]) {
    for event in values {
        for value in event {
            hash_u32(hash, *value);
        }
    }
}

fn hash_u32(hash: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}
