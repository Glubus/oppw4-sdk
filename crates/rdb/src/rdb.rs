use crate::bytes::{align4, read_c_string, read_u32};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbHeader {
    pub first_block_offset: u32,
    pub declared_count: u32,
    pub data_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbBlock {
    pub offset: usize,
    pub length: u32,
    pub kind: [u8; 4],
    pub field_10: u32,
    pub data_offset: u32,
    pub field_20: u32,
    pub primary_hash: u32,
    pub field_28: u32,
    pub field_2c: u32,
    pub payload: Vec<u8>,
    pub raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbIndex {
    pub header: RdbHeader,
    pub blocks: Vec<RdbBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdbError {
    TooSmall,
    InvalidRootMagic,
    InvalidBlockMagic { offset: usize },
    InvalidBlockLength { offset: usize, length: u32 },
    TruncatedBlock { offset: usize, length: u32 },
}

pub fn parse_rdb(bytes: &[u8]) -> Result<RdbIndex, RdbError> {
    validate_root(bytes)?;
    let header = parse_header(bytes)?;
    let blocks = parse_blocks(bytes, header.first_block_offset as usize)?;

    Ok(RdbIndex { header, blocks })
}

fn validate_root(bytes: &[u8]) -> Result<(), RdbError> {
    if bytes.len() < 0x20 {
        return Err(RdbError::TooSmall);
    }

    if &bytes[0..8] != b"_DRK0000" {
        return Err(RdbError::InvalidRootMagic);
    }

    Ok(())
}

fn parse_header(bytes: &[u8]) -> Result<RdbHeader, RdbError> {
    Ok(RdbHeader {
        first_block_offset: read_u32(bytes, 0x08)?,
        declared_count: read_u32(bytes, 0x10)?,
        data_prefix: read_c_string(&bytes[0x18..0x20]),
    })
}

fn parse_blocks(bytes: &[u8], first_offset: usize) -> Result<Vec<RdbBlock>, RdbError> {
    let mut blocks = Vec::new();
    let mut offset = first_offset;

    while offset < bytes.len() {
        let block = parse_block(bytes, offset)?;
        offset = align4(offset + block.length as usize);
        blocks.push(block);
    }

    Ok(blocks)
}

fn parse_block(bytes: &[u8], offset: usize) -> Result<RdbBlock, RdbError> {
    if bytes.len() - offset < 0x30 {
        return Err(RdbError::TruncatedBlock {
            offset,
            length: 0x30,
        });
    }

    if &bytes[offset..offset + 4] != b"IDRK" {
        return Err(RdbError::InvalidBlockMagic { offset });
    }

    let length = read_u32(bytes, offset + 0x08)?;
    if length < 0x30 {
        return Err(RdbError::InvalidBlockLength { offset, length });
    }

    let end = offset + length as usize;
    if end > bytes.len() {
        return Err(RdbError::TruncatedBlock { offset, length });
    }

    Ok(RdbBlock {
        offset,
        length,
        kind: bytes[offset..offset + 4].try_into().unwrap(),
        field_10: read_u32(bytes, offset + 0x10)?,
        data_offset: read_u32(bytes, offset + 0x18)?,
        field_20: read_u32(bytes, offset + 0x20)?,
        primary_hash: read_u32(bytes, offset + 0x24)?,
        field_28: read_u32(bytes, offset + 0x28)?,
        field_2c: read_u32(bytes, offset + 0x2c)?,
        payload: bytes[offset + 0x30..end].to_vec(),
        raw: bytes[offset..end].to_vec(),
    })
}
