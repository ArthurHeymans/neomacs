fn main() {
    use std::mem::size_of;
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8};
    
    let header_size = size_of::<AtomicPtr<u8>>() // forwarding
        + size_of::<&'static str>()  // desc pointer  
        + size_of::<usize>() * 3     // total_size, payload_size, payload_offset
        + size_of::<AtomicU8>() * 3  // space, generation, age, mark_bits
        + size_of::<AtomicBool>();  // moved_out
    
    // Rough estimate with alignment
    println!("Estimated ObjectHeader size: {}", size_of::<AtomicU8>() * 4 
        + size_of::<usize>() * 3 
        + size_of::<AtomicPtr<u8>>()
        + size_of::<AtomicBool>());
}
