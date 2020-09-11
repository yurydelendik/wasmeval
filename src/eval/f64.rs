use std::mem::transmute;

use crate::values::TrapKind;

const NAN_MASK: u64 = 0x7FF0_0000_0000_0000;
const NAN_DATA_MASK: u64 = 0xF_FFFF_FFFF_FFFF;
const NAN_DATA_CANONICAL: u64 = 0x8_0000_0000_0000;
const NEG_0: u64 = 0x8000_0000_0000_0000;

#[inline]
fn is_nan(a: u64) -> bool {
    (a & NAN_MASK) == NAN_MASK && (a & NAN_DATA_MASK) != 0
}

fn nans(a: u64, b: u64) -> Option<u64> {
    if is_nan(a) {
        if is_nan(b) {
            Some(NAN_MASK | NAN_DATA_CANONICAL | (a ^ b))
        } else {
            Some(a | NAN_DATA_CANONICAL)
        }
    } else if is_nan(b) {
        Some(b | NAN_DATA_CANONICAL)
    } else {
        None
    }
}

#[inline]
pub fn abs(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.abs())
    }
}

#[inline]
pub fn neg(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(-a)
    }
}

#[inline]
pub fn ceil(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.ceil())
    }
}

#[inline]
pub fn floor(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.floor())
    }
}

#[inline]
pub fn trunc(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.trunc())
    }
}

#[inline]
pub fn nearby(a: u64) -> u64 {
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
pub fn sqrt(a: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        transmute(a.sqrt())
    }
}

#[inline]
pub fn add(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a + b)
    }
}

#[inline]
pub fn sub(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a - b)
    }
}

#[inline]
pub fn mul(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a * b)
    }
}

#[inline]
pub fn div(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a / b)
    }
}

#[inline]
pub fn min(a: u64, b: u64) -> u64 {
    if let Some(nan) = nans(a, b) {
        return nan;
    }
    if (a | b) == NEG_0 {
        return NEG_0;
    }
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a.min(b))
    }
}

#[inline]
pub fn max(a: u64, b: u64) -> u64 {
    if let Some(nan) = nans(a, b) {
        return nan;
    }
    if (a | b) == NEG_0 {
        return if a == b { NEG_0 } else { 0 };
    }
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a.max(b))
    }
}

#[inline]
pub fn copysign(a: u64, b: u64) -> u64 {
    unsafe {
        let a: f64 = transmute(a);
        let b: f64 = transmute(b);
        transmute(a.copysign(b))
    }
}

#[inline]
pub fn from_i64(a: i64) -> u64 {
    unsafe {
        let c = a as f64;
        transmute(c)
    }
}

#[inline]
pub fn from_u64(a: i64) -> u64 {
    unsafe {
        let c = a as u64 as f64;
        transmute(c)
    }
}

#[inline]
pub fn from_f32(a: u32) -> u64 {
    unsafe {
        let c = transmute::<_, f32>(a) as f64;
        transmute(c)
    }
}

#[inline]
pub fn from_u32(a: i32) -> u64 {
    unsafe {
        let c = a as u32 as f64;
        transmute(c)
    }
}

#[inline]
pub fn from_i32(a: i32) -> u64 {
    unsafe {
        let c = a as f64;
        transmute(c)
    }
}

#[inline]
pub fn trunc_i32(a: u64) -> Result<i32, TrapKind> {
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
pub fn trunc_u32(a: u64) -> Result<i32, TrapKind> {
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
pub fn trunc_i64(a: u64) -> Result<i64, TrapKind> {
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
pub fn trunc_u64(a: u64) -> Result<i64, TrapKind> {
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
pub fn eq(a: u64, b: u64) -> i32 {
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
pub fn ne(a: u64, b: u64) -> i32 {
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
pub fn lt(a: u64, b: u64) -> i32 {
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
pub fn gt(a: u64, b: u64) -> i32 {
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
pub fn le(a: u64, b: u64) -> i32 {
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
pub fn ge(a: u64, b: u64) -> i32 {
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
