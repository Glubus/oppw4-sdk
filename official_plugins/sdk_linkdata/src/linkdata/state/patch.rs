use plugin_sdk::linkdata::LinkDataEntrySections;

#[derive(Clone, Debug)]
pub(in crate::linkdata) enum RowPatch {
    Replace {
        section: usize,
        record_size: usize,
        row: usize,
        payload: Vec<u8>,
    },
    Insert {
        section: usize,
        record_size: usize,
        row: usize,
        payload: Vec<u8>,
    },
    Remove {
        section: usize,
        record_size: usize,
        row: usize,
    },
}

impl RowPatch {
    pub(in crate::linkdata) fn key(&self) -> RowPatchKey {
        let (section, record_size, row) = match self {
            Self::Replace {
                section,
                record_size,
                row,
                ..
            }
            | Self::Insert {
                section,
                record_size,
                row,
                ..
            }
            | Self::Remove {
                section,
                record_size,
                row,
            } => (*section, *record_size, *row),
        };
        RowPatchKey {
            section,
            record_size,
            row,
        }
    }

    pub(in crate::linkdata) fn apply(
        &self,
        sections: &mut LinkDataEntrySections,
    ) -> Result<(), String> {
        match self {
            Self::Replace {
                section,
                record_size,
                row,
                payload,
            } => sections.replace_row(*section, *record_size, *row, payload),
            Self::Insert {
                section,
                record_size,
                row,
                payload,
            } => sections.insert_row(*section, *record_size, *row, payload),
            Self::Remove {
                section,
                record_size,
                row,
            } => sections
                .remove_row(*section, *record_size, *row)
                .map(|_| ()),
        }
        .map_err(|error| error.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::linkdata) struct RowPatchKey {
    section: usize,
    record_size: usize,
    row: usize,
}
