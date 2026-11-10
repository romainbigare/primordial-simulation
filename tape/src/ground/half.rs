//! 16-bit floats (IEEE 754 binary16), as a graphics card reads them: a tile's numbers take half the room of 32-bit ones.

/// `v` as the nearest 16-bit float, ties to even; past the largest, infinite.
pub fn half(v: f32) -> u16 {
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    if b & 0x7fff_ffff > 0x7f80_0000 {
        return sign | 0x7e00;
    }
    let (e, m) = (((b >> 23) & 0xff) as i32 - 127 + 15, b & 0x7f_ffff);
    if e >= 31 {
        return sign | 0x7c00;
    }
    // Below the least normal 16-bit float the leading bit joins the fraction (a subnormal).
    let (m, shift) = if e <= 0 { (m | 0x80_0000, (14 - e) as u32) } else { (m, 13) };
    if shift > 24 {
        return sign;
    }
    let (kept, rest, mid) = (m >> shift, m & ((1 << shift) - 1), 1 << (shift - 1));
    let rounded = kept + (rest > mid || (rest == mid && kept & 1 == 1)) as u32;
    // A carry out of the fraction raises the exponent, and past the largest gives infinity: the bits add up so.
    sign | (if e <= 0 { rounded } else { ((e as u32) << 10) + rounded }) as u16
}

/// The value of 16-bit float `h`.
pub fn unhalf(h: u16) -> f32 {
    let (sign, e, m) = ((h as u32 & 0x8000) << 16, (h as u32 >> 10) & 0x1f, h as u32 & 0x3ff);
    match e {
        0 => f32::from_bits(sign) + if sign == 0 { 1.0 } else { -1.0 } * m as f32 * f32::from_bits(0x3380_0000),
        31 => f32::from_bits(sign | 0x7f80_0000 | m << 13),
        _ => f32::from_bits(sign | (e + 127 - 15) << 23 | m << 13),
    }
}
