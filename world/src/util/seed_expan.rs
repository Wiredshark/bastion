/// The zerocopy crate exists and can replace this function.
/// We should evaluate using it when we have multiple usage spots for it.
/// For now we have this safe alternative.
fn cast_u32x8_u8x32(a: [u32; 8]) -> [u8; 32] {
    let mut r = [0; 32];
    for i in 0..8 {
        // RNG-DEEP-001 (determinism audit): to_LE_bytes, not to_ne_bytes —
        // the world-seed expansion must be endianness-portable, or the same
        // seed produces a different RNG state (and thus a different world /
        // every downstream keyed stream) on a big-endian host. No-op on the
        // little-endian x86 fleet, so no current values shift; correctness by
        // construction for portability.
        let a = a[i].to_le_bytes();
        for j in 0..4 {
            r[i * 4 + j] = a[j];
        }
    }
    r
}

/// Simple non-cryptographic diffusion function.
#[inline(always)]
pub fn diffuse(mut a: u32) -> u32 {
    a ^= a.rotate_right(23);
    a.wrapping_mul(2654435761)
}

/// Diffuse but takes multiple values as input.
#[inline(always)]
pub fn diffuse_mult(v: &[u32]) -> u32 {
    let mut state = (1 << 31) - 1;
    for e in v {
        state = diffuse(state ^ e);
    }
    state
}

/// Expand a 32 bit seed into a 32 byte RNG state.
pub fn rng_state(mut x: u32) -> [u8; 32] {
    let mut r: [u32; 8] = [0; 8];
    for s in &mut r {
        x = diffuse(x);
        *s = x;
    }
    cast_u32x8_u8x32(r)
}
