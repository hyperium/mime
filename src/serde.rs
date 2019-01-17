use std::fmt;

use serde1::de::{self, Deserialize, Deserializer};
use serde1::ser::{Serialize, Serializer};

use super::{MediaType, MediaRange};

macro_rules! serde_impl {
    ($ty:ident) => (
        impl Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_ref())
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> de::Visitor<'de> for Visitor {
                    type Value = $ty;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str(concat!("a valid ", stringify!($ty)))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$ty, E>
                    where
                        E: de::Error,
                    {
                        $ty::parse(value).map_err(E::custom)
                    }
                }

                deserializer.deserialize_str(Visitor)
            }
        }
    )
}

serde_impl!(MediaType);
serde_impl!(MediaRange);
