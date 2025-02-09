use anyhow::{anyhow, Error, Result};
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size2MiB, Size4KiB};

pub const KERNEL_START: u64 = 0xffffffffaf000000;
pub const KERNEL_END:   u64 = 0xffffffffffffffff;
pub const KERNEL_SIZE:  u64 = KERNEL_END - KERNEL_START + 1;

#[derive(Clone, Copy)]
pub struct MemoryPool {
    pub start: u64,
    pub end:   u64
}

impl MemoryPool {
    pub unsafe fn map<A: FrameAllocator<Size4KiB>>(&self, page_table: &mut OffsetPageTable, falloc: &mut A, vstart: u64) -> Result<()> {
        let pstart = self.start;
        let pend = self.end - 1;
        let vend = vstart + (pend - pstart);

        let vstart: Page<Size2MiB> = Page::from_start_address(VirtAddr::new(vstart)).map_err(Error::msg)?;
        let vend = Page::containing_address(VirtAddr::new(vend));
        let pstart = PhysFrame::from_start_address(PhysAddr::new(pstart)).map_err(Error::msg)?;
        let pend = PhysFrame::containing_address(PhysAddr::new(pend));

        let pages = Page::range_inclusive(vstart, vend);
        let frames = PhysFrame::range_inclusive(pstart, pend);

        if pages.len() != frames.len() { return Err(anyhow!("Incorrect mapping")); }

        for (page, frame) in pages.zip(frames) {
            page_table
                .map_to(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE, falloc)
                .map_err(|e| anyhow!("{e:?}"))?
                .flush();
        }

        Ok(())
    }
}
