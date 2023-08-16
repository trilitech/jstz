use derive_more::{Display, Error, From};

use tezos_crypto_rs::{base58::FromBase58CheckError, hash::TryFromPKError, CryptoError};

#[derive(Display, Debug, Error, From)]
pub enum Error {
    TezosFromBase58Error { source: FromBase58CheckError },
    TezosTryFromPKError { source: TryFromPKError },
    TezosCryptoError { source: CryptoError },
    InvalidSignature,
}

pub type Result<T> = std::result::Result<T, Error>;
