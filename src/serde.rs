extern crate serde;

use std::fmt;
use std::str::FromStr;

use self::serde::de::{self, Deserialize, Deserializer};
use self::serde::ser::{Serialize, Serializer};

use super::Mime;

impl Serialize for Mime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for Mime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Mime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid MIME type")
            }

            fn visit_str<E>(self, value: &str) -> Result<Mime, E>
            where
                E: de::Error,
            {
                Mime::from_str(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
