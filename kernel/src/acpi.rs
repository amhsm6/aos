use core::ptr::NonNull;
use acpi::{mcfg::Mcfg, AcpiHandler, AcpiTables, PhysicalMapping};
use anyhow::{anyhow, Result};

use crate::println;

#[derive(Clone)]
struct AcpiMapper;

impl AcpiHandler for AcpiMapper {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        let virt = NonNull::new(physical_address as *mut T).unwrap();
        PhysicalMapping::new(physical_address, virt, size, size, self.clone())
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}

pub fn parse(addr: u64) -> Result<()> {
    println!("Parsing...");
    let acpi = unsafe { AcpiTables::from_rsdp(AcpiMapper, addr as usize).map_err(|e| anyhow!("{e:?}"))? };

    let mcfg = acpi.find_table::<Mcfg>().map_err(|e| anyhow!("{e:?}"))?;
    println!("0x{:x}", mcfg.physical_start());
    for entry in mcfg.entries() {
       let base = entry.base_address;
       let seggroup = entry.pci_segment_group;
       println!("0x{:x}: SEGGROUP {} BUS {} - {}", base, seggroup, entry.bus_number_start, entry.bus_number_end);
    }

    Ok(())
}
