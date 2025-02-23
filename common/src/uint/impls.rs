// Rust Bitcoin Library
// Written in 2014 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
//
// Modified in 2022 by
//     Carla Yap <carla.yap@mintlayer.org>
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Big unsigned integer types.
//!
//! Implementation of various large-but-fixed sized unsigned integer types.
//! The functions here are designed to be fast.

use crate::primitives::{Amount, H256};
use serialization::{Decode, Encode};

macro_rules! construct_uint {
    ($name:ident, $n_words:expr) => {
        /// little endian large integer type
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub [u64; $n_words]);
        impl_array_newtype!($name, u64, $n_words);

        impl $name {
            pub const ZERO: Self = Self::from_u64(0u64);

            /// Conversion to u32
            #[inline]
            pub fn low_u32(&self) -> u32 {
                let &$name(ref arr) = self;
                arr[0] as u32
            }

            /// Conversion to u64
            #[inline]
            pub fn low_u64(&self) -> u64 {
                let &$name(ref arr) = self;
                arr[0] as u64
            }

            /// Return the least number of bits needed to represent the number
            #[inline]
            pub fn bits(&self) -> usize {
                let &$name(ref arr) = self;
                for i in 1..$n_words {
                    if arr[$n_words - i] > 0 {
                        return (0x40 * ($n_words - i + 1))
                            - arr[$n_words - i].leading_zeros() as usize;
                    }
                }
                0x40 - arr[0].leading_zeros() as usize
            }

            /// Multiplication by u32
            pub fn mul_u32(self, other: u32) -> $name {
                let $name(ref arr) = self;
                let mut carry = [0u64; $n_words];
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    let not_last_word = i < $n_words - 1;
                    let upper = other as u64 * (arr[i] >> 32);
                    let lower = other as u64 * (arr[i] & 0xFFFFFFFF);
                    if not_last_word {
                        carry[i + 1] += upper >> 32;
                    }
                    let (sum, overflow) = lower.overflowing_add(upper << 32);
                    ret[i] = sum;
                    if overflow && not_last_word {
                        carry[i + 1] += 1;
                    }
                }
                $name(ret) + $name(carry)
            }

            /// Create an object from a given unsigned 64-bit integer
            #[inline]
            pub const fn from_u64(init: u64) -> $name {
                let mut ret = [0; $n_words];
                ret[0] = init;
                $name(ret)
            }

            /// Create an object from a given unsigned 128-bit integer
            #[inline]
            pub fn from_u128(init: u128) -> $name {
                let serialized_input = init.encode();
                let pad: [u8; 8 * ($n_words - 2)] = [0; 8 * ($n_words - 2)];
                let full: [u8; 8 * $n_words] = serialized_input
                    .into_iter()
                    .chain(pad.into_iter())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .expect("Size should match");
                Self::from_bytes(full)
            }

            /// Create an object from a given unsigned Amount
            #[inline]
            pub fn from_amount(init: Amount) -> $name {
                Self::from_u128(init.into_atoms())
            }

            /// Create an object from a given signed 64-bit integer
            #[inline]
            pub fn from_i64(init: i64) -> Option<$name> {
                if init >= 0 {
                    Some($name::from_u64(init as u64))
                } else {
                    None
                }
            }

            /// Creates big integer value from a byte array using
            /// big endian encoding
            pub fn from_be_bytes(bytes: [u8; $n_words * 8]) -> $name {
                Self::_from_be_slice(&bytes)
            }

            /// Creates big integer value from a byte slice using
            /// big endian encoding
            pub fn from_be_slice(bytes: &[u8]) -> Result<$name, ParseLengthError> {
                if bytes.len() != $n_words * 8 {
                    Err(ParseLengthError {
                        actual: bytes.len(),
                        expected: $n_words * 8,
                    })
                } else {
                    Ok(Self::_from_be_slice(bytes))
                }
            }

            fn _from_be_slice(bytes: &[u8]) -> $name {
                use crate::uint::endian::slice_to_u64_be;
                let mut slice = [0u64; $n_words];
                slice
                    .iter_mut()
                    .rev()
                    .zip(bytes.chunks(8))
                    .for_each(|(word, bytes)| *word = slice_to_u64_be(bytes));
                $name(slice)
            }

            /// Convert a big integer into a byte array using big endian encoding
            pub fn to_be_bytes(&self) -> [u8; $n_words * 8] {
                use crate::uint::endian::u64_to_array_be;
                let mut res = [0; $n_words * 8];
                for i in 0..$n_words {
                    let start = i * 8;
                    res[start..start + 8]
                        .copy_from_slice(&u64_to_array_be(self.0[$n_words - (i + 1)]));
                }
                res
            }

            /// Creates big integer value from a byte array using
            /// little endian encoding
            pub fn from_bytes(bytes: [u8; $n_words * 8]) -> $name {
                Self::inner_from_slice(&bytes)
            }

            /// Creates big integer value from a byte slice using
            /// little endian encoding
            pub fn from_slice(bytes: &[u8]) -> Result<$name, ParseLengthError> {
                if bytes.len() != $n_words * 8 {
                    Err(ParseLengthError {
                        actual: bytes.len(),
                        expected: $n_words * 8,
                    })
                } else {
                    Ok(Self::inner_from_slice(bytes))
                }
            }

            fn inner_from_slice(bytes: &[u8]) -> $name {
                use crate::uint::endian::slice_to_u64_le;
                let mut slice = [0u64; $n_words];
                slice
                    .iter_mut()
                    .zip(bytes.chunks(8))
                    .for_each(|(word, bytes)| *word = slice_to_u64_le(bytes));
                $name(slice)
            }

            /// Convert a big integer into a byte array using little endian encoding
            pub fn to_bytes(&self) -> [u8; $n_words * 8] {
                use crate::uint::endian::u64_to_array_le;
                let mut res = [0; $n_words * 8];
                for i in 0..$n_words {
                    let start = i * 8;
                    res[start..start + 8].copy_from_slice(&u64_to_array_le(self.0[i]));
                }
                res
            }

            // divmod like operation, returns (quotient, remainder)
            #[inline]
            fn div_rem(self, other: Self) -> (Self, Self) {
                let mut sub_copy = self;
                let mut shift_copy = other;
                let mut ret = [0u64; $n_words];

                let my_bits = self.bits();
                let your_bits = other.bits();

                // Check for division by 0
                assert!(your_bits != 0);

                // Early return in case we are dividing by a larger number than us
                if my_bits < your_bits {
                    return ($name(ret), sub_copy);
                }

                // Bitwise long division
                let mut shift = my_bits - your_bits;
                shift_copy = shift_copy << shift;
                loop {
                    if sub_copy >= shift_copy {
                        ret[shift / 64] |= 1 << (shift % 64);
                        sub_copy = sub_copy - shift_copy;
                    }
                    shift_copy = shift_copy >> 1;
                    if shift == 0 {
                        break;
                    }
                    shift -= 1;
                }

                ($name(ret), sub_copy)
            }

            /// Increment by 1
            #[inline]
            pub fn increment(&mut self) {
                let &mut $name(ref mut arr) = self;
                for i in 0..$n_words {
                    arr[i] = arr[i].wrapping_add(1);
                    if arr[i] != 0 {
                        break;
                    }
                }
            }
        }

        impl From<[u8; $n_words * 8]> for $name {
            /// Creates a Uint256 from the given bytes array of fixed length.
            ///
            /// # Note
            ///
            /// The given bytes are assumed to be in little endian order.
            #[inline]
            fn from(data: [u8; $n_words * 8]) -> Self {
                Self::from_bytes(data)
            }
        }

        impl<'a> From<&'a [u8; $n_words * 8]> for $name {
            /// Creates a Uint256 from the given reference
            /// to the bytes array of fixed length.
            ///
            /// # Note
            ///
            /// The given bytes are assumed to be in little endian order.
            #[inline]
            fn from(data: &'a [u8; $n_words * 8]) -> Self {
                Self::inner_from_slice(data)
            }
        }

        impl From<u64> for $name {
            #[inline]
            fn from(n: u64) -> Self {
                Self::from_u64(n)
            }
        }

        impl From<u128> for $name {
            #[inline]
            fn from(n: u128) -> Self {
                Self::from_u128(n)
            }
        }

        impl From<Amount> for $name {
            #[inline]
            fn from(n: Amount) -> Self {
                Self::from_amount(n)
            }
        }

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &$name) -> Option<::core::cmp::Ordering> {
                Some(self.cmp(&other))
            }
        }

        impl Ord for $name {
            #[inline]
            fn cmp(&self, other: &$name) -> ::core::cmp::Ordering {
                // We need to manually implement ordering because we use little endian
                // and the auto derive is a lexicographic ordering(i.e. memcmp)
                // which with numbers is equivalent to big endian
                for i in 0..$n_words {
                    if self[$n_words - 1 - i] < other[$n_words - 1 - i] {
                        return ::core::cmp::Ordering::Less;
                    }
                    if self[$n_words - 1 - i] > other[$n_words - 1 - i] {
                        return ::core::cmp::Ordering::Greater;
                    }
                }
                ::core::cmp::Ordering::Equal
            }
        }

        impl ::core::ops::Add<$name> for $name {
            type Output = $name;

            fn add(self, other: $name) -> $name {
                let $name(ref me) = self;
                let $name(ref you) = other;
                let mut ret = [0u64; $n_words];
                let mut carry = [0u64; $n_words];
                let mut b_carry = false;
                for i in 0..$n_words {
                    ret[i] = me[i].wrapping_add(you[i]);
                    if i < $n_words - 1 && ret[i] < me[i] {
                        carry[i + 1] = 1;
                        b_carry = true;
                    }
                }
                if b_carry {
                    $name(ret) + $name(carry)
                } else {
                    $name(ret)
                }
            }
        }

        impl ::core::ops::Sub<$name> for $name {
            type Output = $name;

            #[inline]
            fn sub(self, other: $name) -> $name {
                self + !other + $crate::uint::BitArray::one()
            }
        }

        impl ::core::ops::Mul<$name> for $name {
            type Output = $name;

            fn mul(self, other: $name) -> $name {
                use $crate::uint::BitArray;
                let mut me = $name::zero();
                // TODO: be more efficient about this
                for i in 0..(2 * $n_words) {
                    let to_mul = (other >> (32 * i)).low_u32();
                    me = me + (self.mul_u32(to_mul) << (32 * i));
                }
                me
            }
        }

        impl ::core::ops::Div<$name> for $name {
            type Output = $name;

            fn div(self, other: $name) -> $name {
                self.div_rem(other).0
            }
        }

        impl ::core::ops::Rem<$name> for $name {
            type Output = $name;

            fn rem(self, other: $name) -> $name {
                self.div_rem(other).1
            }
        }

        impl $crate::uint::BitArray for $name {
            #[inline]
            fn bit(&self, index: usize) -> bool {
                let &$name(ref arr) = self;
                arr[index / 64] & (1 << (index % 64)) != 0
            }

            #[inline]
            fn bit_slice(&self, start: usize, end: usize) -> $name {
                (*self >> start).mask(end - start)
            }

            #[inline]
            fn mask(&self, n: usize) -> $name {
                let &$name(ref arr) = self;
                let mut ret = [0; $n_words];
                for i in 0..$n_words {
                    if n >= 0x40 * (i + 1) {
                        ret[i] = arr[i];
                    } else {
                        ret[i] = arr[i] & ((1 << (n - 0x40 * i)) - 1);
                        break;
                    }
                }
                $name(ret)
            }

            #[inline]
            fn trailing_zeros(&self) -> usize {
                let &$name(ref arr) = self;
                for i in 0..($n_words - 1) {
                    if arr[i] > 0 {
                        return (0x40 * i) + arr[i].trailing_zeros() as usize;
                    }
                }
                (0x40 * ($n_words - 1)) + arr[$n_words - 1].trailing_zeros() as usize
            }

            fn zero() -> $name {
                Default::default()
            }
            fn one() -> $name {
                $name({
                    let mut ret = [0; $n_words];
                    ret[0] = 1;
                    ret
                })
            }
        }

        impl ::core::ops::BitAnd<$name> for $name {
            type Output = $name;

            #[inline]
            fn bitand(self, other: $name) -> $name {
                let $name(ref arr1) = self;
                let $name(ref arr2) = other;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = arr1[i] & arr2[i];
                }
                $name(ret)
            }
        }

        impl ::core::ops::BitXor<$name> for $name {
            type Output = $name;

            #[inline]
            fn bitxor(self, other: $name) -> $name {
                let $name(ref arr1) = self;
                let $name(ref arr2) = other;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = arr1[i] ^ arr2[i];
                }
                $name(ret)
            }
        }

        impl ::core::ops::BitOr<$name> for $name {
            type Output = $name;

            #[inline]
            fn bitor(self, other: $name) -> $name {
                let $name(ref arr1) = self;
                let $name(ref arr2) = other;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = arr1[i] | arr2[i];
                }
                $name(ret)
            }
        }

        impl ::core::ops::Not for $name {
            type Output = $name;

            #[inline]
            fn not(self) -> $name {
                let $name(ref arr) = self;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = !arr[i];
                }
                $name(ret)
            }
        }

        impl ::core::ops::Shl<usize> for $name {
            type Output = $name;

            fn shl(self, shift: usize) -> $name {
                let $name(ref original) = self;
                let mut ret = [0u64; $n_words];
                let word_shift = shift / 64;
                let bit_shift = shift % 64;
                for i in 0..$n_words {
                    // Shift
                    if bit_shift < 64 && i + word_shift < $n_words {
                        ret[i + word_shift] += original[i] << bit_shift;
                    }
                    // Carry
                    if bit_shift > 0 && i + word_shift + 1 < $n_words {
                        ret[i + word_shift + 1] += original[i] >> (64 - bit_shift);
                    }
                }
                $name(ret)
            }
        }

        impl ::core::ops::Shr<usize> for $name {
            type Output = $name;

            fn shr(self, shift: usize) -> $name {
                let $name(ref original) = self;
                let mut ret = [0u64; $n_words];
                let word_shift = shift / 64;
                let bit_shift = shift % 64;
                for i in word_shift..$n_words {
                    // Shift
                    ret[i - word_shift] += original[i] >> bit_shift;
                    // Carry
                    if bit_shift > 0 && i < $n_words - 1 {
                        ret[i - word_shift] += original[i + 1] << (64 - bit_shift);
                    }
                }
                $name(ret)
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let &$name(ref data) = self;
                write!(f, "0x")?;
                for ch in data.iter().rev() {
                    write!(f, "{:016x}", ch)?;
                }
                Ok(())
            }
        }
    };
}

construct_uint!(Uint256, 4);
construct_uint!(Uint128, 2);

impl Encode for Uint256 {
    fn size_hint(&self) -> usize {
        32
    }

    fn encode_to<T: serialization::Output + ?Sized>(&self, dest: &mut T) {
        let v: H256 = (*self).into();
        v.encode_to(dest)
    }

    fn encoded_size(&self) -> usize {
        32
    }
}

impl Decode for Uint256 {
    fn decode<I: serialization::Input>(input: &mut I) -> Result<Self, serialization::Error> {
        let v = <H256>::decode(input)?;
        Ok(v.into())
    }
}

/// Invalid slice length
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
/// Invalid slice length
pub struct ParseLengthError {
    /// The length of the slice de-facto
    pub actual: usize,
    /// The required length of the slice
    pub expected: usize,
}

impl ::core::fmt::Display for ParseLengthError {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(
            f,
            "Invalid length: got {}, expected {}",
            self.actual, self.expected
        )
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl ::std::error::Error for ParseLengthError {}

impl Uint256 {
    /// Decay to a uint128
    #[inline]
    pub fn low_128(&self) -> Uint128 {
        let &Uint256(data) = self;
        Uint128([data[0], data[1]])
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serialization::DecodeAll;

    use super::*;
    use crate::uint::BitArray;
    use test_utils::random::Seed;

    #[rstest]
    #[trace]
    #[case(Seed::from_entropy())]
    pub fn uint256_serialization(#[case] seed: Seed) {
        let mut rng = test_utils::random::make_seedable_rng(seed);

        let h256val = H256::random_using(&mut rng);
        let uint256val: Uint256 = h256val.into();
        let encoded_h256 = h256val.encode();
        let encoded_uint256val = uint256val.encode();
        assert_eq!(encoded_uint256val.len(), 32);
        assert_eq!(encoded_h256, encoded_uint256val);

        let decoded_uint256 = Uint256::decode_all(&mut encoded_uint256val.as_slice()).unwrap();
        assert_eq!(decoded_uint256, uint256val);
    }

    #[test]
    pub fn uint256_bits_test() {
        assert_eq!(Uint256::from_u64(255).bits(), 8);
        assert_eq!(Uint256::from_u64(256).bits(), 9);
        assert_eq!(Uint256::from_u64(300).bits(), 9);
        assert_eq!(Uint256::from_u64(60000).bits(), 16);
        assert_eq!(Uint256::from_u64(70000).bits(), 17);

        // Try to read the following lines out loud quickly
        let mut shl = Uint256::from_u64(70000);
        shl = shl << 100;
        assert_eq!(shl.bits(), 117);
        shl = shl << 100;
        assert_eq!(shl.bits(), 217);
        shl = shl << 100;
        assert_eq!(shl.bits(), 0);

        // Bit set check
        assert!(!Uint256::from_u64(10).bit(0));
        assert!(Uint256::from_u64(10).bit(1));
        assert!(!Uint256::from_u64(10).bit(2));
        assert!(Uint256::from_u64(10).bit(3));
        assert!(!Uint256::from_u64(10).bit(4));
    }

    #[test]
    pub fn uint256_display_test() {
        assert_eq!(
            format!("{:?}", Uint256::from_u64(0xDEADBEEF)),
            "0x00000000000000000000000000000000000000000000000000000000deadbeef"
        );
        assert_eq!(
            format!("{:?}", Uint256::from_u64(u64::max_value())),
            "0x000000000000000000000000000000000000000000000000ffffffffffffffff"
        );

        let max_val = Uint256([
            0xFFFFFFFFFFFFFFFF,
            0xFFFFFFFFFFFFFFFF,
            0xFFFFFFFFFFFFFFFF,
            0xFFFFFFFFFFFFFFFF,
        ]);
        assert_eq!(
            format!("{:?}", max_val),
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
    }

    #[test]
    pub fn uint256_comp_test() {
        let small = Uint256([10u64, 0, 0, 0]);
        let big = Uint256([0x8C8C3EE70C644118u64, 0x0209E7378231E632, 0, 0]);
        let bigger = Uint256([0x9C8C3EE70C644118u64, 0x0209E7378231E632, 0, 0]);
        let biggest = Uint256([0x5C8C3EE70C644118u64, 0x0209E7378231E632, 0, 1]);

        assert!(small < big);
        assert!(big < bigger);
        assert!(bigger < biggest);
        assert!(bigger <= biggest);
        assert!(bigger >= big);
        assert!(bigger >= small);
    }

    #[test]
    pub fn uint_from_be_bytes() {
        assert_eq!(
            Uint128::from_be_bytes([
                0x1b, 0xad, 0xca, 0xfe, 0xde, 0xad, 0xbe, 0xef, 0xde, 0xaf, 0xba, 0xbe, 0x2b, 0xed,
                0xfe, 0xed
            ]),
            Uint128([0xdeafbabe2bedfeed, 0x1badcafedeadbeef])
        );

        assert_eq!(
            Uint256::from_be_bytes([
                0x1b, 0xad, 0xca, 0xfe, 0xde, 0xad, 0xbe, 0xef, 0xde, 0xaf, 0xba, 0xbe, 0x2b, 0xed,
                0xfe, 0xed, 0xba, 0xad, 0xf0, 0x0d, 0xde, 0xfa, 0xce, 0xda, 0x11, 0xfe, 0xd2, 0xba,
                0xd1, 0xc0, 0xff, 0xe0
            ]),
            Uint256([
                0x11fed2bad1c0ffe0,
                0xbaadf00ddefaceda,
                0xdeafbabe2bedfeed,
                0x1badcafedeadbeef
            ])
        );
    }

    #[test]
    pub fn uint_to_be_bytes() {
        assert_eq!(
            Uint128([0xdeafbabe2bedfeed, 0x1badcafedeadbeef]).to_be_bytes(),
            [
                0x1b, 0xad, 0xca, 0xfe, 0xde, 0xad, 0xbe, 0xef, 0xde, 0xaf, 0xba, 0xbe, 0x2b, 0xed,
                0xfe, 0xed
            ]
        );

        assert_eq!(
            Uint256([
                0x11fed2bad1c0ffe0,
                0xbaadf00ddefaceda,
                0xdeafbabe2bedfeed,
                0x1badcafedeadbeef
            ])
            .to_be_bytes(),
            [
                0x1b, 0xad, 0xca, 0xfe, 0xde, 0xad, 0xbe, 0xef, 0xde, 0xaf, 0xba, 0xbe, 0x2b, 0xed,
                0xfe, 0xed, 0xba, 0xad, 0xf0, 0x0d, 0xde, 0xfa, 0xce, 0xda, 0x11, 0xfe, 0xd2, 0xba,
                0xd1, 0xc0, 0xff, 0xe0
            ]
        );
    }

    #[test]
    pub fn uint_from_le_bytes() {
        assert_eq!(
            Uint128::from([
                0xed, 0xfe, 0xed, 0x2b, 0xbe, 0xba, 0xaf, 0xde, 0xef, 0xbe, 0xad, 0xde, 0xfe, 0xca,
                0xad, 0x1b
            ]),
            Uint128([0xdeafbabe2bedfeed, 0x1badcafedeadbeef])
        );

        assert_eq!(
            Uint256::from([
                0xe0, 0xff, 0xc0, 0xd1, 0xba, 0xd2, 0xfe, 0x11, 0xda, 0xce, 0xfa, 0xde, 0x0d, 0xf0,
                0xad, 0xba, 0xed, 0xfe, 0xed, 0x2b, 0xbe, 0xba, 0xaf, 0xde, 0xef, 0xbe, 0xad, 0xde,
                0xfe, 0xca, 0xad, 0x1b,
            ]),
            Uint256([
                0x11fed2bad1c0ffe0,
                0xbaadf00ddefaceda,
                0xdeafbabe2bedfeed,
                0x1badcafedeadbeef
            ])
        );
    }

    #[test]
    pub fn uint_to_le_bytes() {
        assert_eq!(
            Uint128([0xdeafbabe2bedfeed, 0x1badcafedeadbeef]).to_bytes(),
            [
                0xed, 0xfe, 0xed, 0x2b, 0xbe, 0xba, 0xaf, 0xde, 0xef, 0xbe, 0xad, 0xde, 0xfe, 0xca,
                0xad, 0x1b
            ]
        );

        assert_eq!(
            Uint256([
                0x11fed2bad1c0ffe0,
                0xbaadf00ddefaceda,
                0xdeafbabe2bedfeed,
                0x1badcafedeadbeef,
            ])
            .to_bytes(),
            [
                0xe0, 0xff, 0xc0, 0xd1, 0xba, 0xd2, 0xfe, 0x11, 0xda, 0xce, 0xfa, 0xde, 0x0d, 0xf0,
                0xad, 0xba, 0xed, 0xfe, 0xed, 0x2b, 0xbe, 0xba, 0xaf, 0xde, 0xef, 0xbe, 0xad, 0xde,
                0xfe, 0xca, 0xad, 0x1b,
            ]
        );
    }

    #[test]
    pub fn uint256_arithmetic_test() {
        let init = Uint256::from_u64(0xDEADBEEFDEADBEEF);
        let copy = init;

        let add = init + copy;
        assert_eq!(add, Uint256([0xBD5B7DDFBD5B7DDEu64, 1, 0, 0]));
        // Bitshifts
        let shl = add << 88;
        assert_eq!(shl, Uint256([0u64, 0xDFBD5B7DDE000000, 0x1BD5B7D, 0]));
        let shr = shl >> 40;
        assert_eq!(
            shr,
            Uint256([0x7DDE000000000000u64, 0x0001BD5B7DDFBD5B, 0, 0])
        );
        // Increment
        let mut incr = shr;
        incr.increment();
        assert_eq!(
            incr,
            Uint256([0x7DDE000000000001u64, 0x0001BD5B7DDFBD5B, 0, 0])
        );
        // Subtraction
        let sub = incr - init;
        assert_eq!(
            sub,
            Uint256([0x9F30411021524112u64, 0x0001BD5B7DDFBD5A, 0, 0])
        );
        // Multiplication
        let mult = sub.mul_u32(300);
        assert_eq!(
            mult,
            Uint256([0x8C8C3EE70C644118u64, 0x0209E7378231E632, 0, 0])
        );
        // Division
        assert_eq!(
            Uint256::from_u64(105) / Uint256::from_u64(5),
            Uint256::from_u64(21)
        );
        let div = mult / Uint256::from_u64(300);
        assert_eq!(
            div,
            Uint256([0x9F30411021524112u64, 0x0001BD5B7DDFBD5A, 0, 0])
        );

        assert_eq!(
            Uint256::from_u64(105) % Uint256::from_u64(5),
            Uint256::from_u64(0)
        );
        assert_eq!(
            Uint256::from_u64(35498456) % Uint256::from_u64(3435),
            Uint256::from_u64(1166)
        );
        let rem_src = mult * Uint256::from_u64(39842) + Uint256::from_u64(9054);
        assert_eq!(rem_src % Uint256::from_u64(39842), Uint256::from_u64(9054));
        // TODO: bit inversion
    }

    #[test]
    pub fn mul_u32_test() {
        let u64_val = Uint256::from_u64(0xDEADBEEFDEADBEEF);

        let u96_res = u64_val.mul_u32(0xFFFFFFFF);
        let u128_res = u96_res.mul_u32(0xFFFFFFFF);
        let u160_res = u128_res.mul_u32(0xFFFFFFFF);
        let u192_res = u160_res.mul_u32(0xFFFFFFFF);
        let u224_res = u192_res.mul_u32(0xFFFFFFFF);
        let u256_res = u224_res.mul_u32(0xFFFFFFFF);

        assert_eq!(u96_res, Uint256([0xffffffff21524111u64, 0xDEADBEEE, 0, 0]));
        assert_eq!(
            u128_res,
            Uint256([0x21524111DEADBEEFu64, 0xDEADBEEE21524110, 0, 0])
        );
        assert_eq!(
            u160_res,
            Uint256([0xBD5B7DDD21524111u64, 0x42A4822200000001, 0xDEADBEED, 0])
        );
        assert_eq!(
            u192_res,
            Uint256([0x63F6C333DEADBEEFu64, 0xBD5B7DDFBD5B7DDB, 0xDEADBEEC63F6C334, 0])
        );
        assert_eq!(
            u224_res,
            Uint256([0x7AB6FBBB21524111u64, 0xFFFFFFFBA69B4558, 0x854904485964BAAA, 0xDEADBEEB])
        );
        assert_eq!(
            u256_res,
            Uint256([
                0xA69B4555DEADBEEFu64,
                0xA69B455CD41BB662,
                0xD41BB662A69B4550,
                0xDEADBEEAA69B455C
            ])
        );
    }

    #[test]
    pub fn multiplication_test() {
        let u64_val = Uint256::from_u64(0xDEADBEEFDEADBEEF);

        let u128_res = u64_val * u64_val;

        assert_eq!(
            u128_res,
            Uint256([0x048D1354216DA321u64, 0xC1B1CD13A4D13D46, 0, 0])
        );

        let u256_res = u128_res * u128_res;

        assert_eq!(
            u256_res,
            Uint256([
                0xF4E166AAD40D0A41u64,
                0xF5CF7F3618C2C886u64,
                0x4AFCFF6F0375C608u64,
                0x928D92B4D7F5DF33u64
            ])
        );
    }

    #[test]
    pub fn increment_test() {
        let mut val = Uint256([
            0xFFFFFFFFFFFFFFFEu64,
            0xFFFFFFFFFFFFFFFFu64,
            0xFFFFFFFFFFFFFFFFu64,
            0xEFFFFFFFFFFFFFFFu64,
        ]);
        val.increment();
        assert_eq!(
            val,
            Uint256([
                0xFFFFFFFFFFFFFFFFu64,
                0xFFFFFFFFFFFFFFFFu64,
                0xFFFFFFFFFFFFFFFFu64,
                0xEFFFFFFFFFFFFFFFu64,
            ])
        );
        val.increment();
        assert_eq!(
            val,
            Uint256([
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0xF000000000000000u64,
            ])
        );

        let mut val = Uint256([
            0xFFFFFFFFFFFFFFFFu64,
            0xFFFFFFFFFFFFFFFFu64,
            0xFFFFFFFFFFFFFFFFu64,
            0xFFFFFFFFFFFFFFFFu64,
        ]);
        val.increment();
        assert_eq!(
            val,
            Uint256([
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
            ])
        );
    }

    #[test]
    pub fn uint256_bitslice_test() {
        let init = Uint256::from_u64(0xDEADBEEFDEADBEEF);
        let add = init + (init << 64);
        assert_eq!(add.bit_slice(64, 128), init);
        assert_eq!(add.mask(64), init);
    }

    #[test]
    pub fn uint256_extreme_bitshift_test() {
        // Shifting a u64 by 64 bits gives an undefined value, so make sure that
        // we're doing the Right Thing here
        let init = Uint256::from_u64(0xDEADBEEFDEADBEEF);

        assert_eq!(init << 64, Uint256([0, 0xDEADBEEFDEADBEEF, 0, 0]));
        let add = (init << 64) + init;
        assert_eq!(add, Uint256([0xDEADBEEFDEADBEEF, 0xDEADBEEFDEADBEEF, 0, 0]));
        assert_eq!(add >> 64, Uint256([0xDEADBEEFDEADBEEF, 0, 0, 0]));
        assert_eq!(
            add << 64,
            Uint256([0, 0xDEADBEEFDEADBEEF, 0xDEADBEEFDEADBEEF, 0])
        );
    }

    fn test_op_equality(v: u128) {
        let a1 = v;
        let b1 = a1 << 64;
        let a2 = Uint256::from_u64(v as u64);
        let b2 = a2 << 64;
        let b3 = Uint256::from_u128(b1);
        assert_eq!(b2, b3);
    }

    #[test]
    pub fn uint256_from_uint128() {
        test_op_equality(1);
        test_op_equality(10);
        test_op_equality(100);
        test_op_equality(1000);
        test_op_equality(10000);
        test_op_equality(100000);
        test_op_equality(1000000);
        test_op_equality(10000000);
        test_op_equality(100000000);
        test_op_equality(10000000000);
        test_op_equality(100000000000);
        test_op_equality(1000000000000);
        test_op_equality(10000000000000);
        test_op_equality(100000000000000);
        test_op_equality(1000000000000000);
        test_op_equality(10000000000000000);
        test_op_equality(100000000000000000);
        test_op_equality(1000000000000000000);
        test_op_equality(10000000000000000000);
        test_op_equality(100000000000000000000);
        test_op_equality(1000000000000000000000);
        test_op_equality(10000000000000000000000);
        test_op_equality(100000000000000000000000);
        test_op_equality(1000000000000000000000000);
        test_op_equality(10000000000000000000000000);
        test_op_equality(100000000000000000000000000);
    }
}
