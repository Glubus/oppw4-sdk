use super::super::types::LinkDataEntryId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkDataEntry {
    pub id: LinkDataEntryId,
    pub table_offset: usize,
    pub data_offset: usize,
    pub field_04: u32,
    pub compressed_span: usize,
    pub uncompressed_size: usize,
}
