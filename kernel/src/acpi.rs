pub mod mapper;
pub mod pci;

use acpi::AcpiTables;
use anyhow::{anyhow, Result};

use crate::println;
use mapper::AcpiMapper;
use pci::PCI;

pub struct ACPI {
    tables: AcpiTables<AcpiMapper>
}

impl ACPI {
    pub fn parse(addr: u64) -> Result<ACPI> {
        println!("Parsing ACPI Tables");

        let mapper = AcpiMapper::new();
        let acpi = unsafe { AcpiTables::from_rsdp(mapper, addr as usize).map_err(|e| anyhow!("{e:?}"))? };

        PCI::enumerate(&acpi)?;

        Ok(
            ACPI { tables: acpi }
        )
    }
}
