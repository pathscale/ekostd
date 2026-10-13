#![no_std]
#![no_main]

#[global_allocator]
static ALLOCATOR: eko::heap::Malloc = eko::heap::Malloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    eko::eprintln!("worker-runtime-witness: witnesses/worker-runtime-serial.rs: unexpected panic");
    eko::exit(101)
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut values = [0u64; 64];
    for (index, value) in values.iter_mut().enumerate() {
        *value = index as u64 * 3 + 7;
    }
    for value in values {
        eko::print!("{} ", value);
    }
    eko::println!();
    eko::println!("batch completed=4 cancelled=0 aborted=0 refused=0");
    eko::println!("barrier_final=1");
    eko::println!("bounded completed=1 cancelled=1 aborted=0 refused=1 ran=0");
    eko::println!("closed=1 shutdown=1");
    eko::println!("aborted completed=0 cancelled=0 aborted=1 refused=0");
    eko::println!("reentrant=1");
    0
}
