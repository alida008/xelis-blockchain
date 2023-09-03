use std::{collections::HashMap, borrow::Cow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::{serializer::{Serializer, Reader, ReaderError, Writer}, crypto::hash::Hash};

pub mod wallet;
pub mod daemon;

// All types availables
#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum DataType {
    Bool,
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    Hash,
}

// This enum allows complex structures with multi depth if necessary
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DataElement {
    // Value can be Optional to represent null in JSON
    Value(Option<DataValue>),
    // For two next variants, we support up to 255 (u8::MAX) elements maximum
    Array(Vec<DataElement>),
    Fields(HashMap<DataValue, DataElement>)
}

impl DataElement {
    pub fn get_value(&self, name: String, data_type: DataType) -> Option<&DataValue> {
        let Self::Fields(data) = &self else {
            return None
        };

        let Self::Value(value) = data.get(&DataValue::String(name))? else {
            return None;
        };

        let Some(unwrapped) = &value else {
            return None;
        };

        if unwrapped.kind() != data_type {
            return None
        }

        value.as_ref()
    }
} 

impl Serializer for DataElement {
    // Don't do any pre-allocation because of infinite depth
    // Otherwise an attacker could generate big depth with high size until max limit
    // which can create OOM on low devices
    fn read(reader: &mut Reader) -> Result<Self, ReaderError> {
        Ok(match reader.read_u8()? {
            0 => Self::Value(Option::<DataValue>::read(reader)?),
            1 => {
                let size = reader.read_u8()?;
                let mut values = Vec::new();
                for _ in 0..size {
                    values.push(DataElement::read(reader)?)
                }
                Self::Array(values)
            },
            2 => {
                let size = reader.read_u8()?;
                let mut fields = HashMap::new();
                for _ in 0..size {
                    let key = DataValue::read(reader)?;
                    let value = DataElement::read(reader)?;
                    fields.insert(key, value);
                }
                Self::Fields(fields)
            },
            _ => return Err(ReaderError::InvalidValue)
        })
    }

    fn write(&self, writer: &mut Writer) {
        match self {
            Self::Value(value) => {
                writer.write_u8(0);
                value.write(writer);
            }
            Self::Array(values) => {
                writer.write_u8(1);
                writer.write_u8(values.len() as u8); // we accept up to 255 values
                for value in values {
                    value.write(writer);    
                }
            }
            Self::Fields(fields) => {
                writer.write_u8(2);
                writer.write_u8(fields.len() as u8);
                for (key, value) in fields {
                    key.write(writer);
                    value.write(writer);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
#[serde(untagged)]
pub enum DataValue {
    // represent a null value
    Bool(bool),
    String(String),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Hash(Hash),
}

impl DataValue {
    pub fn kind(&self) -> DataType {
        match self {
            Self::Bool(_) => DataType::Bool,
            Self::String(_) => DataType::String,
            Self::U8(_) => DataType::U8,
            Self::U16(_) => DataType::U16,
            Self::U32(_) => DataType::U32,
            Self::U64(_) => DataType::U64,
            Self::U128(_) => DataType::U128,
            Self::Hash(_) => DataType::Hash
        }
    }
}

impl Serializer for DataValue {
    fn read(reader: &mut Reader) -> Result<Self, ReaderError> {
        Ok(match reader.read_u8()? {
            0 => Self::Bool(reader.read_bool()?),
            1 => Self::String(reader.read_string()?),
            2 => Self::U8(reader.read_u8()?),
            3 => Self::U16(reader.read_u16()?),
            4 => Self::U32(reader.read_u32()?),
            5 => Self::U64(reader.read_u64()?),
            6 => Self::U128(reader.read_u128()?),
            7 => Self::Hash(reader.read_hash()?),
            _ => return Err(ReaderError::InvalidValue)
        })
    }

    fn write(&self, writer: &mut Writer) {
        match self {
            Self::Bool(bool) => {
                writer.write_u8(0);
                writer.write_bool(*bool);
            },
            Self::String(string) => {
                writer.write_u8(1);
                writer.write_string(string);
            },
            Self::U8(value) => {
                writer.write_u8(2);
                writer.write_u8(*value);
            },
            Self::U16(value) => {
                writer.write_u8(3);
                writer.write_u16(*value);
            },
            Self::U32(value) => {
                writer.write_u8(4);
                writer.write_u32(value);
            },
            Self::U64(value) => {
                writer.write_u8(5);
                writer.write_u64(value);
            },
            Self::U128(value) => {
                writer.write_u8(6);
                writer.write_u128(value);
            },
            Self::Hash(hash) => {
                writer.write_u8(7);
                writer.write_hash(hash);
            }
        };
    }
}

#[derive(Serialize, Deserialize)]
pub struct SubscribeParams<E> {
    pub notify: E
}

#[derive(Serialize, Deserialize)]
pub struct EventResult<'a, E: Clone> {
    pub event: Cow<'a, E>,
    #[serde(flatten)]
    pub value: Value
}

#[derive(Serialize, Deserialize)]
pub struct DataHash<'a, T: Clone> {
    pub hash: Cow<'a, Hash>,
    #[serde(flatten)]
    pub data: Cow<'a, T>
}
