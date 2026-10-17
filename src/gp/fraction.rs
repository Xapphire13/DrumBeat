use std::fmt;
use std::ops::{Add, Mul};

/// An exact, always-reduced fraction, used for rhythmic positions and lengths
/// so tuplets don't accumulate rounding error.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fraction {
    num: u32,
    den: u32,
}

impl Fraction {
    pub const ZERO: Fraction = Fraction { num: 0, den: 1 };

    pub fn new(num: u32, den: u32) -> Self {
        assert!(den != 0, "fraction with zero denominator");
        let divisor = gcd(num, den);
        Fraction {
            num: num / divisor,
            den: den / divisor,
        }
    }

    pub fn num(self) -> u32 {
        self.num
    }

    pub fn den(self) -> u32 {
        self.den
    }

    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Convert to a whole number of ticks, or `None` if this fraction doesn't
    /// land exactly on one (e.g. a triplet at 256th-note resolution)
    pub fn to_ticks(self, ticks_per_whole: u32) -> Option<u32> {
        let ticks = self.num as u64 * ticks_per_whole as u64;
        let remainder = ticks % self.den as u64;
        (remainder == 0).then(|| (ticks / self.den as u64) as u32)
    }
}

impl Add for Fraction {
    type Output = Fraction;

    fn add(self, other: Fraction) -> Fraction {
        let den = self.den / gcd(self.den, other.den) * other.den;
        Fraction::new(
            self.num * (den / self.den) + other.num * (den / other.den),
            den,
        )
    }
}

impl Mul for Fraction {
    type Output = Fraction;

    fn mul(self, other: Fraction) -> Fraction {
        // Cross-reduce first to keep the intermediate products small
        let a = gcd(self.num, other.den);
        let b = gcd(other.num, self.den);
        Fraction::new(
            (self.num / a) * (other.num / b),
            (self.den / b) * (other.den / a),
        )
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.num as u64 * other.den as u64).cmp(&(other.num as u64 * self.den as u64))
    }
}

impl fmt::Debug for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}
