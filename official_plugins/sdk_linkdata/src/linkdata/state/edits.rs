use std::collections::BTreeMap;

use plugin_sdk::linkdata::{LinkDataArchive, LinkDataEntryId, LinkDataEntrySections};

use super::patch::{RowPatch, RowPatchKey};

#[derive(Default)]
pub(super) struct LinkDataEdits {
    entry_edits: BTreeMap<LinkDataEntryId, EntryPatch>,
    row_edits: BTreeMap<LinkDataEntryId, Vec<RowEdit>>,
    row_claims: BTreeMap<(LinkDataEntryId, RowPatchKey), String>,
}

impl LinkDataEdits {
    pub(super) fn has_patches(&self) -> bool {
        !self.entry_edits.is_empty() || !self.row_edits.is_empty()
    }

    pub(super) fn patch_count(&self) -> usize {
        self.entry_edits.len() + self.row_edits.values().map(Vec::len).sum::<usize>()
    }

    pub(super) fn replace_entry(
        &mut self,
        plugin_id: &str,
        entry: LinkDataEntryId,
        payload: Vec<u8>,
    ) -> Result<(), String> {
        self.ensure_entry_owner(plugin_id, entry)?;
        self.ensure_row_owners(plugin_id, entry)?;
        self.entry_edits.insert(
            entry,
            EntryPatch {
                plugin_id: plugin_id.to_string(),
                payload,
            },
        );
        Ok(())
    }

    pub(super) fn patch_row(
        &mut self,
        plugin_id: &str,
        entry: LinkDataEntryId,
        patch: RowPatch,
    ) -> Result<(), String> {
        self.ensure_entry_owner(plugin_id, entry)?;
        self.claim_row(plugin_id, entry, patch.key())?;
        self.row_edits.entry(entry).or_default().push(RowEdit {
            plugin_id: plugin_id.to_string(),
            patch,
        });
        Ok(())
    }

    pub(super) fn entry_payloads(
        &self,
        archive: &LinkDataArchive,
    ) -> Result<BTreeMap<LinkDataEntryId, Vec<u8>>, String> {
        let mut edits = self.replaced_entry_payloads();
        for (entry, row_edits) in &self.row_edits {
            let payload = self.payload_for_row_edits(archive, &mut edits, *entry)?;
            edits.insert(*entry, apply_row_edits(payload, row_edits)?);
        }
        Ok(edits)
    }

    fn ensure_entry_owner(&self, plugin_id: &str, entry: LinkDataEntryId) -> Result<(), String> {
        if let Some(existing) = self.entry_edits.get(&entry) {
            if existing.plugin_id != plugin_id {
                return Err(existing.plugin_id.clone());
            }
        }
        Ok(())
    }

    fn ensure_row_owners(&self, plugin_id: &str, entry: LinkDataEntryId) -> Result<(), String> {
        if let Some(owner) = self.row_edits.get(&entry).and_then(|edits| {
            edits
                .iter()
                .find_map(|edit| (edit.plugin_id != plugin_id).then(|| edit.plugin_id.clone()))
        }) {
            return Err(owner);
        }
        Ok(())
    }

    fn claim_row(
        &mut self,
        plugin_id: &str,
        entry: LinkDataEntryId,
        key: RowPatchKey,
    ) -> Result<(), String> {
        let claim_key = (entry, key);
        if let Some(owner) = self.row_claims.get(&claim_key) {
            if owner != plugin_id {
                return Err(owner.clone());
            }
        }
        self.row_claims.insert(claim_key, plugin_id.to_string());
        Ok(())
    }

    fn replaced_entry_payloads(&self) -> BTreeMap<LinkDataEntryId, Vec<u8>> {
        self.entry_edits
            .iter()
            .map(|(entry, patch)| (*entry, patch.payload.clone()))
            .collect()
    }

    fn payload_for_row_edits(
        &self,
        archive: &LinkDataArchive,
        edits: &mut BTreeMap<LinkDataEntryId, Vec<u8>>,
        entry: LinkDataEntryId,
    ) -> Result<Vec<u8>, String> {
        match edits.remove(&entry) {
            Some(payload) => Ok(payload),
            None => archive
                .entry_payload(entry)
                .map_err(|error| error.to_string()),
        }
    }
}

struct EntryPatch {
    plugin_id: String,
    payload: Vec<u8>,
}

struct RowEdit {
    plugin_id: String,
    patch: RowPatch,
}

fn apply_row_edits(payload: Vec<u8>, edits: &[RowEdit]) -> Result<Vec<u8>, String> {
    let mut sections = LinkDataEntrySections::parse(&payload).map_err(|error| error.to_string())?;
    for edit in edits {
        edit.patch.apply(&mut sections)?;
    }
    Ok(sections.rebuild())
}

#[cfg(test)]
#[path = "edits_tests.rs"]
mod tests;
