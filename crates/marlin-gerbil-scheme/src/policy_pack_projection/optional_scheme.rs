use std::fmt;

use serde::{
    Deserializer,
    de::{self, Visitor},
};

pub(super) fn deserialize_optional_scheme_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalSchemeStringVisitor;

    impl<'de> Visitor<'de> for OptionalSchemeStringVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string, null, or Scheme #f")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value {
                Err(E::custom("expected Scheme #f or string, got #t"))
            } else {
                Ok(None)
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_optional_scheme_string(deserializer)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_owned()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(OptionalSchemeStringVisitor)
}
