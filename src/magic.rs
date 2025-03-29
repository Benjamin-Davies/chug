//! Detect file magic numbers

use std::{fs::File, io::Read, path::Path};

pub enum Magic {
    MachO,
    FatMachO,
    Unknown,
}

pub fn detect(path: &Path) -> anyhow::Result<Magic> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;

    match u32::from_be_bytes(magic) {
        0xCAFEBABE => Ok(Magic::FatMachO),
        0xFEEDFACE | 0xFEEDFACF | 0xCEFAEDFE | 0xCFFAEDFE => Ok(Magic::MachO),
        _ => Ok(Magic::Unknown),
    }
}
