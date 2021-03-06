// LNP/BP Core Library implementing LNPBP specifications & standards
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

mod encoding;
mod error;
pub mod payload;
pub mod tlv;

pub use encoding::{
    CreateUnmarshaller, Decode, Encode, Unmarshall, UnmarshallFn,
};
pub use error::{Error, UnknownTypeError};
pub use payload::{Payload, Unmarshaller};

use amplify::Wrapper;
use core::ops::Rem;
use std::borrow::Borrow;

pub trait EvenOdd
where
    Self: Wrapper,
    Self::Inner: Rem + From<u8>,
    <Self::Inner as Rem>::Output: Eq + From<u8>,
{
    #[inline]
    fn is_odd(&self) -> bool {
        !self.is_even()
    }

    #[inline]
    fn is_even(&self) -> bool {
        self.to_inner() % 2.into() == 0.into()
    }
}

/// Lightning TLV uses a custom variable-length integer called BigSize. It is
/// similar to Bitcoin's variable-length integers except that it is serialized
/// in big-endian instead of little-endian.
///
/// BigSize specification is given at
/// https://github.com/lightningnetwork/lightning-rfc/blob/master/01-messaging.md#type-length-value-format
///
/// Like Bitcoin's variable-length integer, it exhibits ambiguity in that
/// certain values can be encoded in several different ways, which we must check
/// for at deserialization-time. Thus, if you're looking for an example of a
/// variable-length integer to use for your own project, move along, this is a
/// rather poor design.

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Error, From)]
#[display(doc_comments)]
pub(crate) enum BigsizeError {
    /// I/O Error in Bigsize
    #[from]
    IoError(std::io::ErrorKind),

    /// Invalid value in BigSize
    InvalidValue(String),
}
#[derive(Debug, PartialEq)]
pub(crate) struct BigSize(pub u64);

impl Encode for BigSize {
    type Error = BigsizeError;

    fn encode(&self) -> Result<Vec<u8>, Self::Error> {
        match self.0 {
            0..=0xFC => Ok(vec![self.0 as u8]),
            0xFD..=0xFFFF => {
                let mut result = (self.0 as u16).to_be_bytes().to_vec();
                result.insert(0, 0xFDu8);
                Ok(result)
            }
            0x10000..=0xFFFFFFFF => {
                let mut result = (self.0 as u32).to_be_bytes().to_vec();
                result.insert(0, 0xFEu8);
                Ok(result)
            }
            _ => {
                let mut result = (self.0 as u64).to_be_bytes().to_vec();
                result.insert(0, 0xFF);
                Ok(result)
            }
        }
    }
}

impl Decode for BigSize {
    type Error = BigsizeError;

    fn decode(data: &dyn Borrow<[u8]>) -> Result<Self, Self::Error> {
        let data = data.borrow().to_vec();
        match data[0] {
            0xFFu8 => {
                if data.len() < 9 {
                    return Err(BigsizeError::InvalidValue(String::from(
                        "Unexpected EOF",
                    )));
                }
                let mut x = [0u8; 8];
                x.copy_from_slice(&data[1..]);
                let value = u64::from_be_bytes(x);
                if value < 0x100000000 {
                    Err(BigsizeError::InvalidValue(String::from(
                        "decoded bigsize is not canonical",
                    )))
                } else {
                    Ok(BigSize(value as u64))
                }
            }
            0xFEu8 => {
                if data.len() < 5 {
                    return Err(BigsizeError::InvalidValue(String::from(
                        "Unexpected EOF",
                    )));
                }
                let mut x = [0u8; 4];
                x.copy_from_slice(&data[1..]);
                let value = u32::from_be_bytes(x);
                if value < 0x10000 {
                    Err(BigsizeError::InvalidValue(String::from(
                        "decoded bigsize is not canonical",
                    )))
                } else {
                    Ok(BigSize(value as u64))
                }
            }
            0xFDu8 => {
                if data.len() < 3 {
                    return Err(BigsizeError::InvalidValue(String::from(
                        "Unexpected EOF",
                    )));
                }
                let mut x = [0u8; 2];
                x.copy_from_slice(&data[1..]);
                let value = u16::from_be_bytes(x);
                if value < 0xFD {
                    Err(BigsizeError::InvalidValue(String::from(
                        "decoded bigsize is not canonical",
                    )))
                } else {
                    Ok(BigSize(value as u64))
                }
            }
            _ => Ok(BigSize(data[0] as u64)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Runs unit tests for BigSize structure.
    // Testing vectors taken from
    //https://github.com/lightningnetwork/lightning-rfc/blob/master/01-messaging.md#appendix-a-bigsize-test-vectors

    fn test_runner(value: u64, bytes: &[u8]) {
        let bigsize = BigSize(value);

        let encoded_bigsize = bigsize.encode().unwrap();

        assert_eq!(encoded_bigsize, bytes);

        let decoded_bigsize = BigSize::decode(&encoded_bigsize).unwrap();

        assert_eq!(decoded_bigsize, bigsize);
    }

    #[test]
    fn test_1() {
        test_runner(0, &[0x00]);
        test_runner(252, &[0xfc]);
        test_runner(253, &[0xfd, 0x00, 0xfd]);
        test_runner(65535, &[0xfd, 0xff, 0xff]);
        test_runner(65536, &[0xfe, 0x00, 0x01, 0x00, 0x00]);
        test_runner(4294967295, &[0xfe, 0xff, 0xff, 0xff, 0xff]);
        test_runner(
            4294967296,
            &[0xff, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
        );
        test_runner(
            18446744073709551615,
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
    }

    #[should_panic(expected = "decoded bigsize is not canonical")]
    #[test]
    fn test_canonical_value_error_1() {
        BigSize::decode(&[0xfd, 0x00, 0xfc]).unwrap();
    }

    #[should_panic(expected = "decoded bigsize is not canonical")]
    #[test]
    fn test_canonical_value_error_2() {
        BigSize::decode(&[0xfe, 0x00, 0x00, 0xff, 0xff]).unwrap();
    }

    #[should_panic(expected = "decoded bigsize is not canonical")]
    #[test]
    fn test_canonical_value_error_3() {
        BigSize::decode(&[
            0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
        ])
        .unwrap();
    }

    #[should_panic(expected = "Unexpected EOF")]
    #[test]
    fn test_eof_error_1() {
        BigSize::decode(&[0xfd, 0x00]).unwrap();
    }

    #[should_panic(expected = "Unexpected EOF")]
    #[test]
    fn test_eof_error_2() {
        BigSize::decode(&[0xfe, 0xff, 0xff]).unwrap();
    }

    #[should_panic(expected = "Unexpected EOF")]
    #[test]
    fn test_eof_error_3() {
        BigSize::decode(&[0xff, 0xff, 0xff, 0xff, 0xff]).unwrap();
    }

    #[should_panic(expected = "Unexpected EOF")]
    #[test]
    fn test_eof_error_4() {
        BigSize::decode(&[0xfd]).unwrap();
    }

    #[should_panic(expected = "Unexpected EOF")]
    #[test]
    fn test_eof_error_5() {
        BigSize::decode(&[0xfe]).unwrap();
    }

    #[should_panic(expected = "Unexpected EOF")]
    #[test]
    fn test_eof_error_6() {
        BigSize::decode(&[0xff]).unwrap();
    }
}
