use core::ptr::NonNull;

use crate::println;
use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};

#[derive(Clone)]
struct AcpiMapper {}

impl AcpiHandler for AcpiMapper {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        let virt = NonNull::new(physical_address as *mut T).unwrap();
        PhysicalMapping::new(physical_address, virt, size, size, self.clone())
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}

pub fn parse(acpi: *const ()) {
    let acpi = unsafe { AcpiTables::from_rsdp(AcpiMapper {}, acpi as usize).unwrap() };
}
