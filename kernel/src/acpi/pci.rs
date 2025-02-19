use acpi::AcpiTables;
use acpi::mcfg::Mcfg;
use anyhow::{anyhow, Result};

use crate::{memory, println};
use crate::memory::MemoryPool;
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

            let confarea = 0xffffffff_00000000;

            for bus in entry.bus_number_start..entry.bus_number_end {
                let bus_start = base + bus as u64 * 256 * 4096;
                let pool = MemoryPool::single(bus_start);
                unsafe { memory::map(pool, confarea)?; }

                for device in 0..32 {
                    for function in 0..8 {
                        let a = PCI::addr(confarea, bus, device, function);
                        println!("BUS {bus} DEV {device} FUNC {function}");

                        unsafe { println!("0x{:x}", *(a as *const u16)); }
                    }
                }

                unsafe { memory::unmap(confarea, 1)? }
            }
        }

        Ok(PCI)
    }

    pub fn addr(base: u64, bus: u8, device: u8, function: u8) -> u64 {
        base + ((bus as u64) << 20 | (device as u64) << 15 | (function as u64) << 12)
    }
}
