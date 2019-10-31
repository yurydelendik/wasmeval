use std::mem::transmute;

use crate::values::TrapKind;

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
pub fn u64_to_f64(a: i64) -> u64 {
    unsafe {
        let c = a as u64 as f64;
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
pub fn f64_to_f32(a: u64) -> u32 {
    unsafe {
        let c = transmute::<_, f64>(a) as f32;
        transmute(c)
    }
}

#[inline]
pub fn u32_to_f32(a: i32) -> u32 {
    unsafe {
        let c = a as u32 as f32;
        transmute(c)
    }
}

#[inline]
pub fn i32_to_f32(a: i32) -> u32 {
    unsafe {
        let c = a as f32;
        transmute(c)
    }
}

#[inline]
pub fn u64_to_f32(a: i64) -> u32 {
    unsafe {
        let c = a as u64 as f32;
        transmute(c)
    }
}

#[inline]
pub fn i64_to_f32(a: i64) -> u32 {
    unsafe {
        let c = a as f32;
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

#[inline]
pub fn f32_trunc_i32(a: u32) -> Result<i32, TrapKind> {
    unsafe {
        let a = transmute::<_, f32>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f32 = -2147483648.0;
        const MAX: f32 = 2147483520.0;
        if MIN <= a && a <= MAX {
            Ok(a as i32)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f32_trunc_u32(a: u32) -> Result<i32, TrapKind> {
    unsafe {
        let a = transmute::<_, f32>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f32 = 0.0;
        const MAX: f32 = 4294967040.0;
        if MIN <= a && a <= MAX {
            Ok(a as u32 as i32)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f64_trunc_i32(a: u64) -> Result<i32, TrapKind> {
    unsafe {
        let a = transmute::<_, f64>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f64 = -2147483648.0;
        const MAX: f64 = 2147483647.0;
        if MIN <= a && a <= MAX {
            Ok(a as i32)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f64_trunc_u32(a: u64) -> Result<i32, TrapKind> {
    unsafe {
        let a = transmute::<_, f64>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f64 = 0.0;
        const MAX: f64 = 4294967295.0;
        if MIN <= a && a <= MAX {
            Ok(a as u32 as i32)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f32_trunc_i64(a: u32) -> Result<i64, TrapKind> {
    unsafe {
        let a = transmute::<_, f32>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f32 = -9223372036854775808.0;
        const MAX: f32 = 9223371487098961920.0;
        if MIN <= a && a <= MAX {
            Ok(a as i64)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f32_trunc_u64(a: u32) -> Result<i64, TrapKind> {
    unsafe {
        let a = transmute::<_, f32>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f32 = 0.0;
        const MAX: f32 = 18446742974197923840.0;
        if MIN <= a && a <= MAX {
            Ok(a as u64 as i64)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f64_trunc_i64(a: u64) -> Result<i64, TrapKind> {
    unsafe {
        let a = transmute::<_, f64>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f64 = -9223372036854775808.0;
        const MAX: f64 = 9223372036854774784.0;
        if MIN <= a && a <= MAX {
            Ok(a as i64)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn f64_trunc_u64(a: u64) -> Result<i64, TrapKind> {
    unsafe {
        let a = transmute::<_, f64>(a).trunc();
        if a.is_nan() {
            return Err(TrapKind::InvalidIntegerConversion);
        }
        const MIN: f64 = 0.0;
        const MAX: f64 = 18446744073709550000.0;
        if MIN <= a && a <= MAX {
            Ok(a as u64 as i64)
        } else {
            Err(TrapKind::IntegerOverflow)
        }
    }
}

#[inline]
pub fn eq_f32(a: u32, b: u32) -> i32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        if a == b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn ne_f32(a: u32, b: u32) -> i32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        if a == b {
            0
        } else {
            1
        }
    }
}

#[inline]
pub fn lt_f32(a: u32, b: u32) -> i32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        if a < b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn gt_f32(a: u32, b: u32) -> i32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        if a > b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn le_f32(a: u32, b: u32) -> i32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        if a <= b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn ge_f32(a: u32, b: u32) -> i32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        if a >= b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn eq_f64(a: u64, b: u64) -> i32 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        if a == b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn ne_f64(a: u64, b: u64) -> i32 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        if a == b {
            0
        } else {
            1
        }
    }
}

#[inline]
pub fn lt_f64(a: u64, b: u64) -> i32 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        if a < b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn gt_f64(a: u64, b: u64) -> i32 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        if a > b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn le_f64(a: u64, b: u64) -> i32 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        if a <= b {
            1
        } else {
            0
        }
    }
}

#[inline]
pub fn ge_f64(a: u64, b: u64) -> i32 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        if a >= b {
            1
        } else {
            0
        }
    }
}
