use crate::num::ParseFloatError;
use crate::result::Result::{self, Err, Ok};

/// Significant decimal digits kept; any nonzero digit after them sets a sticky digit.
const MAX_DIGITS: usize = 768;
/// u32 limbs per big integer: enough for 10^1094 shifted by the quotient width.
const LIMBS: usize = 144;

struct Big {
    n: usize,
    d: [u32; LIMBS],
}

impl Big {
    fn zero() -> Big {
        Big { n: 0, d: [0u32; LIMBS] }
    }

    fn dup(&self) -> Big {
        let mut r = Big::zero();
        let mut i = 0usize;
        while i < self.n {
            r.d[i] = self.d[i];
            i += 1;
        }
        r.n = self.n;
        r
    }

    fn trim(&mut self) {
        while self.n > 0 && self.d[self.n - 1] == 0 {
            self.n -= 1;
        }
    }

    /// self = self * m + a
    fn mul_add_small(&mut self, m: u32, a: u32) {
        let mut carry = a as u64;
        let mut i = 0usize;
        while i < self.n {
            let v = self.d[i] as u64 * m as u64 + carry;
            self.d[i] = v as u32;
            carry = v >> 32;
            i += 1;
        }
        if carry != 0 {
            self.d[self.n] = carry as u32;
            self.n += 1;
        }
    }

    fn mul_pow10(&mut self, e: u32) {
        let mut e = e;
        while e >= 9 {
            self.mul_add_small(1_000_000_000, 0);
            e -= 9;
        }
        if e > 0 {
            self.mul_add_small(pow10_u32(e), 0);
        }
    }

    fn bit_len(&self) -> u32 {
        if self.n == 0 {
            return 0;
        }
        (self.n as u32 - 1) * 32 + (32 - self.d[self.n - 1].leading_zeros())
    }

    fn shl(&mut self, bits: u32) {
        if self.n == 0 {
            return;
        }
        let ls = (bits / 32) as usize;
        let bs = bits % 32;
        let n = self.n;
        if bs == 0 {
            let mut i = n;
            while i > 0 {
                i -= 1;
                self.d[i + ls] = self.d[i];
            }
            self.n = n + ls;
        } else {
            self.d[n + ls] = self.d[n - 1] >> (32 - bs);
            let mut i = n - 1;
            while i > 0 {
                self.d[i + ls] = (self.d[i] << bs) | (self.d[i - 1] >> (32 - bs));
                i -= 1;
            }
            self.d[ls] = self.d[0] << bs;
            self.n = n + ls + 1;
        }
        let mut i = 0usize;
        while i < ls {
            self.d[i] = 0;
            i += 1;
        }
        self.trim();
    }

    fn shr1(&mut self) {
        let mut i = 0usize;
        while i < self.n {
            let hi = if i + 1 < self.n { self.d[i + 1] << 31 } else { 0 };
            self.d[i] = (self.d[i] >> 1) | hi;
            i += 1;
        }
        self.trim();
    }

    /// self -= o, requires self >= o.
    fn sub(&mut self, o: &Big) {
        let mut borrow: u64 = 0;
        let mut i = 0usize;
        while i < self.n {
            let a = self.d[i] as u64;
            let b = (if i < o.n { o.d[i] } else { 0 }) as u64 + borrow;
            if a >= b {
                self.d[i] = (a - b) as u32;
                borrow = 0;
            } else {
                self.d[i] = (a + (1u64 << 32) - b) as u32;
                borrow = 1;
            }
            i += 1;
        }
        self.trim();
    }
}

fn pow10_u32(e: u32) -> u32 {
    let mut p: u32 = 1;
    let mut i = 0u32;
    while i < e {
        p *= 10;
        i += 1;
    }
    p
}

/// -1, 0 or 1 as a < b, a == b, a > b.
fn cmp_big(a: &Big, b: &Big) -> i32 {
    if a.n != b.n {
        return if a.n < b.n { -1 } else { 1 };
    }
    let mut i = a.n;
    while i > 0 {
        i -= 1;
        if a.d[i] != b.d[i] {
            return if a.d[i] < b.d[i] { -1 } else { 1 };
        }
    }
    0
}

/// Quotient (known to be below 2^(qbits + 1)) and remainder of num / den.
fn div_small(num: &Big, den: &Big, qbits: u32) -> (u64, Big) {
    let mut r = num.dup();
    let mut t = den.dup();
    t.shl(qbits);
    let mut q: u64 = 0;
    let mut s = qbits;
    loop {
        if cmp_big(&r, &t) >= 0 {
            r.sub(&t);
            q |= 1u64 << s;
        }
        if s == 0 {
            break;
        }
        t.shr1();
        s -= 1;
    }
    (q, r)
}

fn is_digit(b: u8) -> bool {
    b >= b'0' && b <= b'9'
}

fn is_negative(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes[0] == b'-'
}

/// True when the accepted text is an `inf`/`infinity`/`nan` spelling rather than a number.
fn is_special(bytes: &[u8]) -> bool {
    let mut i = 0usize;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    i < bytes.len() && !is_digit(bytes[i]) && bytes[i] != b'.'
}

/// Correctly rounded (nearest, ties to even) magnitude bits of an already validated decimal.
/// `mant_bits` counts the hidden bit; the sign is applied by the caller.
fn decode(bytes: &[u8], mant_bits: u32, bias: i32, max_dp: i64, min_dp: i64) -> u64 {
    let mut digits = [0u8; MAX_DIGITS + 1];
    let mut nd = 0usize;
    let mut sticky = false;
    let mut exp10: i64 = 0;
    let mut i = 0usize;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    while i < bytes.len() && is_digit(bytes[i]) {
        let d = bytes[i] - b'0';
        if nd == 0 && d == 0 {
        } else if nd < MAX_DIGITS {
            digits[nd] = d;
            nd += 1;
        } else {
            if d != 0 {
                sticky = true;
            }
            exp10 += 1;
        }
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && is_digit(bytes[i]) {
            let d = bytes[i] - b'0';
            if nd == 0 && d == 0 {
                exp10 -= 1;
            } else if nd < MAX_DIGITS {
                digits[nd] = d;
                nd += 1;
                exp10 -= 1;
            } else if d != 0 {
                sticky = true;
            }
            i += 1;
        }
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        let mut exp_neg = false;
        if i < bytes.len() {
            if bytes[i] == b'-' {
                exp_neg = true;
                i += 1;
            } else if bytes[i] == b'+' {
                i += 1;
            }
        }
        let mut ev: i64 = 0;
        while i < bytes.len() && is_digit(bytes[i]) {
            if ev < 1_000_000 {
                ev = ev * 10 + (bytes[i] - b'0') as i64;
            }
            i += 1;
        }
        if exp_neg {
            exp10 -= ev;
        } else {
            exp10 += ev;
        }
    }

    if nd == 0 {
        return 0;
    }
    if sticky {
        // The dropped tail lies strictly between D and D + 1; one more nonzero digit keeps it there.
        digits[nd] = 1;
        nd += 1;
        exp10 -= 1;
    } else {
        while nd > 0 && digits[nd - 1] == 0 {
            nd -= 1;
            exp10 += 1;
        }
    }

    let inf_bits: u64 = ((2 * bias + 1) as u64) << (mant_bits - 1);
    let dp = exp10 + nd as i64;
    if dp > max_dp {
        return inf_bits;
    }
    if dp < min_dp {
        return 0;
    }

    let mut num = Big::zero();
    let mut chunk: u32 = 0;
    let mut clen: u32 = 0;
    let mut j = 0usize;
    while j < nd {
        chunk = chunk * 10 + digits[j] as u32;
        clen += 1;
        if clen == 9 {
            num.mul_add_small(1_000_000_000, chunk);
            chunk = 0;
            clen = 0;
        }
        j += 1;
    }
    if clen > 0 {
        num.mul_add_small(pow10_u32(clen), chunk);
    }
    let mut den = Big::zero();
    den.mul_add_small(1, 1);
    if exp10 >= 0 {
        num.mul_pow10(exp10 as u32);
    } else {
        den.mul_pow10((-exp10) as u32);
    }

    let p = mant_bits as i64;
    // Unit of the smallest subnormal: 2^-1074 for f64, 2^-149 for f32.
    let kmin: i64 = 2 - bias as i64 - p;
    // value = num / den; pick k so that floor(value / 2^k) has mant_bits bits.
    let mut k: i64 = num.bit_len() as i64 - den.bit_len() as i64 - p;
    loop {
        let mut ke = if k < kmin { kmin } else { k };
        let mut n2 = num.dup();
        let mut d2 = den.dup();
        if ke >= 0 {
            d2.shl(ke as u32);
        } else {
            n2.shl((-ke) as u32);
        }
        let (q0, rem) = div_small(&n2, &d2, mant_bits + 1);
        if (q0 >> mant_bits) != 0 {
            k = ke + 1;
            continue;
        }
        let mut q = q0;
        let mut r2 = rem;
        r2.shl(1);
        let c = cmp_big(&r2, &d2);
        if c > 0 || (c == 0 && (q & 1) == 1) {
            q += 1;
        }
        if (q >> mant_bits) != 0 {
            q >>= 1;
            ke += 1;
        }
        if q == 0 {
            return 0;
        }
        let hidden = 1u64 << (mant_bits - 1);
        if q < hidden {
            return q;
        }
        let biased = ke + p - 1 + bias as i64;
        if biased >= (2 * bias + 1) as i64 {
            return inf_bits;
        }
        return ((biased as u64) << (mant_bits - 1)) | (q - hidden);
    }
}

pub(crate) fn parse_f64(s: &str) -> Result<f64, ParseFloatError> {
    match crate::num::parse_f64(s) {
        Err(e) => Err(e),
        Ok(v) => {
            let bytes = s.as_bytes();
            if is_special(bytes) {
                return Ok(v);
            }
            let bits = decode(bytes, 53, 1023, 310, -326);
            let sign: u64 = if is_negative(bytes) { 1u64 << 63 } else { 0 };
            Ok(f64::from_bits(bits | sign))
        }
    }
}

pub(crate) fn parse_f32(s: &str) -> Result<f32, ParseFloatError> {
    match crate::num::parse_f64(s) {
        Err(e) => Err(e),
        Ok(v) => {
            let bytes = s.as_bytes();
            if is_special(bytes) {
                return Ok(v as f32);
            }
            let bits = decode(bytes, 24, 127, 40, -47) as u32;
            let sign: u32 = if is_negative(bytes) { 1u32 << 31 } else { 0 };
            Ok(f32::from_bits(bits | sign))
        }
    }
}
