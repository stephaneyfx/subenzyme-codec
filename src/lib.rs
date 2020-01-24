// Copyright (C) 2020 Stephane Raux. Distributed under the MIT license.

// #![deny(missing_docs)]
#![deny(warnings)]

use blake2b_simd::blake2b;
use parity_scale_codec::{Decode, Encode};
use std::convert::TryInto;
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::hash::Hasher;
use std::str::FromStr;
use twox_hash::XxHash64;

pub fn storage_key(module: &str, item: &str) -> u128 {
    let low = hash_with_space(XxHash64::with_seed(0), module, item) as u128;
    let high = hash_with_space(XxHash64::with_seed(1), module, item) as u128;
    let key = high << 64 | low;
    u128::from_be(key.to_le())
}

fn hash_with_space<H: Hasher>(mut hasher: H, left: &str, right: &str) -> u64 {
    hasher.write(left.as_bytes());
    hasher.write_u8(b' ');
    hasher.write(right.as_bytes());
    hasher.finish()
}

#[derive(Clone, Debug, Decode, Encode, Eq, Hash, PartialEq, PartialOrd)]
pub struct AccountId([u8; 32]);

impl AccountId {
    pub fn to_string(&self) -> String {
        let mut bytes = vec![42];
        bytes.extend(&self.0);
        let hash = hash_account(&bytes);
        bytes.extend(&hash.as_array()[0..2]);
        bs58::encode(&bytes).into_string()
    }
}

impl FromStr for AccountId {
    type Err = BadAccountId;

    fn from_str(s: &str) -> Result<Self, BadAccountId> {
        let bytes = bs58::decode(s).into_vec().map_err(BadAccountId::from_reason)?;
        if bytes.len() != 35 {
            return Err(BadAccountId::from_str(
                format!("Expected 35 bytes in account ID but found {}", bytes.len())
            ))
        }
        let account = &bytes[1..33];
        let hash = hash_account(&bytes[..33]);
        if bytes[33..] != hash.as_array()[0..2] {
            return Err(BadAccountId::from_str("Invalid hash in account ID"))
        }
        Ok(AccountId(account.try_into().unwrap()))
    }
}

#[derive(Debug)]
pub struct BadAccountId {
    reason: String,
}

impl BadAccountId {
    fn from_reason<E: StdError>(reason: E) -> Self {
        BadAccountId::from_str(reason.to_string())
    }

    fn from_str<S>(reason: S) -> Self
    where
        S: Into<String>,
    {
        let reason = reason.into();
        BadAccountId { reason }
    }
}

impl StdError for BadAccountId {}

impl Display for BadAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid account ID ({})", self.reason)
    }
}

fn hash_account(bytes: &[u8]) -> blake2b_simd::Hash {
    blake2b(&[&b"SS58PRE"[..], bytes].concat())
}

#[cfg(test)]
mod tests {
    use crate::{AccountId, storage_key};
    use std::convert::TryFrom;

    #[test]
    fn hash_sudo_key() {
        assert_eq!(storage_key("Sudo", "Key"), 0x50a63a871aced22e88ee6466fe5aa5d9);
    }

    const ACCOUNT_HEX: &str = "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const ACCOUNT_ID: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

    fn make_account_id() -> AccountId {
        let to_hex_digit = |n: u8| (n as char).to_digit(16).unwrap() as u8;
        let to_hex = |b: &[u8]| to_hex_digit(b[0]) << 4 | to_hex_digit(b[1]);
        let decoded = ACCOUNT_HEX.as_bytes()
            .chunks_exact(2)
            .map(to_hex)
            .collect::<Vec<_>>();
        let decoded = <[u8; 32]>::try_from(&*decoded).unwrap();
        AccountId(decoded)
    }

    #[test]
    fn can_convert_account_id_to_string() {
        assert_eq!(make_account_id().to_string(), ACCOUNT_ID);
    }

    #[test]
    fn can_convert_string_to_account_id() {
        assert_eq!(ACCOUNT_ID.parse::<AccountId>().unwrap(), make_account_id());
    }
}
