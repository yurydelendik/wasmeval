use std::mem::transmute;

pub fn add_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a + b)
    }
}

pub fn add_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a + b)
    }
}
