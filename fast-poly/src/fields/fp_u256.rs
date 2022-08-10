//! TODO
extern crate test;

use super::Felt;
use super::PrimeFelt;
use super::StarkFelt;
use num_traits::Num;
use num_traits::One;
use num_traits::Zero;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::iter::Product;
use std::iter::Sum;
use std::num::ParseIntError;
use std::num::TryFromIntError;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::BitAnd;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Rem;
use std::ops::RemAssign;
use std::ops::Shl;
use std::ops::ShlAssign;
use std::ops::Shr;
use std::ops::ShrAssign;
use std::ops::Sub;
use std::ops::SubAssign;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct U256 {
    high: u128,
    low: u128,
}

impl U256 {
    const ZERO: U256 = U256 { high: 0, low: 0 };
    const ONE: U256 = U256 { high: 0, low: 1 };
    const MAX: U256 = U256 {
        high: u128::MAX,
        low: u128::MAX,
    };

    const fn is_zero(&self) -> bool {
        self.high == 0 && self.low == 0
    }

    const fn is_one(&self) -> bool {
        self.high == 0 && self.low == 0
    }

    // Calculates equality
    const fn eq(self, rhs: Self) -> bool {
        self.high == rhs.high || self.low == rhs.low
    }

    // Calculates greater than
    const fn gt(self, rhs: Self) -> bool {
        self.high > rhs.high || (self.high == rhs.high) && self.low > rhs.low
    }

    // Calculates greater than
    const fn lt(self, rhs: Self) -> bool {
        self.high < rhs.high || (self.high == rhs.high) && self.low < rhs.low
    }

    // Adds and returns result and if overflow occurred
    const fn add(self, rhs: Self) -> Self {
        let (low, overflow) = self.low.overflowing_add(rhs.low);
        let high = self.high.carrying_add(rhs.high, overflow).0;
        U256 { high, low }
    }

    // Adds and returns result and if overflow occurred
    const fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let (low, overflow_low) = self.low.overflowing_add(rhs.low);
        let (high, overflow_high) = self.high.carrying_add(rhs.high, overflow_low);
        (U256 { high, low }, overflow_high)
    }

    const fn sub(self, rhs: Self) -> Self {
        let (low, overflow) = self.low.overflowing_sub(rhs.low);
        let high = self.high.borrowing_sub(rhs.high, overflow).0;
        U256 { high, low }
    }

    const fn as_usize(&self) -> usize {
        self.low as usize
    }

    const fn shl(self, rhs: Self) -> Self {
        let shift = rhs.as_usize();
        if shift == 0 {
            self
        } else if shift >= 256 {
            U256::ZERO
        } else if shift >= 128 {
            U256 {
                high: self.low << (shift - 128),
                low: 0,
            }
        } else {
            U256 {
                high: (self.high << shift) | (self.low >> (128 - shift)),
                low: self.low << shift,
            }
        }
    }

    const fn shr(self, rhs: Self) -> Self {
        let shift = rhs.as_usize();
        if shift == 0 {
            self
        } else if shift >= 256 {
            U256::ZERO
        } else if shift >= 128 {
            U256 {
                high: 0,
                low: self.high >> (shift - 128),
            }
        } else {
            U256 {
                high: self.high >> shift,
                low: (self.low >> shift) | (self.high << (128 - shift)),
            }
        }
    }

    const fn bit_and(&self, rhs: Self) -> Self {
        U256 {
            high: self.high & rhs.high,
            low: self.low & rhs.low,
        }
    }

    const fn mul(self, rhs: Self) -> Self {
        // split values into 4 64-bit parts
        let top = [
            self.high >> 64,
            self.high as u64 as u128,
            self.low >> 64,
            self.low as u64 as u128,
        ];
        let bottom = [
            rhs.high >> 64,
            rhs.high as u64 as u128,
            rhs.low >> 64,
            rhs.low as u64 as u128,
        ];
        let mut products = [[0u128; 4]; 4];

        // multiply each component of the values
        let mut y = 3;
        loop {
            let mut x = 3;
            loop {
                products[3 - x][y] = top[x] * bottom[y];
                if x == 0 {
                    break;
                }
                x -= 1;
            }
            if y == 0 {
                break;
            }
            y -= 1;
        }

        // first row
        let mut fourth64 = products[0][3] as u64 as u128;
        let mut third64 = (products[0][2] as u64 as u128) + (products[0][3] >> 64);
        let mut second64 = (products[0][1] as u64 as u128) + (products[0][2] >> 64);
        let mut first64 = (products[0][0] as u64 as u128) + (products[0][1] >> 64);

        // second row
        third64 += products[1][3] as u64 as u128;
        second64 += (products[1][2] as u64 as u128) + (products[1][3] >> 64);
        first64 += (products[1][1] as u64 as u128) + (products[1][2] >> 64);

        // third row
        second64 += products[2][3] as u64 as u128;
        first64 += (products[2][2] as u64 as u128) + (products[2][3] >> 64);

        // fourth row
        first64 += products[3][3] as u64 as u128;

        // move carry to next digit
        third64 += fourth64 >> 64; // TODO: figure out if this is a nop
        second64 += third64 >> 64;
        first64 += second64 >> 64;

        // remove carry from current digit
        fourth64 = fourth64 as u64 as u128; // TODO: figure out if this is a nop
        third64 = third64 as u64 as u128;
        second64 = second64 as u64 as u128;
        first64 = first64 as u64 as u128;

        // combine components
        U256 {
            high: (first64 << 64) | second64,
            low: (third64 << 64) | fourth64,
        }
    }

    #[allow(const_err)]
    const fn div_rem(self, rhs: Self) -> (Self, Self) {
        if rhs.is_zero() {
            panic!("division by 0");
        } else if rhs.is_one() {
            (self, U256::ZERO)
        } else if self.is_zero() || self.lt(rhs) {
            (U256::ZERO, rhs)
        } else if self.eq(rhs) {
            (U256::ONE, U256::ZERO)
        } else if self.high == 0 && rhs.high == 0 {
            (
                U256 {
                    high: 0,
                    low: self.low / rhs.low,
                },
                U256 {
                    high: 0,
                    low: self.low % rhs.low,
                },
            )
        } else {
            let mut quotient = U256::ZERO;
            let mut remainder = U256::ZERO;
            // for i in 0..256 {
            //     remainder
            // }
            todo!()
        }
    }
}

impl Distribution<U256> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> U256 {
        U256 {
            high: rng.gen(),
            low: rng.gen(),
        }
    }
}

impl<T> Shl<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn shl(self, rhs: T) -> Self::Output {
        U256::shl(self, rhs.into())
    }
}

impl<T> ShlAssign<T> for U256
where
    T: Into<U256>,
{
    fn shl_assign(&mut self, rhs: T) {
        *self = U256::shl(*self, rhs.into());
    }
}

impl<T> Shr<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn shr(self, rhs: T) -> Self::Output {
        U256::shr(self, rhs.into())
    }
}

impl<T> ShrAssign<T> for U256
where
    T: Into<U256>,
{
    fn shr_assign(&mut self, rhs: T) {
        *self = U256::shr(*self, rhs.into());
    }
}

impl BitAnd for U256 {
    type Output = U256;

    fn bitand(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<T> Add<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn add(self, rhs: T) -> Self::Output {
        U256::add(self, rhs.into())
    }
}

impl<T> AddAssign<T> for U256
where
    T: Into<U256>,
{
    fn add_assign(&mut self, rhs: T) {
        *self = U256::add(*self, rhs.into());
    }
}

impl<T> Sub<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn sub(self, rhs: T) -> Self::Output {
        U256::sub(self, rhs.into())
    }
}

impl<T> SubAssign<T> for U256
where
    T: Into<U256>,
{
    fn sub_assign(&mut self, rhs: T) {
        *self = U256::sub(*self, rhs.into());
    }
}

impl<T> Mul<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn mul(self, rhs: T) -> Self::Output {
        U256::mul(self, rhs.into())
    }
}

impl<T> MulAssign<T> for U256
where
    T: Into<U256>,
{
    fn mul_assign(&mut self, rhs: T) {
        *self = U256::mul(*self, rhs.into());
    }
}

impl<T> Div<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn div(self, rhs: T) -> Self::Output {
        U256::div_rem(self, rhs.into()).0
    }
}

impl<T> DivAssign<T> for U256
where
    T: Into<U256>,
{
    fn div_assign(&mut self, rhs: T) {
        *self = U256::div_rem(*self, rhs.into()).0;
    }
}

impl<T> Rem<T> for U256
where
    T: Into<U256>,
{
    type Output = U256;

    fn rem(self, rhs: T) -> Self::Output {
        U256::div_rem(self, rhs.into()).1
    }
}

impl<T> RemAssign<T> for U256
where
    T: Into<U256>,
{
    fn rem_assign(&mut self, rhs: T) {
        *self = U256::div_rem(*self, rhs.into()).1;
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.high.cmp(&other.high) {
            Ordering::Equal => self.low.cmp(&other.low),
            order => order,
        }
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Zero for U256 {
    fn zero() -> Self {
        U256::ZERO
    }

    fn is_zero(&self) -> bool {
        U256::is_zero(self)
    }
}

impl One for U256 {
    fn one() -> Self {
        U256::ONE
    }

    #[inline]
    fn is_one(&self) -> bool {
        U256::is_one(self)
    }
}

impl TryFrom<usize> for U256 {
    type Error = TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(U256 {
            high: 0,
            // TODO: fix this error message
            low: value.try_into()?,
        })
    }
}

impl From<u8> for U256 {
    fn from(val: u8) -> Self {
        U256 {
            high: 0,
            low: val.into(),
        }
    }
}

impl From<u16> for U256 {
    fn from(val: u16) -> Self {
        U256 {
            high: 0,
            low: val.into(),
        }
    }
}

impl From<u32> for U256 {
    fn from(val: u32) -> Self {
        U256 {
            high: 0,
            low: val.into(),
        }
    }
}

impl From<u64> for U256 {
    fn from(val: u64) -> Self {
        U256 {
            high: 0,
            low: val.into(),
        }
    }
}

impl From<u128> for U256 {
    fn from(val: u128) -> Self {
        U256 { high: 0, low: val }
    }
}

impl Display for U256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ten = U256 { high: 0, low: 10 };
        let mut digits = Vec::new();
        let mut remainder = *self;

        while !remainder.is_zero() {
            digits.push(remainder % ten);
            remainder /= ten;
        }

        digits.reverse();

        for digit in digits {
            write!(f, "{}", digit)?;
        }

        Ok(())
    }
}

impl Num for U256 {
    type FromStrRadixErr = ParseIntError;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        todo!()
    }
}

type PositiveInteger = U256;

/// Field modulus
///
/// =115792089237316195423570985008687907853269984665640564039457584007908834671663
const N: U256 = U256 {
    high: 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF,
    low: 0xFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F,
};

/// Square of auxiliary modulus `R` for montgomery reduction `R_SQUARED ≡ R^2
/// mod N`
///
/// Auxiliary modulus is `R = 1<<256` which exceeds U256::MAX which is why it
/// isn't listed as a variable. The square `R^2` is useful for converting field
/// elements into montgomery form.
///
/// Value was calculated using Python
///
/// ```python
/// R = 2**256
/// N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
/// R_SQUARED = (R * R) % N
/// print(R_SQUARED)
/// ```
///
/// =18446752466076602529
const R_SQUARED: U256 = U256 {
    high: 0,
    low: 0x1000007A2000E90A1,
};

/// Integer `N'` in `[0, R − 1]` such that `N * N' ≡ −1 mod R`.
///
/// Initial calculated using Python (integers are unbounded in size in Python)
///
/// ```python
/// # Reference:
/// # https://www.geeksforgeeks.org/python-program-for-basic-and-extended-euclidean-algorithms-2/
/// def gcdExtended(a, b):
///     # Base Case
///     if a == 0:
///         return b, 0, 1
///     gcd,x1,y1 = gcdExtended(b%a, a)
///     # Update x and y using results of recursive call
///     x = y1 - (b // a) * x1
///     y = x1
///     return gcd, x, y
///
/// R = 2**256
/// N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
/// g, R_prime, N_prime = gcdExtended(R, N)
/// print(R - N_prime)
/// ```
///
/// =91248989341183975618893650062416139444822672217621753343178995607987479196977
const N_PRIME: U256 = U256 {
    high: 0xC9BD1905155383999C46C2C295F2B761,
    low: 0xBCB223FEDC24A059D838091DD2253531,
};

/// Represents a base field element for the secp256k1 prime field
///
/// Values are stored internally in Montgomery form.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BaseFelt(pub U256);

impl BaseFelt {
    pub fn new(value: U256) -> BaseFelt {
        // Convert to Montgomery form
        BaseFelt(mul(value, R_SQUARED))
    }
}

impl Felt for BaseFelt {
    type PositiveInteger = PositiveInteger;

    const ELEMENT_BYTES: usize = core::mem::size_of::<U256>();

    const FIELD_ORDER_BITS: u32 = 256;

    fn inverse(&self) -> Option<Self> {
        Some(BaseFelt(modular_inverse(self.0)?))
    }

    fn inverse_in_place(&mut self) -> Option<&mut Self> {
        match self.inverse() {
            Some(inverse) => {
                *self = inverse;
                Some(self)
            }
            None => None,
        }
    }

    fn double(&self) -> Self {
        if self.0 <= Self::MODULUS / 2u64 {
            BaseFelt(self.0 + self.0)
        } else {
            BaseFelt(self.0 - (Self::MODULUS - self.0))
        }
    }

    fn double_in_place(&mut self) -> &mut Self {
        *self = if self.0 <= Self::MODULUS / 2u64 {
            BaseFelt(self.0 + self.0)
        } else {
            BaseFelt(self.0 - (Self::MODULUS - self.0))
        };
        self
    }

    /// Converts internal value out of Montgomery form.
    fn as_integer(&self) -> Self::PositiveInteger {
        mul(U256::one(), self.0)
    }

    // TODO: find out if difference in performance if borrowed or owned self.
    fn pow(self, power: Self::PositiveInteger) -> Self {
        let mut res = Self::one();

        if power.is_zero() {
            return Self::one();
        } else if self.is_zero() {
            return Self::zero();
        }

        let mut power = power;
        let mut accumulator = self;

        while !power.is_zero() {
            if power & Self::PositiveInteger::one() == Self::PositiveInteger::one() {
                res *= accumulator;
            }
            power >>= Self::PositiveInteger::one();
            accumulator.square_in_place();
        }

        res
    }
}

impl Display for BaseFelt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_integer())
    }
}

impl From<u8> for BaseFelt {
    fn from(item: u8) -> Self {
        BaseFelt::new(item.into())
    }
}

impl From<u16> for BaseFelt {
    fn from(item: u16) -> Self {
        BaseFelt::new(item.into())
    }
}

impl From<u32> for BaseFelt {
    fn from(item: u32) -> Self {
        BaseFelt::new(item.into())
    }
}

impl From<u64> for BaseFelt {
    fn from(item: u64) -> Self {
        BaseFelt::new(item.into())
    }
}

impl From<u128> for BaseFelt {
    fn from(item: u128) -> Self {
        BaseFelt::new(item.into())
    }
}

impl From<usize> for BaseFelt {
    fn from(item: usize) -> Self {
        BaseFelt::new(item.try_into().unwrap())
    }
}

impl Sum for BaseFelt {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b).unwrap_or_else(Self::zero)
    }
}

impl<'a> Sum<&'a Self> for BaseFelt {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.copied()
            .reduce(|a, b| a + b)
            .unwrap_or_else(Self::zero)
    }
}

impl Product for BaseFelt {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a * b).unwrap_or_else(Self::one)
    }
}

impl<'a> Product<&'a Self> for BaseFelt {
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.copied().reduce(|a, b| a * b).unwrap_or_else(Self::one)
    }
}

impl StarkFelt for BaseFelt {
    // TODO: find value for this
    const GENERATOR: Self = BaseFelt(U256::ZERO);
    const TWO_ADICITY: u32 = 2;
    // TODO: find value for this
    const TWO_ADIC_ROOT_OF_UNITY: Self = BaseFelt(U256::ZERO);

    /// Returns a root of unity of order `2^n`.
    fn get_root_of_unity(n: u32) -> Self {
        assert_ne!(n, 0, "n must be greater than 0");
        assert!(
            n <= Self::TWO_ADICITY,
            "n must be less than {}",
            Self::TWO_ADICITY
        );
        let power = Self::PositiveInteger::one() << (Self::TWO_ADICITY - n);
        Self::TWO_ADIC_ROOT_OF_UNITY.pow(power)
    }
}

impl PrimeFelt for BaseFelt {
    const MODULUS: U256 = N;
}

impl One for BaseFelt {
    #[inline]
    fn one() -> Self {
        BaseFelt(mul(U256::one(), R_SQUARED))
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.0 == Self::one().0
    }
}

impl Zero for BaseFelt {
    #[inline]
    fn zero() -> Self {
        BaseFelt(U256::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == U256::zero()
    }
}

impl Neg for BaseFelt {
    type Output = Self;

    fn neg(self) -> Self::Output {
        BaseFelt(Self::MODULUS - self.0)
    }
}

impl AddAssign for BaseFelt {
    fn add_assign(&mut self, rhs: Self) {
        *self = BaseFelt(add(self.0, rhs.0))
    }
}

impl<'a> AddAssign<&'a BaseFelt> for BaseFelt {
    fn add_assign(&mut self, rhs: &Self) {
        *self = BaseFelt(add(self.0, rhs.0))
    }
}

impl Add for BaseFelt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        BaseFelt(add(self.0, rhs.0))
    }
}

impl<'a> Add<&'a BaseFelt> for BaseFelt {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        BaseFelt(add(self.0, rhs.0))
    }
}

impl SubAssign for BaseFelt {
    fn sub_assign(&mut self, rhs: Self) {
        *self = BaseFelt(sub(self.0, rhs.0))
    }
}

impl<'a> SubAssign<&'a BaseFelt> for BaseFelt {
    fn sub_assign(&mut self, rhs: &Self) {
        *self = BaseFelt(sub(self.0, rhs.0))
    }
}

impl Sub for BaseFelt {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        BaseFelt(sub(self.0, rhs.0))
    }
}

impl<'a> Sub<&'a BaseFelt> for BaseFelt {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        BaseFelt(sub(self.0, rhs.0))
    }
}

impl MulAssign for BaseFelt {
    fn mul_assign(&mut self, rhs: Self) {
        *self = BaseFelt(mul(self.0, rhs.0))
    }
}

impl<'a> MulAssign<&'a BaseFelt> for BaseFelt {
    fn mul_assign(&mut self, rhs: &Self) {
        *self = BaseFelt(mul(self.0, rhs.0))
    }
}

impl<'a> Mul<&'a BaseFelt> for BaseFelt {
    type Output = Self;

    fn mul(self, rhs: &Self) -> Self::Output {
        BaseFelt(mul(self.0, rhs.0))
    }
}

impl Mul for BaseFelt {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        BaseFelt(mul(self.0, rhs.0))
    }
}

impl DivAssign for BaseFelt {
    fn div_assign(&mut self, rhs: Self) {
        *self = BaseFelt(div(self.0, rhs.0))
    }
}

impl<'a> DivAssign<&'a BaseFelt> for BaseFelt {
    fn div_assign(&mut self, rhs: &Self) {
        *self = BaseFelt(div(self.0, rhs.0))
    }
}

impl<'a> Div<&'a BaseFelt> for BaseFelt {
    type Output = Self;

    fn div(self, rhs: &Self) -> Self::Output {
        BaseFelt(div(self.0, rhs.0))
    }
}

impl Div for BaseFelt {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        BaseFelt(div(self.0, rhs.0))
    }
}

/// Returns the multiplicative inverse in Montgomery form
///
/// `b` should be in Montgomery form. Uses binary extended GCD algorithm: https://ieeexplore.ieee.org/document/403725.
///
/// Returns None if `GCD(b, N) != 1` i.e. when `b = N` or `b = 0`.
#[inline(always)]
fn modular_inverse(b: U256) -> Option<U256> {
    let a = N;

    let mut u = a;
    let mut v = b;
    let mut r = U256::zero();
    let mut s = U256::one();
    let mut k = 0;

    // First phase
    while v > U256::zero() {
        if u & U256::one() == U256::zero() {
            u /= 2u64;
            s *= 2u64;
        } else if v & U256::one() == U256::zero() {
            v /= 2u64;
            r *= 2u64;
        } else if u > v {
            u = (u - v) / 2u64;
            r += s;
            s *= 2u64;
        } else {
            v = (v - u) / 2u64;
            s += r;

            // Can cause overflow. re-use add to reduce by the MODULUS
            r = add(r, r);
        }

        k += 1;
    }

    // GCD needs to be 1
    if u != U256::one() {
        return None;
    }

    if r >= a {
        r -= a;
    }

    // Second phase
    for _ in 0..(k - 256) {
        if r & U256::one() == U256::zero() {
            r /= 2u64;
        } else {
            // Non overflowing (r + a) / 2
            r = r / 2u64 + a / 2u64 + (U256::one() & a & r);
        }
    }

    Some(mul(a - r, R_SQUARED))
}

/// Computes `lhs * rhs mod M`
///
/// `lhs` and `rhs` are assumed to be in montgomery form. Reference:
/// - https://en.wikipedia.org/wiki/Montgomery_modular_multiplication (the REDC
///   algorithm)
/// - https://www.youtube.com/watch?v=2UmQDKcelBQ
#[inline(always)]
fn mul(lhs: U256, rhs: U256) -> U256 {
    // let half_bits = U256 { high: 0, low: 128 };
    let lhs_low = U256 {
        high: 0,
        low: lhs.low,
    };
    let lhs_high = U256 {
        high: 0,
        low: lhs.high,
    };
    let rhs_low = U256 {
        high: 0,
        low: rhs.low,
    };
    let rhs_high = U256 {
        high: 0,
        low: rhs.high,
    };

    let partial_t_high = lhs_high.mul(rhs_high);
    let partial_t_mid_a = lhs_high.mul(rhs_low);
    let partial_t_mid_a_low = U256 {
        high: 0,
        low: partial_t_mid_a.low,
    };
    let partial_t_mid_a_high = U256 {
        high: 0,
        low: partial_t_mid_a.high,
    };
    let partial_t_mid_b = rhs_high.mul(lhs_low);
    let partial_t_mid_b_low = U256 {
        high: 0,
        low: partial_t_mid_b.low,
    };
    let partial_t_mid_b_high = U256 {
        high: 0,
        low: partial_t_mid_b.high,
    };
    let partial_t_low = lhs_low.mul(rhs_low);

    // TODO: potential opportunity to optimize
    let tmp = partial_t_mid_a_low.add(partial_t_mid_b_low).add(U256 {
        high: 0,
        low: partial_t_low.high,
    });
    let carry = U256 {
        high: 0,
        low: tmp.high,
    };
    let t_low = U256 {
        high: tmp.low,
        low: partial_t_low.low,
    };
    let t_high = partial_t_high
        .add(partial_t_mid_a_high)
        .add(partial_t_mid_b_high)
        .add(carry);

    println!("t_low: {:?}", t_low);
    println!("N_PRIME: {:?}", N_PRIME);
    // Compute `m = T * N' mod R`
    let m = t_low.mul(N_PRIME); // overflowing mult
    println!("m: {:?}", m);

    // Compute `t = (T + m * N) / R`
    let n_low = U256 {
        high: 0,
        low: N.low,
    };
    let n_high = U256 {
        high: 0,
        low: N.high,
    };
    let m_low = U256 {
        high: 0,
        low: m.low,
    };
    let m_high = U256 {
        high: 0,
        low: m.high,
    };

    let partial_mn_high = m_high.mul(n_high);
    let partial_mn_mid_a = m_high.mul(n_low);
    let partial_mn_mid_a_low = U256 {
        high: 0,
        low: partial_mn_mid_a.low,
    };
    let partial_mn_mid_a_high = U256 {
        high: 0,
        low: partial_mn_mid_a.high,
    };
    let partial_mn_mid_b = n_high.mul(m_low);
    let partial_mn_mid_b_low = U256 {
        high: 0,
        low: partial_mn_mid_b.low,
    };
    let partial_mn_mid_b_high = U256 {
        high: 0,
        low: partial_mn_mid_b.high,
    };
    let partial_mn_low = m_low.mul(n_low);

    let tmp = partial_mn_mid_a_low.add(partial_mn_mid_b_low).add(U256 {
        high: 0,
        low: partial_mn_low.high,
    });
    let carry = U256 {
        high: 0,
        low: tmp.high,
    };
    let mn_low = U256 {
        high: tmp.low,
        low: partial_mn_low.low,
    };
    let mn_high = partial_mn_high
        .add(partial_mn_mid_a_high)
        .add(partial_mn_mid_b_high)
        .add(carry);

    let (_, overflow) = U256::overflowing_add(mn_low, t_low);
    let (t, overflows_r) = U256::overflowing_add(
        t_high.add(U256 {
            high: 0,
            low: overflow as u128,
        }),
        mn_high,
    );

    let overflows_r = U256 {
        high: 0,
        low: overflows_r as u128,
    };
    let overflows_modulus = U256 {
        high: 0,
        low: (t.gt(N) || t.eq(N)) as u128,
    };

    // TODO: overflows_r * 0u128.wrapping_sub(N) might need to be within the
    // overflows_modulus check above.
    t.add(overflows_r.mul(U256::ZERO.sub(N)))
        .sub(overflows_modulus.mul(N))
}

/// Computes `lhs + rhs mod N`
#[inline(always)]
const fn add(lhs: U256, rhs: U256) -> U256 {
    let (addition, overflows_r) = U256::overflowing_add(lhs, rhs);
    let overflows_r = U256 {
        high: 0,
        low: overflows_r as u128,
    };
    let exceeds_modulus = U256 {
        high: 0,
        low: (addition.gt(N) || addition.eq(N)) as u128,
    };
    addition
        .sub(exceeds_modulus.mul(N))
        .add(overflows_r.mul(U256::ZERO.sub(N)))
}

/// Computes `lhs - rhs mod N`
#[inline(always)]
fn sub(lhs: U256, rhs: U256) -> U256 {
    add(lhs, BaseFelt::MODULUS - rhs)
}

/// Computes `lhs / rhs mod N`
#[inline(always)]
fn div(lhs: U256, rhs: U256) -> U256 {
    mul(lhs, modular_inverse(rhs).unwrap())
}

#[cfg(test)]
mod tests {
    use super::super::batch_inverse;
    use super::*;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_pcg::Pcg64;
    use test::Bencher;

    #[test]
    fn adds_small_numbers() {
        let a = BaseFelt::new(5u32.into());
        assert_eq!((a + a).as_integer(), 10u32.into());
    }

    #[test]
    fn adds_large_numbers() {
        let a = BaseFelt::new(N - 1u32);
        assert_eq!((a + a).as_integer(), N - 2u32);
    }

    #[test]
    fn test_batch_inverses() {
        let values = vec![
            BaseFelt::new(1u32.into()),
            BaseFelt::new(2u32.into()),
            BaseFelt::new(3u32.into()),
        ];

        let batch_inversed = batch_inverse(&values);

        assert_eq!(
            batch_inversed[0].unwrap().as_integer(),
            values[0].inverse().unwrap().as_integer()
        );
        assert_eq!(
            batch_inversed[1].unwrap().as_integer(),
            values[1].inverse().unwrap().as_integer()
        );
        assert_eq!(
            batch_inversed[2].unwrap().as_integer(),
            values[2].inverse().unwrap().as_integer()
        );
    }

    #[test]
    fn multiplication_multiplies_large_numbers() {
        // Large compared to u128::MAX
        let a = BaseFelt::new(BaseFelt::MODULUS - 2u32);

        assert_eq!((a * a).as_integer(), 4u32.into());
    }

    #[test]
    fn multiplication_multiplies_odd_numbers() {
        // Large compared to u128::MAX
        let a = BaseFelt::new(BaseFelt::MODULUS - 1u32);
        let b = BaseFelt::new(BaseFelt::MODULUS - 2u32);

        assert_eq!((a * b).as_integer(), 2u32.into());
    }

    #[test]
    fn subtraction_subtracts() {
        let a = BaseFelt::new(1u32.into());
        let b = BaseFelt::new(2u32.into());

        assert_eq!((a - b).as_integer(), BaseFelt::MODULUS - 1u32);
    }

    #[test]
    fn division_divides() {
        let two = BaseFelt::new(2u32.into());

        assert_eq!((two / two).as_integer(), 1u32.into());
    }

    #[bench]
    fn bench_inverse_1000_items(b: &mut Bencher) {
        let mut rng = Pcg64::seed_from_u64(42);
        let items = (0..1000)
            .map(|_| BaseFelt::new(rng.gen()))
            .collect::<Vec<BaseFelt>>();

        b.iter(|| items.iter().map(|item| item.inverse()).collect::<Vec<_>>())
    }

    #[bench]
    fn bench_batch_inverse_1000_items(b: &mut Bencher) {
        let mut rng = Pcg64::seed_from_u64(42);
        let items = (0..1000)
            .map(|_| BaseFelt::new(rng.gen()))
            .collect::<Vec<BaseFelt>>();

        b.iter(|| batch_inverse(&items))
    }

    #[bench]
    fn bench_sum_20000_items(b: &mut Bencher) {
        let mut rng = Pcg64::seed_from_u64(42);
        let items = (0..20000)
            .map(|_| BaseFelt::new(rng.gen()))
            .collect::<Vec<BaseFelt>>();

        b.iter(|| items.iter().sum::<BaseFelt>())
    }

    #[bench]
    fn bench_product_1000_items(b: &mut Bencher) {
        let mut rng = Pcg64::seed_from_u64(42);
        let items = (0..1000)
            .map(|_| BaseFelt::new(rng.gen()))
            .collect::<Vec<BaseFelt>>();

        b.iter(|| items.iter().product::<BaseFelt>());

        println!("{}", items.iter().product::<BaseFelt>().as_integer());
    }

    #[test]
    fn u256_adds() {
        let a = U256 { high: 0, low: 10 };
        let b = U256 { high: 0, low: 10 };

        assert_eq!(a + b, 20u32.into());
    }

    #[test]
    fn u256_overflowing_add() {
        let a = U256 {
            high: u128::MAX,
            low: u128::MAX,
        };
        let b = U256 { high: 1, low: 10 };

        assert_eq!(a + b, U256 { high: 1, low: 9 });
    }

    #[test]
    fn u256_overflowing_mul() {
        assert_eq!(
            U256::ZERO.sub(N),
            U256 {
                high: 0,
                low: 0x1000003d1
            }
        );
    }

    #[test]
    fn u256_displays_correctly() {
        assert_eq!(format!("{}", N), "test");
    }

    #[test]
    fn u256_sub() {
        assert_eq!(U256::zero() - U256::one(), U256::MAX);
    }

    #[test]
    fn converts_to_montgomery_form() {
        println!("{:?}", BaseFelt::new(U256 { high: 1, low: 2 }));
        println!("{:?}", BaseFelt::new(U256 { high: 0, low: 4 }));
        assert_eq!(
            BaseFelt::new(U256 { high: 0, low: 2 }).as_integer(),
            U256::one()
        );
    }

    #[test]
    fn overflowing_add() {
        let a = U256::MAX;
        let b = U256::one();
        assert_eq!(a.overflowing_add(b), (U256::zero(), true));
    }

    #[test]
    fn u128_overflowing_add() {
        let a = U256 {
            high: 0,
            low: u128::MAX,
        };
        let b = U256::one();
        assert_eq!(a.overflowing_add(b), (U256 { high: 1, low: 0 }, false));
    }

    #[test]
    fn u128_multiply() {
        let a = U256 {
            high: 0,
            low: u128::MAX,
        };
        assert_eq!(
            a.mul(a),
            U256 {
                high: 0xfffffffffffffffffffffffffffffffe,
                low: 0x00000000000000000000000000000001
            }
        );
    }
}
