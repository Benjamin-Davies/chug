//! Detect file magic numbers

use anyhow::Context;

pub enum Magic {
    MachO,
    FatMachO,
    Elf,
    Unknown,
}

pub fn detect(contents: &[u8]) -> anyhow::Result<Magic> {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(
        contents
            .get(..4)
            .context("File too short to detect magic number")?,
    );

    match u32::from_be_bytes(magic) {
        0xCAFEBABE => Ok(Magic::FatMachO),
        0xFEEDFACE | 0xFEEDFACF | 0xCEFAEDFE | 0xCFFAEDFE => Ok(Magic::MachO),
        0x7F454C46 => Ok(Magic::Elf),
        _ => Ok(Magic::Unknown),
    }
}
