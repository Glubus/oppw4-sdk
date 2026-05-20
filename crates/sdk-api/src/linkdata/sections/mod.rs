mod error;
mod parse;
mod rows;

use super::types::{align_up, write_u32};

pub use error::LinkDataSectionError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkDataEntrySections {
    sections: Vec<Vec<u8>>,
}

impl LinkDataEntrySections {
    pub fn parse(payload: &[u8]) -> Result<Self, LinkDataSectionError> {
        parse::parse_sections(payload)
    }

    pub fn new(section_count: usize) -> Self {
        Self {
            sections: vec![Vec::new(); section_count],
        }
    }

    pub(super) fn from_sections(sections: Vec<Vec<u8>>) -> Self {
        Self { sections }
    }

    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn section(&self, section: usize) -> Option<&[u8]> {
        self.sections.get(section).map(Vec::as_slice)
    }

    pub fn section_mut(&mut self, section: usize) -> Option<&mut Vec<u8>> {
        self.sections.get_mut(section)
    }

    pub fn row_count(
        &self,
        section: usize,
        record_size: usize,
    ) -> Result<usize, LinkDataSectionError> {
        rows::row_count(&self.sections, section, record_size)
    }

    pub fn replace_row(
        &mut self,
        section: usize,
        record_size: usize,
        row: usize,
        bytes: &[u8],
    ) -> Result<(), LinkDataSectionError> {
        rows::replace_row(&mut self.sections, section, record_size, row, bytes)
    }

    pub fn insert_row(
        &mut self,
        section: usize,
        record_size: usize,
        row: usize,
        bytes: &[u8],
    ) -> Result<(), LinkDataSectionError> {
        rows::insert_row(&mut self.sections, section, record_size, row, bytes)
    }

    pub fn remove_row(
        &mut self,
        section: usize,
        record_size: usize,
        row: usize,
    ) -> Result<Vec<u8>, LinkDataSectionError> {
        rows::remove_row(&mut self.sections, section, record_size, row)
    }

    pub fn rebuild(&self) -> Vec<u8> {
        let section_count = self.sections.len();
        let header_len = align_up(4 + section_count * 4 + 4, 0x10);
        let mut output = vec![0u8; header_len];
        write_u32(&mut output, 0, section_count as u32);
        write_section_payloads(&mut output, &self.sections, header_len);
        output
    }
}

fn write_section_payloads(output: &mut Vec<u8>, sections: &[Vec<u8>], mut cursor: usize) {
    for (index, bytes) in sections.iter().enumerate() {
        write_u32(output, 4 + index * 4, cursor as u32);
        output.extend_from_slice(bytes);
        cursor += bytes.len();
        while !output.len().is_multiple_of(0x10) {
            output.push(0);
            cursor += 1;
        }
    }
}
