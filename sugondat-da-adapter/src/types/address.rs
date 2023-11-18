use core::{
    fmt::{Display, Formatter},
    str::FromStr,
};
use serde::{Deserialize, Serialize};
use sov_rollup_interface::BasicAddress;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct Address(pub [u8; 32]);

impl BasicAddress for Address {}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let hash = hex::encode(&self.0);
        write!(f, "{hash}")
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<[u8; 32]> for Address {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for Address {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self(<[u8; 32]>::try_from(value)?))
    }
}

impl FromStr for Address {
    type Err = anyhow::Error;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        // TODO
        unimplemented!()
    }
}
