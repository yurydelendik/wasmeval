use std::mem::transmute;

use crate::values::TrapKind;

#[inline]
pub fn abs(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.abs())
    }
}

#[inline]
pub fn neg(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(-a)
    }
}

#[inline]
pub fn ceil(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.ceil())
    }
}

#[inline]
pub fn floor(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.floor())
    }
}

#[inline]
pub fn trunc(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.trunc())
    }
}

#[inline]
pub fn nearby(a: u32) -> u32 {
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
pub fn sqrt(a: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        transmute(a.sqrt())
    }
}

#[inline]
pub fn add(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a + b)
    }
}

#[inline]
pub fn sub(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a - b)
    }
}

#[inline]
pub fn mul(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a * b)
    }
}

#[inline]
pub fn div(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a / b)
    }
}

#[inline]
pub fn min(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a.min(b))
    }
}

#[inline]
pub fn max(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a.max(b))
    }
}

#[inline]
pub fn copysign(a: u32, b: u32) -> u32 {
    unsafe {
        let a: f32 = transmute(a);
        let b: f32 = transmute(b);
        transmute(a.copysign(b))
    }
}

#[inline]
pub fn from_u32(a: i32) -> u32 {
    unsafe {
        let c = a as u32 as f32;
        transmute(c)
    }
}

#[inline]
pub fn from_f64(a: u64) -> u32 {
    unsafe {
        let c = transmute::<_, f64>(a) as f32;
        transmute(c)
    }
}

#[inline]
pub fn from_i32(a: i32) -> u32 {
    unsafe {
        let c = a as f32;
        transmute(c)
    }
}

#[inline]
pub fn from_u64(a: i64) -> u32 {
    unsafe {
        let c = a as u64 as f32;
        transmute(c)
    }
}

#[inline]
pub fn from_i64(a: i64) -> u32 {
    unsafe {
        let c = a as f32;
        transmute(c)
    }
}

#[inline]
pub fn trunc_i32(a: u32) -> Result<i32, TrapKind> {
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
pub fn trunc_u32(a: u32) -> Result<i32, TrapKind> {
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
pub fn trunc_i64(a: u32) -> Result<i64, TrapKind> {
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
pub fn trunc_u64(a: u32) -> Result<i64, TrapKind> {
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
pub fn eq(a: u32, b: u32) -> i32 {
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
pub fn ne(a: u32, b: u32) -> i32 {
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
pub fn lt(a: u32, b: u32) -> i32 {
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
pub fn gt(a: u32, b: u32) -> i32 {
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
pub fn le(a: u32, b: u32) -> i32 {
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
pub fn ge(a: u32, b: u32) -> i32 {
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
