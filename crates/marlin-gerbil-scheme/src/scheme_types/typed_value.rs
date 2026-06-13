//! Typed Scheme values projected into Rust.

use std::{collections::BTreeMap, fmt};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{
        self, DeserializeOwned, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess,
        SeqAccess, VariantAccess, Visitor,
    },
    forward_to_deserialize_any,
};

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{GerbilSchemeSchemaId, GerbilSchemeTypeId},
    projection::{GerbilSchemeProjectionContract, GerbilSchemeTypedProjection},
};

/// Native Scheme datum carried over the extension ABI.
///
/// The serialized form is intentionally only a fixture/debug convenience. Runtime bindings should
/// build this value from Scheme native data and let Rust own projection into concrete types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GerbilSchemeValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Vector(Vec<GerbilSchemeValue>),
    Record(BTreeMap<String, GerbilSchemeValue>),
}

impl GerbilSchemeValue {
    pub fn null() -> Self {
        Self::Null
    }

    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    pub fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    pub fn float(value: f64) -> Self {
        Self::Float(value)
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn vector(values: impl IntoIterator<Item = GerbilSchemeValue>) -> Self {
        Self::Vector(values.into_iter().collect())
    }

    pub fn record<K>(fields: impl IntoIterator<Item = (K, GerbilSchemeValue)>) -> Self
    where
        K: Into<String>,
    {
        Self::Record(
            fields
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn empty_record() -> Self {
        Self::Record(BTreeMap::new())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::Integer(_) | Self::Float(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub fn is_vector(&self) -> bool {
        matches!(self, Self::Vector(_))
    }

    pub fn is_record(&self) -> bool {
        matches!(self, Self::Record(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_vector(&self) -> Option<&[GerbilSchemeValue]> {
        match self {
            Self::Vector(values) => Some(values.as_slice()),
            _ => None,
        }
    }

    pub fn as_record(&self) -> Option<&BTreeMap<String, GerbilSchemeValue>> {
        match self {
            Self::Record(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn get(&self, field_name: &str) -> Option<&GerbilSchemeValue> {
        self.as_record()?.get(field_name)
    }

    pub fn get_index(&self, index: usize) -> Option<&GerbilSchemeValue> {
        self.as_vector()?.get(index)
    }

    fn deserializer(&self) -> GerbilSchemeValueDeserializer<'_> {
        GerbilSchemeValueDeserializer { value: self }
    }
}

impl From<bool> for GerbilSchemeValue {
    fn from(value: bool) -> Self {
        Self::boolean(value)
    }
}

impl From<i32> for GerbilSchemeValue {
    fn from(value: i32) -> Self {
        Self::integer(i64::from(value))
    }
}

impl From<i64> for GerbilSchemeValue {
    fn from(value: i64) -> Self {
        Self::integer(value)
    }
}

impl From<&str> for GerbilSchemeValue {
    fn from(value: &str) -> Self {
        Self::text(value)
    }
}

impl From<String> for GerbilSchemeValue {
    fn from(value: String) -> Self {
        Self::text(value)
    }
}

struct GerbilSchemeValueDeserializer<'de> {
    value: &'de GerbilSchemeValue,
}

impl<'de> Deserializer<'de> for GerbilSchemeValueDeserializer<'de> {
    type Error = GerbilSchemeTypeDecodeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Null => visitor.visit_unit(),
            GerbilSchemeValue::Boolean(value) => visitor.visit_bool(*value),
            GerbilSchemeValue::Integer(value) => visitor.visit_i64(*value),
            GerbilSchemeValue::Float(value) => visitor.visit_f64(*value),
            GerbilSchemeValue::Text(value) => visitor.visit_str(value),
            GerbilSchemeValue::Vector(values) => visitor.visit_seq(GerbilSchemeVectorAccess {
                iter: values.iter(),
            }),
            GerbilSchemeValue::Record(fields) => visitor.visit_map(GerbilSchemeRecordAccess {
                iter: fields.iter(),
                value: None,
            }),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.value.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Boolean(value) => visitor.visit_bool(*value),
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_signed_integer(self.value, visitor, i8::MIN.into(), i8::MAX.into())
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_signed_integer(self.value, visitor, i16::MIN.into(), i16::MAX.into())
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_signed_integer(self.value, visitor, i32::MIN.into(), i32::MAX.into())
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_signed_integer(self.value, visitor, i64::MIN, i64::MAX)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned_integer(self.value, visitor, u8::MAX.into())
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned_integer(self.value, visitor, u16::MAX.into())
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned_integer(self.value, visitor, u32::MAX.into())
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned_integer(self.value, visitor, u64::MAX)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Integer(value) => visitor.visit_f32(*value as f32),
            GerbilSchemeValue::Float(value) => visitor.visit_f32(*value as f32),
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Integer(value) => visitor.visit_f64(*value as f64),
            GerbilSchemeValue::Float(value) => visitor.visit_f64(*value),
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Text(value) => {
                let mut chars = value.chars();
                let Some(value) = chars.next() else {
                    return Err(de::Error::invalid_length(0, &visitor));
                };
                if chars.next().is_some() {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Str(value.encode_utf8(&mut [0; 4])),
                        &visitor,
                    ));
                }
                visitor.visit_char(value)
            }
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Text(value) => visitor.visit_str(value),
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.value.is_null() {
            visitor.visit_unit()
        } else {
            Err(de::Error::invalid_type(
                unexpected_scheme_value(self.value),
                &visitor,
            ))
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Vector(values) => visitor.visit_seq(GerbilSchemeVectorAccess {
                iter: values.iter(),
            }),
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Record(fields) => visitor.visit_map(GerbilSchemeRecordAccess {
                iter: fields.iter(),
                value: None,
            }),
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            GerbilSchemeValue::Text(value) => {
                visitor.visit_enum(value.as_str().into_deserializer())
            }
            GerbilSchemeValue::Record(fields) if fields.len() == 1 => {
                let (variant, value) = fields.iter().next().expect("single variant");
                visitor.visit_enum(GerbilSchemeEnumAccess {
                    variant,
                    value: Some(value),
                })
            }
            value => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &visitor,
            )),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    forward_to_deserialize_any! {
        bytes byte_buf ignored_any
    }
}

fn deserialize_signed_integer<'de, V>(
    value: &GerbilSchemeValue,
    visitor: V,
    min: i64,
    max: i64,
) -> Result<V::Value, GerbilSchemeTypeDecodeError>
where
    V: Visitor<'de>,
{
    match value {
        GerbilSchemeValue::Integer(value) if (min..=max).contains(value) => {
            visitor.visit_i64(*value)
        }
        value => Err(de::Error::invalid_type(
            unexpected_scheme_value(value),
            &visitor,
        )),
    }
}

fn deserialize_unsigned_integer<'de, V>(
    value: &GerbilSchemeValue,
    visitor: V,
    max: u64,
) -> Result<V::Value, GerbilSchemeTypeDecodeError>
where
    V: Visitor<'de>,
{
    match value {
        GerbilSchemeValue::Integer(value) => {
            let unsigned = u64::try_from(*value)
                .map_err(|_| de::Error::invalid_type(de::Unexpected::Signed(*value), &visitor))?;
            if unsigned <= max {
                visitor.visit_u64(unsigned)
            } else {
                Err(de::Error::invalid_type(
                    de::Unexpected::Unsigned(unsigned),
                    &visitor,
                ))
            }
        }
        value => Err(de::Error::invalid_type(
            unexpected_scheme_value(value),
            &visitor,
        )),
    }
}

struct GerbilSchemeEnumAccess<'de> {
    variant: &'de str,
    value: Option<&'de GerbilSchemeValue>,
}

impl<'de> EnumAccess<'de> for GerbilSchemeEnumAccess<'de> {
    type Error = GerbilSchemeTypeDecodeError;
    type Variant = GerbilSchemeVariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, GerbilSchemeVariantAccess { value: self.value }))
    }
}

struct GerbilSchemeVariantAccess<'de> {
    value: Option<&'de GerbilSchemeValue>,
}

impl<'de> VariantAccess<'de> for GerbilSchemeVariantAccess<'de> {
    type Error = GerbilSchemeTypeDecodeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            Some(GerbilSchemeValue::Null) | None => Ok(()),
            Some(value) => Err(de::Error::invalid_type(
                unexpected_scheme_value(value),
                &"unit variant",
            )),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let value = self
            .value
            .ok_or_else(|| GerbilSchemeTypeDecodeError::RustProjection {
                message: "Scheme enum variant is missing payload".to_owned(),
            })?;
        seed.deserialize(value.deserializer())
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self
            .value
            .ok_or_else(|| GerbilSchemeTypeDecodeError::RustProjection {
                message: "Scheme enum tuple variant is missing payload".to_owned(),
            })?;
        value.deserializer().deserialize_seq(visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self
            .value
            .ok_or_else(|| GerbilSchemeTypeDecodeError::RustProjection {
                message: "Scheme enum struct variant is missing payload".to_owned(),
            })?;
        value.deserializer().deserialize_map(visitor)
    }
}

struct GerbilSchemeVectorAccess<'de> {
    iter: std::slice::Iter<'de, GerbilSchemeValue>,
}

impl<'de> SeqAccess<'de> for GerbilSchemeVectorAccess<'de> {
    type Error = GerbilSchemeTypeDecodeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(|value| seed.deserialize(value.deserializer()))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct GerbilSchemeRecordAccess<'de> {
    iter: std::collections::btree_map::Iter<'de, String, GerbilSchemeValue>,
    value: Option<&'de GerbilSchemeValue>,
}

impl<'de> MapAccess<'de> for GerbilSchemeRecordAccess<'de> {
    type Error = GerbilSchemeTypeDecodeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.iter.next() else {
            return Ok(None);
        };

        self.value = Some(value);
        seed.deserialize(key.as_str().into_deserializer()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value =
            self.value
                .take()
                .ok_or_else(|| GerbilSchemeTypeDecodeError::RustProjection {
                    message: "Scheme record value requested before key".to_owned(),
                })?;
        seed.deserialize(value.deserializer())
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

fn unexpected_scheme_value(value: &GerbilSchemeValue) -> de::Unexpected<'_> {
    match value {
        GerbilSchemeValue::Null => de::Unexpected::Unit,
        GerbilSchemeValue::Boolean(value) => de::Unexpected::Bool(*value),
        GerbilSchemeValue::Integer(value) => de::Unexpected::Signed(*value),
        GerbilSchemeValue::Float(value) => de::Unexpected::Float(*value),
        GerbilSchemeValue::Text(value) => de::Unexpected::Str(value),
        GerbilSchemeValue::Vector(_) => de::Unexpected::Seq,
        GerbilSchemeValue::Record(_) => de::Unexpected::Map,
    }
}

impl fmt::Display for GerbilSchemeValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),
            Self::Boolean(value) => write!(formatter, "{value}"),
            Self::Integer(value) => write!(formatter, "{value}"),
            Self::Float(value) => write!(formatter, "{value}"),
            Self::Text(value) => formatter.write_str(value),
            Self::Vector(values) => write!(formatter, "vector(len={})", values.len()),
            Self::Record(fields) => write!(formatter, "record(len={})", fields.len()),
        }
    }
}

/// Stable envelope for Scheme values whose concrete Rust projection may evolve downstream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypedValue {
    pub type_id: GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<GerbilSchemeSchemaId>,
    pub value: GerbilSchemeValue,
}

impl GerbilSchemeTypedValue {
    pub fn new(type_id: GerbilSchemeTypeId, value: GerbilSchemeValue) -> Self {
        Self {
            type_id,
            schema_id: None,
            value,
        }
    }

    pub fn with_schema_id(mut self, schema_id: GerbilSchemeSchemaId) -> Self {
        self.schema_id = Some(schema_id);
        self
    }

    pub fn type_id(&self) -> &GerbilSchemeTypeId {
        &self.type_id
    }

    pub fn schema_id(&self) -> Option<&GerbilSchemeSchemaId> {
        self.schema_id.as_ref()
    }

    pub fn value(&self) -> &GerbilSchemeValue {
        &self.value
    }

    pub fn ensure_type(
        &self,
        expected_type_id: &GerbilSchemeTypeId,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.type_id == *expected_type_id {
            return Ok(());
        }

        Err(GerbilSchemeTypeDecodeError::TypeMismatch {
            expected: expected_type_id.clone(),
            actual: self.type_id.clone(),
        })
    }

    pub fn ensure_schema(
        &self,
        expected_schema_id: &GerbilSchemeSchemaId,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.schema_id() == Some(expected_schema_id) {
            return Ok(());
        }

        Err(GerbilSchemeTypeDecodeError::SchemaMismatch {
            type_id: self.type_id.clone(),
            expected: Some(expected_schema_id.clone()),
            actual: self.schema_id.clone(),
        })
    }

    pub fn ensure_type_and_schema(
        &self,
        expected_type_id: &GerbilSchemeTypeId,
        expected_schema_id: Option<&GerbilSchemeSchemaId>,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        self.ensure_type(expected_type_id)?;

        if let Some(expected_schema_id) = expected_schema_id {
            self.ensure_schema(expected_schema_id)?;
        }

        Ok(())
    }

    pub fn ensure_projection_contract(
        &self,
        contract: &GerbilSchemeProjectionContract,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        self.ensure_type_and_schema(contract.type_id(), contract.schema_id())
    }

    pub fn decode_value<T>(&self) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        T::deserialize(self.value.deserializer())
    }

    pub fn decode_value_as<T>(
        &self,
        expected_type_id: &GerbilSchemeTypeId,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        self.ensure_type(expected_type_id)?;
        self.decode_value()
    }

    pub fn decode_value_with_contract<T>(
        &self,
        contract: &GerbilSchemeProjectionContract,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        self.ensure_projection_contract(contract)?;
        self.decode_value()
    }

    pub fn decode_projection<T>(&self) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: GerbilSchemeTypedProjection,
    {
        let contract = T::scheme_projection_contract();
        self.decode_value_with_contract(&contract)
    }
}

/// Decode a serialized typed-value fixture.
///
/// This is not the native ABI path. Runtime bindings should construct `GerbilSchemeTypedValue`
/// from Scheme native data and use the registry for Rust-side validation/projection.
pub fn decode_gerbil_scheme_typed_value_fixture(
    fixture: &str,
) -> Result<GerbilSchemeTypedValue, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(fixture).map_err(|error| GerbilSchemeTypeDecodeError::SerializedFixture {
        message: error.to_string(),
    })
}
