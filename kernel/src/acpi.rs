use core::ptr::NonNull;
use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};
use anyhow::{anyhow, Error, Result};

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

pub fn parse(addr: usize) -> Result<()> {
    println!("Parsing...");
    let acpi = unsafe { AcpiTables::from_rsdp(AcpiMapper, addr).map_err(|e| anyhow!("{e:?}"))? };

    for h in acpi.headers() {
        println!("{h:?}");
    }

    Ok(())
}
