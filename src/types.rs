use num_traits::{
    FromPrimitive, Num, NumAssign, One, ToPrimitive, Unsigned, WrappingAdd, WrappingMul,
    WrappingShl, WrappingShr, WrappingSub, Zero,
};
use std::num::Wrapping;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

#[allow(non_camel_case_types)]
pub type u8w = Wrapping<u8>;
#[allow(non_camel_case_types)]
pub type u16w = Wrapping<u16>;
#[allow(non_camel_case_types)]
pub type u32w = Wrapping<u32>;
#[allow(non_camel_case_types)]
pub type u64w = Wrapping<u64>;

pub trait HardwareInteger:
    Sized
    + Clone
    + Copy
    + Num
    + NumAssign
    + FromPrimitive
    + ToPrimitive
    + Eq
    + PartialOrd
    + Ord
    + Unsigned
    + WrappingAdd
    + WrappingSub
    + WrappingMul
    + WrappingShl
    + WrappingShr
    + Not
    + BitAnd
    + BitOr
    + BitXor
    + BitAndAssign
    + BitOrAssign
    + BitXorAssign
{
}
impl HardwareInteger for u8w {}
impl HardwareInteger for u16w {}
impl HardwareInteger for u32w {}
impl HardwareInteger for u64w {}

macro_rules! define_unsigned {
    ($name:ident, $bits:expr, $type:ident) => {
        #[allow(non_camel_case_types)]
        #[derive(Default, Clone, Copy, Debug)]
        pub struct $name(pub $type);

        impl $name {
            pub const MAX: Self = $name(((1 as $type) << $bits) - 1);
            pub const MIN: Self = $name(0);
            pub const ZERO: Self = $name(0);
            pub const ONE: Self = $name(1 as $type);

            fn mask(self) -> Self {
                $name(self.0 & (((1 as $type) << $bits).overflowing_sub(1).0))
            }
        }

        implement_common!($name, $bits, $type);

        impl Unsigned for $name {}
    };
}

macro_rules! implement_common {
    ($name:ident, $bits:expr, $type:ident) => {
        impl $name {
            /// Returns the smallest value that can be represented by this integer type.
            pub const fn min_value() -> $name {
                $name::MIN
            }
            /// Returns the largest value that can be represented by this integer type.
            pub const fn max_value() -> $name {
                $name::MAX
            }

            /// This function mainly exists as there is currently not a better way to construct these types.
            /// May be deprecated or removed if a better way to construct these types becomes available.
            pub const fn new(value: $type) -> $name {
                assert!(value <= $name::MAX.0 && value >= $name::MIN.0);
                $name(value)
            }

            /// Wrapping right shift. Computes `self >> other`, without panicing.
            pub fn wrapping_shr(self, rhs: u32) -> Self {
                $name(self.0.wrapping_shr(rhs)).mask()
            }

            /// Wrapping left shift. Computes `self << other`, without panicing.
            pub fn wrapping_shl(self, rhs: u32) -> Self {
                $name(self.0.wrapping_shl(rhs)).mask()
            }

            /// Wrapping (modular) addition. Computes `self + other`,
            /// wrapping around at the boundary of the type.
            pub fn wrapping_add(self, rhs: Self) -> Self {
                $name(self.0.wrapping_add(rhs.0)).mask()
            }

            /// Wrapping (modular) subtraction. Computes `self - other`,
            /// wrapping around at the boundary of the type.
            pub fn wrapping_sub(self, rhs: Self) -> Self {
                $name(self.0.wrapping_sub(rhs.0)).mask()
            }

            /// Wrapping (modular) multiplication. Computes `self * other`,
            /// wrapping around at the boundary of the type.
            pub fn wrapping_mul(self, rhs: Self) -> Self {
                $name(self.0.wrapping_mul(rhs.0)).mask()
            }

            /// Wrapping (modular) division. Computes `self / other`,
            /// wrapping around at the boundary of the type.
            pub fn wrapping_div(self, rhs: Self) -> Self {
                $name(self.0.wrapping_div(rhs.0)).mask()
            }

            /// Wrapping (modular) remainder. Computes `self % other`,
            /// wrapping around at the boundary of the type.
            pub fn wrapping_rem(self, rhs: Self) -> Self {
                $name(self.0.wrapping_rem(rhs.0)).mask()
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.mask().0 == other.mask().0
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &$name) -> Option<std::cmp::Ordering> {
                self.mask().0.partial_cmp(&other.mask().0)
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &$name) -> std::cmp::Ordering {
                self.mask().0.cmp(&other.mask().0)
            }
        }

        impl PartialEq<$type> for $name {
            fn eq(&self, other: &$type) -> bool {
                self.mask().0.eq(other)
            }
        }

        impl PartialOrd<$type> for $name {
            fn partial_cmp(&self, other: &$type) -> Option<std::cmp::Ordering> {
                self.mask().0.partial_cmp(other)
            }
        }

        impl std::hash::Hash for $name {
            fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
                self.mask().0.hash(h)
            }
        }

        // Implement num-traits
        impl Zero for $name {
            fn zero() -> Self {
                Self::ZERO
            }

            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }
        }
        impl One for $name {
            fn one() -> Self {
                Self::ONE
            }
        }
        impl Num for $name {
            type FromStrRadixErr = <$type as Num>::FromStrRadixErr;

            fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
                <$type as Num>::from_str_radix(str, radix).map(|value| $name::new(value))
            }
        }
        impl FromPrimitive for $name {
            fn from_i8(n: i8) -> Option<Self> {
                $type::from_i8(n).map(|value| $name::new(value))
            }
            fn from_i16(n: i16) -> Option<Self> {
                $type::from_i16(n).map(|value| $name::new(value))
            }
            fn from_i32(n: i32) -> Option<Self> {
                $type::from_i32(n).map(|value| $name::new(value))
            }
            fn from_i64(n: i64) -> Option<Self> {
                $type::from_i64(n).map(|value| $name::new(value))
            }
            fn from_isize(n: isize) -> Option<Self> {
                $type::from_isize(n).map(|value| $name::new(value))
            }

            fn from_u8(n: u8) -> Option<Self> {
                $type::from_u8(n).map(|value| $name::new(value))
            }
            fn from_u16(n: u16) -> Option<Self> {
                $type::from_u16(n).map(|value| $name::new(value))
            }
            fn from_u32(n: u32) -> Option<Self> {
                $type::from_u32(n).map(|value| $name::new(value))
            }
            fn from_u64(n: u64) -> Option<Self> {
                $type::from_u64(n).map(|value| $name::new(value))
            }
            fn from_usize(n: usize) -> Option<Self> {
                $type::from_usize(n).map(|value| $name::new(value))
            }

            fn from_f32(n: f32) -> Option<Self> {
                $type::from_f32(n).map(|value| $name::new(value))
            }
            fn from_f64(n: f64) -> Option<Self> {
                $type::from_f64(n).map(|value| $name::new(value))
            }
        }
        impl ToPrimitive for $name {
            fn to_i8(&self) -> Option<i8> {
                self.0.to_i8()
            }
            fn to_i16(&self) -> Option<i16> {
                self.0.to_i16()
            }
            fn to_i32(&self) -> Option<i32> {
                self.0.to_i32()
            }
            fn to_i64(&self) -> Option<i64> {
                self.0.to_i64()
            }
            fn to_isize(&self) -> Option<isize> {
                self.0.to_isize()
            }

            fn to_u8(&self) -> Option<u8> {
                self.0.to_u8()
            }
            fn to_u16(&self) -> Option<u16> {
                self.0.to_u16()
            }
            fn to_u32(&self) -> Option<u32> {
                self.0.to_u32()
            }
            fn to_u64(&self) -> Option<u64> {
                self.0.to_u64()
            }
            fn to_usize(&self) -> Option<usize> {
                self.0.to_usize()
            }

            fn to_f32(&self) -> Option<f32> {
                self.0.to_f32()
            }
            fn to_f64(&self) -> Option<f64> {
                self.0.to_f64()
            }
        }

        // Implement formating
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::Display>::fmt(value, f)
            }
        }
        impl std::fmt::UpperHex for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::UpperHex>::fmt(value, f)
            }
        }
        impl std::fmt::LowerHex for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::LowerHex>::fmt(value, f)
            }
        }
        impl std::fmt::Octal for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::Octal>::fmt(value, f)
            }
        }
        impl std::fmt::Binary for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::Binary>::fmt(value, f)
            }
        }

        impl<T> Shr<T> for $name
        where
            $type: Shr<T, Output = $type>,
        {
            type Output = $name;

            fn shr(self, rhs: T) -> $name {
                $name(self.mask().0.shr(rhs))
            }
        }

        impl<T> Shl<T> for $name
        where
            $type: Shl<T, Output = $type>,
        {
            type Output = $name;

            fn shl(self, rhs: T) -> $name {
                $name(self.mask().0.shl(rhs))
            }
        }

        impl<T> ShrAssign<T> for $name
        where
            $type: ShrAssign<T>,
        {
            fn shr_assign(&mut self, rhs: T) {
                *self = self.mask();
                self.0.shr_assign(rhs);
            }
        }

        impl<T> ShlAssign<T> for $name
        where
            $type: ShlAssign<T>,
        {
            fn shl_assign(&mut self, rhs: T) {
                *self = self.mask();
                self.0.shl_assign(rhs);
            }
        }

        impl BitOr<$name> for $name {
            type Output = $name;

            fn bitor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl<'a> BitOr<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl<'a> BitOr<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl<'a> BitOr<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl BitOrAssign<$name> for $name {
            fn bitor_assign(&mut self, other: $name) {
                *self = self.mask();
                self.0.bitor_assign(other.mask().0)
            }
        }

        impl BitXor<$name> for $name {
            type Output = $name;

            fn bitxor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl<'a> BitXor<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl<'a> BitXor<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl<'a> BitXor<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl BitXorAssign<$name> for $name {
            fn bitxor_assign(&mut self, other: $name) {
                *self = self.mask();
                self.0.bitxor_assign(other.mask().0)
            }
        }

        impl Not for $name {
            type Output = $name;

            fn not(self) -> $name {
                $name(self.mask().0.not())
            }
        }

        impl<'a> Not for &'a $name {
            type Output = <$name as Not>::Output;

            fn not(self) -> $name {
                $name(self.mask().0.not())
            }
        }

        impl BitAnd<$name> for $name {
            type Output = $name;

            fn bitand(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl<'a> BitAnd<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl<'a> BitAnd<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl<'a> BitAnd<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl BitAndAssign<$name> for $name {
            fn bitand_assign(&mut self, other: $name) {
                *self = self.mask();
                self.0.bitand_assign(other.mask().0)
            }
        }

        impl Add<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn add(self, other: $name) -> $name {
                if self.0 > 0 && other.0 > 0 {
                    debug_assert!(Self::MAX.0 - other.0 >= self.0);
                } else if self.0 < 0 && other.0 < 0 {
                    debug_assert!(Self::MIN.0 - other.0 <= self.0);
                }
                self.wrapping_add(other)
            }
        }

        impl AddAssign<$name> for $name {
            fn add_assign(&mut self, rhs: $name) {
                *self = self.add(rhs);
            }
        }

        impl Sub<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn sub(self, other: $name) -> $name {
                if self > other {
                    debug_assert!(Self::MAX.0 + other.0 >= self.0);
                } else if self < other {
                    debug_assert!(Self::MIN.0 + other.0 <= self.0);
                }
                self.wrapping_sub(other)
            }
        }

        impl SubAssign<$name> for $name {
            fn sub_assign(&mut self, rhs: $name) {
                *self = self.sub(rhs);
            }
        }

        impl Mul<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn mul(self, other: $name) -> $name {
                debug_assert!(self.0 * other.0 <= Self::MAX.0);
                debug_assert!(self.0 * other.0 >= Self::MIN.0);
                self.wrapping_mul(other)
            }
        }

        impl MulAssign<$name> for $name {
            fn mul_assign(&mut self, rhs: $name) {
                *self = self.mul(rhs);
            }
        }

        impl Div<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn div(self, other: $name) -> $name {
                debug_assert!(self.0 / other.0 <= Self::MAX.0);
                debug_assert!(self.0 / other.0 >= Self::MIN.0);
                self.wrapping_div(other)
            }
        }

        impl DivAssign<$name> for $name {
            fn div_assign(&mut self, rhs: $name) {
                *self = self.div(rhs);
            }
        }

        impl Rem<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn rem(self, other: $name) -> $name {
                debug_assert!(self.0 % other.0 <= Self::MAX.0);
                debug_assert!(self.0 % other.0 >= Self::MIN.0);
                self.wrapping_rem(other)
            }
        }

        impl RemAssign<$name> for $name {
            fn rem_assign(&mut self, rhs: $name) {
                *self = self.rem(rhs);
            }
        }

        impl BitOr<$type> for $name {
            type Output = $name;

            fn bitor(self, rhs: $type) -> Self::Output {
                $name(self.mask().0.bitor(rhs))
            }
        }

        impl<'a> BitOr<&'a $type> for $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitor(self, rhs: &'a $type) -> Self::Output {
                $name(self.mask().0.bitor(rhs))
            }
        }

        impl<'a> BitOr<$type> for &'a $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitor(self, rhs: $type) -> Self::Output {
                $name(self.mask().0.bitor(rhs))
            }
        }

        impl<'a> BitOr<&'a $type> for &'a $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitor(self, rhs: &'a $type) -> Self::Output {
                $name(self.mask().0.bitor(rhs))
            }
        }

        impl BitOrAssign<$type> for $name {
            fn bitor_assign(&mut self, other: $type) {
                *self = self.mask();
                self.0.bitor_assign(other)
            }
        }

        impl BitXor<$type> for $name {
            type Output = $name;

            fn bitxor(self, rhs: $type) -> Self::Output {
                $name(self.mask().0.bitxor(rhs))
            }
        }

        impl<'a> BitXor<&'a $type> for $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitxor(self, rhs: &'a $type) -> Self::Output {
                $name(self.mask().0.bitxor(rhs))
            }
        }

        impl<'a> BitXor<$type> for &'a $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitxor(self, rhs: $type) -> Self::Output {
                $name(self.mask().0.bitxor(rhs))
            }
        }

        impl<'a> BitXor<&'a $type> for &'a $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitxor(self, rhs: &'a $type) -> Self::Output {
                $name(self.mask().0.bitxor(rhs))
            }
        }

        impl BitXorAssign<$type> for $name {
            fn bitxor_assign(&mut self, other: $type) {
                *self = self.mask();
                self.0.bitxor_assign(other)
            }
        }

        impl BitAnd<$type> for $name {
            type Output = $name;

            fn bitand(self, rhs: $type) -> Self::Output {
                $name(self.mask().0.bitand(rhs))
            }
        }

        impl<'a> BitAnd<&'a $type> for $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitand(self, rhs: &'a $type) -> Self::Output {
                $name(self.mask().0.bitand(rhs))
            }
        }

        impl<'a> BitAnd<$type> for &'a $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitand(self, rhs: $type) -> Self::Output {
                $name(self.mask().0.bitand(rhs))
            }
        }

        impl<'a> BitAnd<&'a $type> for &'a $name {
            type Output = <$name as BitOr<$type>>::Output;

            fn bitand(self, rhs: &'a $type) -> Self::Output {
                $name(self.mask().0.bitand(rhs))
            }
        }

        impl BitAndAssign<$type> for $name {
            fn bitand_assign(&mut self, other: $type) {
                *self = self.mask();
                self.0.bitand_assign(other)
            }
        }

        impl Add<$type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn add(self, other: $type) -> $name {
                if self.0 > 0 && other > 0 {
                    debug_assert!(Self::MAX.0 - other >= self.0);
                } else if self.0 < 0 && other < 0 {
                    debug_assert!(Self::MIN.0 - other <= self.0);
                }
                self.wrapping_add($name::new(other))
            }
        }

        impl AddAssign<$type> for $name {
            fn add_assign(&mut self, rhs: $type) {
                *self = self.add(rhs);
            }
        }

        impl Sub<$type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn sub(self, other: $type) -> $name {
                if self.0 > other {
                    debug_assert!(Self::MAX.0 + other >= self.0);
                } else if self.0 < other {
                    debug_assert!(Self::MIN.0 + other <= self.0);
                }
                self.wrapping_sub($name::new(other))
            }
        }

        impl SubAssign<$type> for $name {
            fn sub_assign(&mut self, rhs: $type) {
                *self = self.sub(rhs);
            }
        }

        impl Mul<$type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn mul(self, other: $type) -> $name {
                debug_assert!(self.0 * other <= Self::MAX.0);
                debug_assert!(self.0 * other >= Self::MIN.0);
                self.wrapping_mul($name::new(other))
            }
        }

        impl MulAssign<$type> for $name {
            fn mul_assign(&mut self, rhs: $type) {
                *self = self.mul(rhs);
            }
        }

        impl Div<$type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn div(self, other: $type) -> $name {
                self.wrapping_div($name::new(other))
            }
        }

        impl DivAssign<$type> for $name {
            fn div_assign(&mut self, rhs: $type) {
                *self = self.div(rhs);
            }
        }

        impl Rem<$type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn rem(self, other: $type) -> $name {
                self.wrapping_rem($name::new(other))
            }
        }

        impl RemAssign<$type> for $name {
            fn rem_assign(&mut self, rhs: $type) {
                *self = self.rem(rhs);
            }
        }
    };
}

macro_rules! define_wrapping {
    ($name:ident, $type:ident, $base_type:ident) => {
        #[allow(non_camel_case_types)]
        #[derive(Default, Clone, Copy, Debug)]
        pub struct $name(pub $type);

        impl $name {
            pub const MAX: Self = $name($type::MAX);
            pub const MIN: Self = $name($type::MIN);
            pub const ZERO: Self = $name($type::ZERO);
            pub const ONE: Self = $name($type::ONE);

            /// This function mainly exists as there is currently not a better way to construct these types.
            /// May be deprecated or removed if a better way to construct these types becomes available.
            pub const fn new(value: $base_type) -> $name {
                $name($type(value))
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &$name) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&other.0)
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &$name) -> std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl PartialEq<$base_type> for $name {
            fn eq(&self, other: &$base_type) -> bool {
                self.0.eq(other)
            }
        }

        impl PartialOrd<$base_type> for $name {
            fn partial_cmp(&self, other: &$base_type) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(other)
            }
        }

        impl std::hash::Hash for $name {
            fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
                self.0.hash(h)
            }
        }

        // Implement num-traits
        impl Zero for $name {
            fn zero() -> Self {
                Self::ZERO
            }

            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }
        }
        impl One for $name {
            fn one() -> Self {
                Self::ONE
            }
        }
        impl Num for $name {
            type FromStrRadixErr = <$type as Num>::FromStrRadixErr;

            fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
                <$type as Num>::from_str_radix(str, radix).map(|value| $name(value))
            }
        }
        impl FromPrimitive for $name {
            fn from_i8(n: i8) -> Option<Self> {
                $type::from_i8(n).map(|value| $name(value))
            }
            fn from_i16(n: i16) -> Option<Self> {
                $type::from_i16(n).map(|value| $name(value))
            }
            fn from_i32(n: i32) -> Option<Self> {
                $type::from_i32(n).map(|value| $name(value))
            }
            fn from_i64(n: i64) -> Option<Self> {
                $type::from_i64(n).map(|value| $name(value))
            }
            fn from_isize(n: isize) -> Option<Self> {
                $type::from_isize(n).map(|value| $name(value))
            }

            fn from_u8(n: u8) -> Option<Self> {
                $type::from_u8(n).map(|value| $name(value))
            }
            fn from_u16(n: u16) -> Option<Self> {
                $type::from_u16(n).map(|value| $name(value))
            }
            fn from_u32(n: u32) -> Option<Self> {
                $type::from_u32(n).map(|value| $name(value))
            }
            fn from_u64(n: u64) -> Option<Self> {
                $type::from_u64(n).map(|value| $name(value))
            }
            fn from_usize(n: usize) -> Option<Self> {
                $type::from_usize(n).map(|value| $name(value))
            }

            fn from_f32(n: f32) -> Option<Self> {
                $type::from_f32(n).map(|value| $name(value))
            }
            fn from_f64(n: f64) -> Option<Self> {
                $type::from_f64(n).map(|value| $name(value))
            }
        }
        impl ToPrimitive for $name {
            fn to_i8(&self) -> Option<i8> {
                self.0.to_i8()
            }
            fn to_i16(&self) -> Option<i16> {
                self.0.to_i16()
            }
            fn to_i32(&self) -> Option<i32> {
                self.0.to_i32()
            }
            fn to_i64(&self) -> Option<i64> {
                self.0.to_i64()
            }
            fn to_isize(&self) -> Option<isize> {
                self.0.to_isize()
            }

            fn to_u8(&self) -> Option<u8> {
                self.0.to_u8()
            }
            fn to_u16(&self) -> Option<u16> {
                self.0.to_u16()
            }
            fn to_u32(&self) -> Option<u32> {
                self.0.to_u32()
            }
            fn to_u64(&self) -> Option<u64> {
                self.0.to_u64()
            }
            fn to_usize(&self) -> Option<usize> {
                self.0.to_usize()
            }

            fn to_f32(&self) -> Option<f32> {
                self.0.to_f32()
            }
            fn to_f64(&self) -> Option<f64> {
                self.0.to_f64()
            }
        }

        // Implement formating
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::Display>::fmt(value, f)
            }
        }
        impl std::fmt::UpperHex for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::UpperHex>::fmt(value, f)
            }
        }
        impl std::fmt::LowerHex for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::LowerHex>::fmt(value, f)
            }
        }
        impl std::fmt::Octal for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::Octal>::fmt(value, f)
            }
        }
        impl std::fmt::Binary for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as std::fmt::Binary>::fmt(value, f)
            }
        }

        impl Shr<usize> for $name {
            type Output = $name;

            fn shr(self, rhs: usize) -> $name {
                $name(self.0.wrapping_shr(rhs as u32))
            }
        }
        impl Shr<u32> for $name {
            type Output = $name;

            fn shr(self, rhs: u32) -> $name {
                $name(self.0.wrapping_shr(rhs))
            }
        }
        impl WrappingShr for $name {
            fn wrapping_shr(&self, rhs: u32) -> Self {
                self.shr(rhs)
            }
        }

        impl Shl<usize> for $name {
            type Output = $name;

            fn shl(self, rhs: usize) -> $name {
                $name(self.0.wrapping_shl(rhs as u32))
            }
        }
        impl Shl<u32> for $name {
            type Output = $name;

            fn shl(self, rhs: u32) -> $name {
                $name(self.0.wrapping_shl(rhs))
            }
        }
        impl WrappingShl for $name {
            fn wrapping_shl(&self, rhs: u32) -> Self {
                self.shl(rhs)
            }
        }

        impl ShrAssign<usize> for $name {
            fn shr_assign(&mut self, rhs: usize) {
                *self = $name(self.0.wrapping_shr(rhs as u32));
            }
        }
        impl ShrAssign<u32> for $name {
            fn shr_assign(&mut self, rhs: u32) {
                *self = $name(self.0.wrapping_shr(rhs));
            }
        }

        impl ShlAssign<usize> for $name {
            fn shl_assign(&mut self, rhs: usize) {
                *self = $name(self.0.wrapping_shl(rhs as u32));
            }
        }
        impl ShlAssign<u32> for $name {
            fn shl_assign(&mut self, rhs: u32) {
                *self = $name(self.0.wrapping_shl(rhs));
            }
        }

        impl BitOr<$name> for $name {
            type Output = $name;

            fn bitor(self, rhs: $name) -> Self::Output {
                $name(self.0.bitor(rhs.0))
            }
        }

        impl<'a> BitOr<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: &'a $name) -> Self::Output {
                $name(self.0.bitor(rhs.0))
            }
        }

        impl<'a> BitOr<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: $name) -> Self::Output {
                $name(self.0.bitor(rhs.0))
            }
        }

        impl<'a> BitOr<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: &'a $name) -> Self::Output {
                $name(self.0.bitor(rhs.0))
            }
        }

        impl BitOrAssign<$name> for $name {
            fn bitor_assign(&mut self, other: $name) {
                self.0.bitor_assign(other.0)
            }
        }

        impl BitXor<$name> for $name {
            type Output = $name;

            fn bitxor(self, rhs: $name) -> Self::Output {
                $name(self.0.bitxor(rhs.0))
            }
        }

        impl<'a> BitXor<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: &'a $name) -> Self::Output {
                $name(self.0.bitxor(rhs.0))
            }
        }

        impl<'a> BitXor<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: $name) -> Self::Output {
                $name(self.0.bitxor(rhs.0))
            }
        }

        impl<'a> BitXor<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: &'a $name) -> Self::Output {
                $name(self.0.bitxor(rhs.0))
            }
        }

        impl BitXorAssign<$name> for $name {
            fn bitxor_assign(&mut self, other: $name) {
                self.0.bitxor_assign(other.0)
            }
        }

        impl Not for $name {
            type Output = $name;

            fn not(self) -> $name {
                $name(self.0.not())
            }
        }

        impl<'a> Not for &'a $name {
            type Output = <$name as Not>::Output;

            fn not(self) -> $name {
                $name(self.0.not())
            }
        }

        impl BitAnd<$name> for $name {
            type Output = $name;

            fn bitand(self, rhs: $name) -> Self::Output {
                $name(self.0.bitand(rhs.0))
            }
        }

        impl<'a> BitAnd<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: &'a $name) -> Self::Output {
                $name(self.0.bitand(rhs.0))
            }
        }

        impl<'a> BitAnd<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: $name) -> Self::Output {
                $name(self.0.bitand(rhs.0))
            }
        }

        impl<'a> BitAnd<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: &'a $name) -> Self::Output {
                $name(self.0.bitand(rhs.0))
            }
        }

        impl BitAndAssign<$name> for $name {
            fn bitand_assign(&mut self, other: $name) {
                self.0.bitand_assign(other.0)
            }
        }

        impl Add<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn add(self, other: $name) -> $name {
                $name(self.0.wrapping_add(other.0))
            }
        }
        impl WrappingAdd for $name {
            fn wrapping_add(&self, other: &$name) -> $name {
                self.add(*other)
            }
        }

        impl AddAssign<$name> for $name {
            fn add_assign(&mut self, rhs: $name) {
                *self = self.add(rhs);
            }
        }

        impl Sub<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn sub(self, other: $name) -> $name {
                $name(self.0.wrapping_sub(other.0))
            }
        }
        impl WrappingSub for $name {
            fn wrapping_sub(&self, other: &$name) -> $name {
                self.sub(*other)
            }
        }

        impl SubAssign<$name> for $name {
            fn sub_assign(&mut self, rhs: $name) {
                *self = self.sub(rhs);
            }
        }

        impl Mul<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn mul(self, other: $name) -> $name {
                $name(self.0.wrapping_mul(other.0))
            }
        }
        impl WrappingMul for $name {
            fn wrapping_mul(&self, other: &$name) -> $name {
                self.mul(*other)
            }
        }

        impl MulAssign<$name> for $name {
            fn mul_assign(&mut self, rhs: $name) {
                *self = self.mul(rhs);
            }
        }

        impl Div<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn div(self, other: $name) -> $name {
                $name(self.0.wrapping_div(other.0))
            }
        }

        impl DivAssign<$name> for $name {
            fn div_assign(&mut self, rhs: $name) {
                *self = self.div(rhs);
            }
        }

        impl Rem<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn rem(self, other: $name) -> $name {
                $name(self.0.wrapping_rem(other.0))
            }
        }

        impl RemAssign<$name> for $name {
            fn rem_assign(&mut self, rhs: $name) {
                *self = self.rem(rhs);
            }
        }

        impl BitOr<$base_type> for $name {
            type Output = $name;

            fn bitor(self, rhs: $base_type) -> Self::Output {
                $name(self.0.bitor(rhs))
            }
        }

        impl<'a> BitOr<&'a $base_type> for $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitor(self, rhs: &'a $base_type) -> Self::Output {
                $name(self.0.bitor(rhs))
            }
        }

        impl<'a> BitOr<$base_type> for &'a $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitor(self, rhs: $base_type) -> Self::Output {
                $name(self.0.bitor(rhs))
            }
        }

        impl<'a> BitOr<&'a $base_type> for &'a $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitor(self, rhs: &'a $base_type) -> Self::Output {
                $name(self.0.bitor(rhs))
            }
        }

        impl BitOrAssign<$base_type> for $name {
            fn bitor_assign(&mut self, other: $base_type) {
                self.0.bitor_assign(other)
            }
        }

        impl BitXor<$base_type> for $name {
            type Output = $name;

            fn bitxor(self, rhs: $base_type) -> Self::Output {
                $name(self.0.bitxor(rhs))
            }
        }

        impl<'a> BitXor<&'a $base_type> for $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitxor(self, rhs: &'a $base_type) -> Self::Output {
                $name(self.0.bitxor(rhs))
            }
        }

        impl<'a> BitXor<$base_type> for &'a $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitxor(self, rhs: $base_type) -> Self::Output {
                $name(self.0.bitxor(rhs))
            }
        }

        impl<'a> BitXor<&'a $base_type> for &'a $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitxor(self, rhs: &'a $base_type) -> Self::Output {
                $name(self.0.bitxor(rhs))
            }
        }

        impl BitXorAssign<$base_type> for $name {
            fn bitxor_assign(&mut self, other: $base_type) {
                self.0.bitxor_assign(other)
            }
        }

        impl BitAnd<$base_type> for $name {
            type Output = $name;

            fn bitand(self, rhs: $base_type) -> Self::Output {
                $name(self.0.bitand(rhs))
            }
        }

        impl<'a> BitAnd<&'a $base_type> for $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitand(self, rhs: &'a $base_type) -> Self::Output {
                $name(self.0.bitand(rhs))
            }
        }

        impl<'a> BitAnd<$base_type> for &'a $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitand(self, rhs: $base_type) -> Self::Output {
                $name(self.0.bitand(rhs))
            }
        }

        impl<'a> BitAnd<&'a $base_type> for &'a $name {
            type Output = <$name as BitOr<$base_type>>::Output;

            fn bitand(self, rhs: &'a $base_type) -> Self::Output {
                $name(self.0.bitand(rhs))
            }
        }

        impl BitAndAssign<$base_type> for $name {
            fn bitand_assign(&mut self, other: $base_type) {
                self.0.bitand_assign(other)
            }
        }

        impl Add<$base_type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn add(self, other: $base_type) -> $name {
                $name(self.0.wrapping_add($type::new(other)))
            }
        }

        impl AddAssign<$base_type> for $name {
            fn add_assign(&mut self, rhs: $base_type) {
                *self = self.add(rhs);
            }
        }

        impl Sub<$base_type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn sub(self, other: $base_type) -> $name {
                $name(self.0.wrapping_sub($type::new(other)))
            }
        }

        impl SubAssign<$base_type> for $name {
            fn sub_assign(&mut self, rhs: $base_type) {
                *self = self.sub(rhs);
            }
        }

        impl Mul<$base_type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn mul(self, other: $base_type) -> $name {
                $name(self.0.wrapping_mul($type::new(other)))
            }
        }

        impl MulAssign<$base_type> for $name {
            fn mul_assign(&mut self, rhs: $base_type) {
                *self = self.mul(rhs);
            }
        }

        impl Div<$base_type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn div(self, other: $base_type) -> $name {
                $name(self.0.wrapping_div($type::new(other)))
            }
        }

        impl DivAssign<$base_type> for $name {
            fn div_assign(&mut self, rhs: $base_type) {
                *self = self.div(rhs);
            }
        }

        impl Rem<$base_type> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn rem(self, other: $base_type) -> $name {
                $name(self.0.wrapping_rem($type::new(other)))
            }
        }

        impl RemAssign<$base_type> for $name {
            fn rem_assign(&mut self, rhs: $base_type) {
                *self = self.rem(rhs);
            }
        }
    };
}

macro_rules! define_type {
    ($name:ident, $type:ident) => {
        #[allow(non_camel_case_types)]
        pub type $name = $type;
    };
}

macro_rules! define_hw_int_for {
    ($name:ident) => {
        impl Unsigned for $name {}
        impl HardwareInteger for $name {}
    };
}

macro_rules! define_hw_int {
    ($struct_name:ident, $name:ident, $w_struct_name:ident, $w_name:ident, $bits:expr, $type:ident) => {
        define_unsigned!($struct_name, $bits, $type);
        define_type!($name, $struct_name);

        define_wrapping!($w_struct_name, $struct_name, $type);
        define_type!($w_name, $w_struct_name);
        define_hw_int_for!($w_name);
    };
}

define_hw_int!(U14, u14, U14W, u14w, 14, u16);
define_hw_int!(U24, u24, U24W, u24w, 24, u32);
