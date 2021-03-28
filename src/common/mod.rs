#![macro_use]

pub mod logger;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
pub use stm32f1xx_hal as hal;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
const HEAP_SIZE: usize = 4 * 1024; // in bytes

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    panic!("Alloc error.");
}

pub fn init_alloc() {
    unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) };
}
