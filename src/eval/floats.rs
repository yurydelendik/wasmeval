use std::mem::transmute;

#[inline]
pub fn abs_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.abs())
    }
}

#[inline]
pub fn neg_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(-a)
    }
}

#[inline]
pub fn ceil_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.ceil())
    }
}

#[inline]
pub fn floor_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.floor())
    }
}

#[inline]
pub fn trunc_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.trunc())
    }
}

#[inline]
pub fn nearby_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(if a.fract().abs() != 0.5 {
            a.round()
        } else {
            (a / 2.0).round() * 2.0
        })
    }
}

#[inline]
pub fn sqrt_f32(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.sqrt())
    }
}

#[inline]
pub fn add_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a + b)
    }
}

#[inline]
pub fn sub_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a - b)
    }
}

#[inline]
pub fn mul_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a * b)
    }
}

#[inline]
pub fn div_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a / b)
    }
}

#[inline]
pub fn min_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a.min(b))
    }
}

#[inline]
pub fn max_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a.max(b))
    }
}

#[inline]
pub fn copysign_f32(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a.copysign(b))
    }
}

#[inline]
pub fn abs_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.abs())
    }
}

#[inline]
pub fn neg_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(-a)
    }
}

#[inline]
pub fn ceil_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.ceil())
    }
}

#[inline]
pub fn floor_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.floor())
    }
}

#[inline]
pub fn trunc_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.trunc())
    }
}

#[inline]
pub fn nearby_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(if a.fract().abs() != 0.5 {
            a.round()
        } else {
            (a / 2.0).round() * 2.0
        })
    }
}

#[inline]
pub fn sqrt_f64(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.sqrt())
    }
}

#[inline]
pub fn add_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a + b)
    }
}

#[inline]
pub fn sub_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a - b)
    }
}

#[inline]
pub fn mul_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a * b)
    }
}

#[inline]
pub fn div_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a / b)
    }
}

#[inline]
pub fn min_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a.min(b))
    }
}

#[inline]
pub fn max_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a.max(b))
    }
}

#[inline]
pub fn copysign_f64(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a.copysign(b))
    }
}

#[inline]
pub fn i64_to_f64(a: i64) -> u64 {
    unsafe {
        let c = a as f64;
        transmute(c)
    }
}

#[inline]
pub fn f32_to_f64(a: u32) -> u64 {
    unsafe {
        let c = transmute::<_, f32>(a) as f64;
        transmute(c)
    }
}

#[inline]
pub fn u32_to_f64(a: i32) -> u64 {
    unsafe {
        let c = a as u32 as f64;
        transmute(c)
    }
}

#[inline]
pub fn i32_to_f64(a: i32) -> u64 {
    unsafe {
        let c = a as f64;
        transmute(c)
    }
}
