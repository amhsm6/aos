use core::ptr::NonNull;
use core::cell::RefCell;
use acpi::{mcfg::Mcfg, AcpiHandler, AcpiTables, PhysicalMapping};
use anyhow::{anyhow, Result};

use crate::println;
use crate::mem::MemoryPool;

#[derive(Clone)]
struct AcpiMapper {
    top: RefCell<u64>
}

impl AcpiMapper {
    fn new() -> AcpiMapper {
        AcpiMapper { top: RefCell::new(0xffffffff80000000) }
    }
}

impl AcpiHandler for AcpiMapper {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        let pool = MemoryPool::align(physical_address as u64, physical_address as u64 + size as u64);
        let offset = physical_address as u64 - pool.start;
        let mapped_size = pool.end - pool.start;

        let virt = *self.top.borrow();
        *self.top.borrow_mut() += mapped_size;

        // TODO: fix remapping same page
        if let Err(err) = crate::mem::map(pool, virt) {
            println!("Error mapping memory: {err}")
        }

        println!("Mapping 0x{:x} - 0x{:x} to 0x{:x}. REQ 0x{:x}", pool.start, pool.end, virt, physical_address);

        PhysicalMapping::new(
            physical_address,
            NonNull::new((virt + offset) as *mut T).expect("Impossible"),
            mapped_size as usize,
            mapped_size as usize,
            self.clone()
        )
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}

pub unsafe fn parse(addr: u64) -> Result<()> {
    println!("Parsing...");
    let mapper = AcpiMapper::new();
    let acpi = AcpiTables::from_rsdp(mapper, addr as usize).map_err(|e| anyhow!("{e:?}"))?;

    let mcfg = acpi.find_table::<Mcfg>().map_err(|e| anyhow!("{e:?}"))?;
    println!("0x{:x}", mcfg.physical_start());
    for entry in mcfg.entries() {
       let base = entry.base_address;
       let seggroup = entry.pci_segment_group;
       println!("0x{:x}: SEGGROUP {} BUS {} - {}", base, seggroup, entry.bus_number_start, entry.bus_number_end);
    }

    Ok(())
}
