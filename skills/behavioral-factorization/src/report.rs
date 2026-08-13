use serde::Serialize;
use serde::de::{Deserialize, Deserializer, Error as _, MapAccess, Visitor};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Box<serde_json::Value>>,
}

impl ToolError {
    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_input",
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn resource_limit(message: impl Into<String>) -> Self {
        Self {
            code: "resource_limit",
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn resource_limit_with_details(
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: "resource_limit",
            message: message.into(),
            details: Some(Box::new(details)),
        }
    }

    #[must_use]
    pub fn io(message: impl Into<String>) -> Self {
        Self {
            code: "io_error",
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_invariant",
            message: message.into(),
            details: None,
        }
    }
}

impl Display for ToolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ToolError {}

pub(crate) fn deserialize_string_map_no_duplicates<'de, D, V>(
    deserializer: D,
) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct StringMapVisitor<V>(PhantomData<fn() -> V>);

    impl<'de, V> Visitor<'de> for StringMapVisitor<V>
    where
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<String, V>;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object with unique member names")
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, V>()? {
                if values.insert(key.clone(), value).is_some() {
                    return Err(A::Error::custom(format!(
                        "duplicate JSON object member {key:?}"
                    )));
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(StringMapVisitor(PhantomData))
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}
