//! Java objects in KBackup-Fabric, with encode and decode logics
//! There are some known issues:
//! - `dateTime` is not implemented. The format is way too complex,
//!   and this field is not necessarily needed. So skip for now.
//! - The implementation of HashMap is not optimal.
//!   The recommended way in doc is to use derive with extract.
//!   But this seems buggy and I cannot get it work. So just use custom type for now.
//!
use jaded::{ConversionError, ConversionResult, FromJava, Value};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use std::hash::Hash;

// #[derive(Debug, Clone)]
// pub struct ZonedDateTime {
//     // #[jaded(field = "dateTime")]
//     // pub date_time: LocalDateTime,
// }
//
// impl FromJava for ZonedDateTime {
//     fn from_value(value: &Value) -> ConversionResult<Self> {
//         match value {
//             Value::Object(obj) => match obj.get_annotation(0) {
//                 Some(mut it) => {
//                     let year = it.read_i32()?;
//                     let month = it.read_u8()?;
//                     let day = it.read_u8()?;
//                     let mut hour = it.read_u8()? as i8;
//                     let mut minute: i8 = 0;
//                     let mut second: i8 = 0;
//                     let mut nano: i32 = 0;
//                     if hour < 0 {
//                         hour = !hour
//                     } else {
//                         minute = it.read_u8()? as i8;
//                         if minute < 0 {
//                             minute = !minute
//                         } else {
//                             second = it.read_u8()? as i8;
//                             if second < 0 {
//                                 second = !second
//                             } else {
//                                 nano = it.read_i32()?;
//                             }
//                         }
//                     }
//                     let offset_byte = it.read_u8()?;
//                     let offset_seconds = if offset_byte == 127 {
//                         it.read_i32()?
//                     } else {
//                         offset_byte as i32 * 900
//                     };
//                     let zone_type = it.read_u8()?;
//                     tracing::info!(
//                         "ymdhms: {} {} {} {} {} {} {} zt: {}",
//                         year,
//                         month,
//                         day,
//                         hour,
//                         minute,
//                         second,
//                         nano,
//                         zone_type,
//                     );
//                     Ok(ZonedDateTime {})
//                 }
//                 None => Err(ConversionError::InvalidType(
//                     "expected object with annotations",
//                 )),
//             },
//             _ => Err(ConversionError::InvalidType("expected object")),
//         }
//     }
// }

#[derive(Debug, FromJava, Serialize)]
pub struct SavedIncBackupV1 {
    #[jaded(field = "objectCollection2")]
    pub object_collection2: ObjectCollection2,
    #[jaded(field = "backupName")]
    pub backup_name: String,
    // #[jaded(field = "backupTime")]
    // pub backup_time: ZonedDateTime,
    #[jaded(field = "totalSizeBytes")]
    pub total_size_bytes: i64,
    #[jaded(field = "increasedSizeBytes")]
    pub increased_size_bytes: i64,
    #[jaded(field = "filesAdded")]
    pub files_added: i32,
    #[jaded(field = "totalFiles")]
    pub total_files: i32,
}

#[derive(Debug, FromJava, Serialize)]
pub struct ObjectCollection2 {
    pub name: String,
    pub elements: JavaHashMap<String, ObjectElement>,
    #[jaded(field = "subCollections")]
    pub sub_collections: JavaHashMap<String, ObjectCollection2>,
}

#[derive(Debug, Serialize)]
pub struct JavaHashMap<K, V>(HashMap<K, V>);

impl<K, V> From<HashMap<K, V>> for JavaHashMap<K, V> {
    fn from(value: HashMap<K, V>) -> Self {
        Self(value)
    }
}

impl<K, V> Into<HashMap<K, V>> for JavaHashMap<K, V> {
    fn into(self) -> HashMap<K, V> {
        self.0
    }
}

impl<K: FromJava + Eq + Hash, V: FromJava> FromJava for JavaHashMap<K, V> {
    fn from_value(value: &Value) -> ConversionResult<Self> {
        match value {
            Value::Object(map_obj) => {
                tracing::debug!("HashMap annotations: {}", map_obj.annotation_count());
                match map_obj.get_annotation(0) {
                    Some(mut v) => {
                        let _ = v.read_i32()?; // discard the first i32, as java HashMap::readObject implemented
                        let cnt = v.read_i32()?;
                        tracing::debug!("HashMap elements: {}", cnt);
                        let mut result = HashMap::with_capacity(cnt as usize);
                        for _ in 0..cnt {
                            let key = v.read_object_as().expect("read key");
                            let value = v.read_object_as().expect("read value");
                            result.insert(key, value);
                        }
                        Ok(result.into())
                    }
                    None => Err(ConversionError::InvalidType(
                        "expected HashMap with one annotation",
                    )),
                }
            }
            _ => Err(ConversionError::InvalidType("expected java HashMap object"))?,
        }
    }
}

// this function does not work, not sure why
// fn read_hashmap<K: FromJava + Eq + Hash, V: FromJava>(
//     annotations: &mut AnnotationIter,
// ) -> ConversionResult<HashMap<K, V>> {
//     let mut result = HashMap::new();
//     for x in 0..annotations.read_i32()? {
//         result.insert(annotations.read_object_as()?, annotations.read_object_as()?);
//     }
//     Ok(result)
// }

#[derive(Debug, FromJava, Serialize)]
pub struct ObjectElement {
    pub name: String,
    pub identifier: SingleHashIdentifier,
}

#[derive(Debug, FromJava)]
pub struct SingleHashIdentifier {
    // Java: private final String type;
    #[jaded(field = "type")]
    pub typ: String,
    // Java: private final byte[] hash;
    pub hash: Vec<u8>,
}

impl Display for SingleHashIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.typ)?;
        f.write_char('-')?;
        f.write_str(&*hex::encode_upper(&self.hash))
    }
}

impl Serialize for SingleHashIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
