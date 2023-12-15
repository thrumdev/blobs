/// Serialize and deserialize vectors of bytes to hex string. The hex string can be of either case.
/// The hex string is serialized in lowercase. The hex string must always start with `0x`.
pub mod bytes_hex {
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = serialize_0x_hex_string(bytes);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> serde::de::Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let Some(s) = s.strip_prefix("0x") else {
                    return Err(serde::de::Error::custom("hex string must start with '0x'"));
                };
                hex::decode(s).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(BytesVisitor)
    }

    pub(crate) fn serialize_0x_hex_string(bytes: &[u8]) -> String {
        let expected_len = bytes.len() * 2 + 2;
        let mut buf = Vec::with_capacity(expected_len);
        buf.extend_from_slice(b"0x");
        buf.resize(expected_len, 0);
        hex::encode_to_slice(bytes, &mut buf[2..])
            .expect("the passed slice must be of even length");
        // SAFETY: the buffer is filled only by a string literal and hex::encode_to_slice. Both
        // are guaranteed to produce valid UTF-8.
        debug_assert!(std::str::from_utf8(&buf).is_ok());
        let s = unsafe { String::from_utf8_unchecked(buf) };
        s
    }

    #[cfg(test)]
    mod tests {
        use quickcheck_macros::quickcheck;
        use serde_json::json;

        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Dummy {
            #[serde(with = "super")]
            bytes: Vec<u8>,
        }

        #[quickcheck]
        fn test_roundtrip(bytes: Vec<u8>) {
            // Tests that serde serialization and deserialization is the inverse of each other.
            let given = Dummy { bytes };
            let bytes = serde_json::to_string(&given).unwrap();
            let actual: Dummy = serde_json::from_str(&bytes).unwrap();
            assert_eq!(given.bytes, actual.bytes);
        }

        #[quickcheck]
        fn test_deserialize(s: String) {
            let json = json!({
                "bytes": s,
            });
            match serde_json::from_value::<Dummy>(json) {
                Ok(d) => {
                    let expected = super::serialize_0x_hex_string(&d.bytes);
                    assert_eq!(s, expected);
                }
                Err(_) => (),
            }
        }

        #[test]
        fn test_serialize_0x_hex_string() {
            assert_eq!(super::serialize_0x_hex_string(&[]), "0x");
            assert_eq!(
                super::serialize_0x_hex_string(&[0x12, 0x34, 0x56, 0x78]),
                "0x12345678"
            );
            assert_eq!(
                super::serialize_0x_hex_string(&[0x00, 0x00, 0x00, 0x00]),
                "0x00000000"
            );
            assert_eq!(
                super::serialize_0x_hex_string(&[0xff, 0xff, 0xff, 0xff]),
                "0xffffffff"
            );
        }

        #[test]
        fn examples() {
            // this test uses manual serialization and deserialization to demonstrate how the
            // type is serialized and deserialized using serde_json.
            assert_eq!(
                serde_json::to_string(&Dummy {
                    bytes: vec![0x12, 0x34, 0x56, 0x78]
                })
                .unwrap(),
                r#"{"bytes":"0x12345678"}"#
            );
            assert_eq!(
                serde_json::to_string(&Dummy { bytes: vec![] }).unwrap(),
                r#"{"bytes":"0x"}"#
            );
        }
    }
}

/// Serialize and deserialize an array of 16 bytes to hex string. The hex string can be of either case.
/// The hex string is serialized in lowercase. The hex string must always start with `0x`.
pub mod bytes16_hex {
    use crate::bytes_hex::serialize_0x_hex_string;
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(bytes: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = serialize_0x_hex_string(&*bytes);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 16], D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> serde::de::Visitor<'de> for BytesVisitor {
            type Value = [u8; 16];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let Some(s) = s.strip_prefix("0x") else {
                    return Err(serde::de::Error::custom("hex string must start with '0x'"));
                };
                let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
                let bytes: [u8; 16] = bytes.try_into().map_err(|e: Vec<u8>| {
                    serde::de::Error::custom(format!(
                        "expected 16 bytes, but got {} bytes",
                        e.len()
                    ))
                })?;
                Ok(bytes)
            }
        }

        deserializer.deserialize_str(BytesVisitor)
    }

    #[cfg(test)]
    mod tests {
        use quickcheck_macros::quickcheck;
        use serde_json::json;

        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
        struct Dummy {
            #[serde(with = "super")]
            bytes: [u8; 16],
        }

        impl quickcheck::Arbitrary for Dummy {
            fn arbitrary(g: &mut quickcheck::Gen) -> Self {
                let mut bytes: [u8; 16] = [0u8; 16];
                for b in bytes.iter_mut() {
                    *b = u8::arbitrary(g);
                }
                Self { bytes }
            }
        }

        #[quickcheck]
        fn test_roundtrip(given: Dummy) {
            // Tests that serde serialization and deserialization is the inverse of each other.
            let bytes = serde_json::to_string(&given).unwrap();
            let actual: Dummy = serde_json::from_str(&bytes).unwrap();
            assert_eq!(given.bytes, actual.bytes);
        }

        #[quickcheck]
        fn test_deserialize(s: String) {
            let json = json!({
                "bytes": s,
            });
            match serde_json::from_value::<Dummy>(json) {
                Ok(d) => {
                    let expected = super::serialize_0x_hex_string(&d.bytes);
                    assert_eq!(s, expected);
                }
                Err(_) => (),
            }
        }

        #[test]
        fn examples() {
            // this test uses manual serialization and deserialization to demonstrate how the
            // type is serialized and deserialized using serde_json.
            assert_eq!(
                serde_json::to_string(&Dummy {
                    bytes: [
                        0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78,
                        0x12, 0x34, 0x56, 0x78
                    ]
                })
                .unwrap(),
                r#"{"bytes":"0x12345678123456781234567812345678"}"#.to_string()
            );
        }
    }
}

/// Serialize and deserialize an array of 32 bytes to hex string. The hex string can be of either case.
/// The hex string is serialized in lowercase. The hex string must always start with `0x`.
pub mod bytes32_hex {
    use crate::bytes_hex::serialize_0x_hex_string;
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = serialize_0x_hex_string(&*bytes);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> serde::de::Visitor<'de> for BytesVisitor {
            type Value = [u8; 32];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let Some(s) = s.strip_prefix("0x") else {
                    return Err(serde::de::Error::custom("hex string must start with '0x'"));
                };
                let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
                let bytes: [u8; 32] = bytes.try_into().map_err(|e: Vec<u8>| {
                    serde::de::Error::custom(format!(
                        "expected 32 bytes, but got {} bytes",
                        e.len()
                    ))
                })?;
                Ok(bytes)
            }
        }

        deserializer.deserialize_str(BytesVisitor)
    }

    #[cfg(test)]
    mod tests {
        use quickcheck_macros::quickcheck;
        use serde_json::json;

        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
        struct Dummy {
            #[serde(with = "super")]
            bytes: [u8; 32],
        }

        impl quickcheck::Arbitrary for Dummy {
            fn arbitrary(g: &mut quickcheck::Gen) -> Self {
                let mut bytes: [u8; 32] = [0u8; 32];
                for b in bytes.iter_mut() {
                    *b = u8::arbitrary(g);
                }
                Self { bytes }
            }
        }

        #[quickcheck]
        fn test_roundtrip(given: Dummy) {
            // Tests that serde serialization and deserialization is the inverse of each other.
            let bytes = serde_json::to_string(&given).unwrap();
            let actual: Dummy = serde_json::from_str(&bytes).unwrap();
            assert_eq!(given.bytes, actual.bytes);
        }

        #[quickcheck]
        fn test_deserialize(s: String) {
            let json = json!({
                "bytes": s,
            });
            match serde_json::from_value::<Dummy>(json) {
                Ok(d) => {
                    let expected = super::serialize_0x_hex_string(&d.bytes);
                    assert_eq!(s, expected);
                }
                Err(_) => (),
            }
        }

        #[test]
        fn examples() {
            // this test uses manual serialization and deserialization to demonstrate how the
            // type is serialized and deserialized using serde_json.
            assert_eq!(
                serde_json::to_string(&Dummy {
                    bytes: [
                        0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78,
                        0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78,
                        0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, // 32 bytes
                    ],
                })
                .unwrap(),
                r#"{"bytes":"0x1234567812345678123456781234567812345678123456781234567812345678"}"#.to_string()
            );
        }
    }
}

pub mod bytes_base64 {
    use base64::prelude::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let base64_string = BASE64_STANDARD.encode(bytes);
        serializer.serialize_str(&base64_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        BASE64_STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use quickcheck_macros::quickcheck;
        use serde_json::json;

        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Dummy {
            #[serde(with = "super")]
            bytes: Vec<u8>,
        }

        #[quickcheck]
        fn test_roundtrip(bytes: Vec<u8>) {
            // Tests that serde serialization and deserialization is the inverse of each other.
            let given = Dummy { bytes };
            let bytes = serde_json::to_string(&given).unwrap();
            let actual: Dummy = serde_json::from_str(&bytes).unwrap();
            assert_eq!(given.bytes, actual.bytes);
        }

        #[quickcheck]
        fn test_deserialize(s: String) {
            let json = json!({
                "bytes": s,
            });
            match serde_json::from_value::<Dummy>(json) {
                Ok(d) => {
                    let expected = super::BASE64_STANDARD.encode(&d.bytes);
                    assert_eq!(s, expected);
                }
                Err(_) => (),
            }
        }

        #[test]
        fn examples() {
            // this test uses manual serialization and deserialization to demonstrate how the
            // type is serialized and deserialized using serde_json.
            assert_eq!(
                serde_json::to_string(&Dummy {
                    bytes: vec![0x12, 0x34, 0x56, 0x78]
                })
                .unwrap(),
                r#"{"bytes":"EjRWeA=="}"#
            );
            assert_eq!(
                serde_json::to_string(&Dummy { bytes: vec![] }).unwrap(),
                r#"{"bytes":""}"#
            );
        }
    }
}
