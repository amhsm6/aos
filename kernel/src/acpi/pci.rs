use acpi::AcpiTables;
use acpi::mcfg::Mcfg;
use anyhow::{anyhow, Result};

use crate::memory::MemoryPool;
use crate::{memory, println};
use crate::acpi::mapper::AcpiMapper;

pub struct PCI;

impl PCI {
    pub fn enumerate(acpi: &AcpiTables<AcpiMapper>) -> Result<PCI> {
        println!("Enumerating PCI Bus");

        let mcfg = acpi.find_table::<Mcfg>().map_err(|e| anyhow!("{e:?}"))?;

        for entry in mcfg.entries() {
            let base = entry.base_address;
            let seggroup = entry.pci_segment_group;
            println!("0x{:x}: SEGGROUP {} BUS {} - {}", base, seggroup, entry.bus_number_start, entry.bus_number_end);

            let mut desc_addr = 0x0;
            for bus in entry.bus_number_start..entry.bus_number_end {
                let bus_start = base + bus as u64 * 256 * 4096;
                let pool = MemoryPool::align(bus_start, bus_start + 1);
                println!("Mapping 0x{:x} -- 0x{:x} to 0x{:x} -- 0x{:x}", pool.start, pool.end - 1, desc_addr, desc_addr + pool.size() - 1);

                unsafe { memory::map(pool, desc_addr)?; }
                desc_addr += 0x200000;

                for device in 0..32 {
                    for function in 0..8 {
                        println!("BUS {bus} DEV {device} FUNC {function}");

                        let a = PCI::addr(base, bus, device, function);
                        println!("0x{a:x}");
                        
                        unsafe {
                            println!("0x{:x}", *((desc_addr + function as u64 * 4096) as *const u16));
                        }
                    }
                }
            }
        }

        Ok(PCI)
    }

    pub fn addr(base: u64, bus: u8, device: u8, function: u8) -> u64 {
        base + ((bus as u64) * 256 + (device as u64) * 8 + (function as u64)) * 4096
    }
}
