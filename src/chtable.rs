use crate::gamma::{gamma_length, parse_gamma, write_gamma, Gamma};
use crate::helpers::{new_args_iter_with, until_magic};
use binrw::{
    binrw,
    io::{Read, Seek},
    BinRead, BinResult, BinWrite, Endian,
};
use serde::{Deserialize, Serialize};

#[binrw]
#[brw(big)]
#[br(import(header: &Vec<TableHeaderProperty>))]
#[derive(Debug, Deserialize, Serialize)]
pub struct ChTableElement {
    // Actual length = size - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (data.iter().map(|x| x.1.byte_len()).sum::<usize>() + leftover.len() + 1).try_into().unwrap() })]
    pub size: Gamma,
    #[br(if(size.value > 1), parse_with = new_args_iter_with(header, |r,e,props| -> BinResult<(String, TableData)> {
        Ok((props.key.to_string(), TableData::read_options(r, e, (props,))?))
    }))]
    #[bw(map = |data| -> Vec<TableData> { data.iter().map(|x| x.1.clone()).collect() })]
    #[serde(with = "tuple_vec_map")]
    pub data: Vec<(String, TableData)>,
    // Sometimes there's some leftover data here
    #[br(count(size.value as usize - 1 - (data.iter().map(|x| x.1.byte_len()).sum::<usize>())))]
    pub leftover: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[br(import(header: &Vec<TableHeaderProperty>))]
#[derive(Debug, Deserialize, Serialize)]
pub struct ChSparseTableElement {
    // Actual length = size - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (data.iter().map(|x| x.1.byte_len()).sum::<usize>() + leftover.len() + gamma_length(*index) as usize + 1).try_into().unwrap() })]
    pub size: Gamma,
    #[br(parse_with = parse_gamma)]
    #[bw(write_with = write_gamma)]
    pub index: u32,
    #[br(if(size.value > 1), parse_with = new_args_iter_with(header, |r,e,props| -> BinResult<(String, TableData)> {
        Ok((props.key.to_string(), TableData::read_options(r, e, (props,))?))
    }))]
    #[bw(map = |data| -> Vec<TableData> { data.iter().map(|x| x.1.clone()).collect() })]
    #[serde(with = "tuple_vec_map")]
    pub data: Vec<(String, TableData)>,
    // Sometimes there's some leftover data here
    #[br(count(size.value as usize - 1 - (data.iter().map(|x| x.1.byte_len()).sum::<usize>()) - gamma_length(index) as usize))]
    pub leftover: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug, Deserialize, Serialize)]
pub struct StructHeaderProperty {
    data_type: SleType,
    #[br(temp)]
    #[bw(calc = Gamma { value: key.len().try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    key: String,
}

impl StructHeaderProperty {
    fn byte_len(&self) -> usize {
        1 + (gamma_length(self.key.len().try_into().unwrap()) as usize) + self.key.len()
    }
}

#[binrw]
#[brw(big)]
#[derive(Debug, Deserialize, Serialize)]
pub struct StructHeader {
    #[br(parse_with = until_magic(0u8))]
    #[bw(pad_after = 1)]
    properties: Vec<StructHeaderProperty>,
    #[br(count = properties.iter().filter(|x| x.data_type == SleType::Struct).count())]
    sub_headers: Vec<StructHeader>,
}

impl StructHeader {
    pub fn byte_len(&self) -> usize {
        self.properties.iter().map(|x| x.byte_len()).sum::<usize>()
            + 1 // Terminator
            + self.sub_headers.iter().map(|x| x.byte_len()).sum::<usize>()
    }
}

impl From<StructHeader> for Vec<TableHeaderProperty> {
    fn from(header: StructHeader) -> Self {
        let StructHeader {
            properties,
            sub_headers,
        } = header;

        let mut sub_header_iter = sub_headers.into_iter();

        properties
            .into_iter()
            .map(|x| {
                let key = x.key;
                match x.data_type {
                    SleType::Int8 => TableHeaderProperty {
                        data_type: TableDataType::Int8,
                        key,
                    },
                    SleType::UInt8 => TableHeaderProperty {
                        data_type: TableDataType::UInt8,
                        key,
                    },
                    SleType::Int16 => TableHeaderProperty {
                        data_type: TableDataType::Int16,
                        key,
                    },
                    SleType::UInt16 => TableHeaderProperty {
                        data_type: TableDataType::UInt16,
                        key,
                    },
                    SleType::Int32 => TableHeaderProperty {
                        data_type: TableDataType::Int32,
                        key,
                    },
                    SleType::UInt32 => TableHeaderProperty {
                        data_type: TableDataType::UInt32,
                        key,
                    },
                    SleType::Int64 => TableHeaderProperty {
                        data_type: TableDataType::Int64,
                        key,
                    },
                    SleType::UInt64 => TableHeaderProperty {
                        data_type: TableDataType::UInt64,
                        key,
                    },
                    SleType::StringId => TableHeaderProperty {
                        data_type: TableDataType::StringId,
                        key,
                    },
                    SleType::Str => TableHeaderProperty {
                        data_type: TableDataType::Str,
                        key,
                    },
                    SleType::Struct => TableHeaderProperty {
                        data_type: TableDataType::Struct(sub_header_iter.next().unwrap().into()),
                        key,
                    },
                    SleType::Int8List => TableHeaderProperty {
                        data_type: TableDataType::Int8List,
                        key,
                    },
                    SleType::UInt8List => TableHeaderProperty {
                        data_type: TableDataType::UInt8List,
                        key,
                    },
                    SleType::Int16List => TableHeaderProperty {
                        data_type: TableDataType::Int16List,
                        key,
                    },
                    SleType::UInt16List => TableHeaderProperty {
                        data_type: TableDataType::UInt16List,
                        key,
                    },
                    SleType::Int32List => TableHeaderProperty {
                        data_type: TableDataType::Int32List,
                        key,
                    },
                    SleType::UInt32List => TableHeaderProperty {
                        data_type: TableDataType::UInt32List,
                        key,
                    },
                    SleType::Int64List => TableHeaderProperty {
                        data_type: TableDataType::Int64List,
                        key,
                    },
                    SleType::UInt64List => TableHeaderProperty {
                        data_type: TableDataType::UInt64List,
                        key,
                    },
                    SleType::StringIdList => TableHeaderProperty {
                        data_type: TableDataType::StringIdList,
                        key,
                    },
                }
            })
            .collect()
    }
}

impl Into<StructHeader> for Vec<TableHeaderProperty> {
    fn into(self) -> StructHeader {
        let mut sub_headers = vec![];

        StructHeader {
            properties: self
                .into_iter()
                .map(|x| StructHeaderProperty {
                    key: x.key,
                    data_type: match x.data_type {
                        TableDataType::Int8 => SleType::Int8,
                        TableDataType::UInt8 => SleType::UInt8,
                        TableDataType::Int16 => SleType::Int16,
                        TableDataType::UInt16 => SleType::UInt16,
                        TableDataType::Int32 => SleType::Int32,
                        TableDataType::UInt32 => SleType::UInt32,
                        TableDataType::Int64 => SleType::Int64,
                        TableDataType::UInt64 => SleType::UInt64,
                        TableDataType::StringId => SleType::StringId,
                        TableDataType::Str => SleType::Str,
                        TableDataType::Struct(struct_header) => {
                            sub_headers.push(struct_header.into());
                            SleType::Struct
                        }
                        TableDataType::Int8List => SleType::Int8List,
                        TableDataType::UInt8List => SleType::UInt8List,
                        TableDataType::Int16List => SleType::Int16List,
                        TableDataType::UInt16List => SleType::UInt16List,
                        TableDataType::Int32List => SleType::Int32List,
                        TableDataType::UInt32List => SleType::UInt32List,
                        TableDataType::Int64List => SleType::Int64List,
                        TableDataType::UInt64List => SleType::UInt64List,
                        TableDataType::StringIdList => SleType::StringIdList,
                    },
                })
                .collect(),
            sub_headers,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TableHeaderProperty {
    key: String,
    data_type: TableDataType,
}

#[binrw]
#[br(import(header: &TableHeaderProperty))]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum TableData {
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int8)))]
    Int8(i8),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt8)))]
    UInt8(u8),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int16)))]
    Int16(i16),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt16)))]
    UInt16(u16),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int32)))]
    Int32(i32),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt32)))]
    UInt32(u32),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int64)))]
    Int64(i64),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt64)))]
    UInt64(u64),
    #[br(pre_assert(matches!(header.data_type, TableDataType::StringId)))]
    StringId(u16),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Str)))]
    Str(
        #[br(map = |x: TableString| x.value)]
        #[bw(map = |x: &String| TableString { value: x.clone() })]
        String,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Struct(_))))]
    Struct(
        #[br(args(match &header.data_type { TableDataType::Struct(x) => &x, _ => unreachable!() }))]
         TableStruct,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int8List)))]
    Int8List(
        #[br(map = |x: TableDataList<i8>| x.data)]
        #[bw(map = |x: &Vec<i8>| TableDataList::<i8> { data: x.clone() })]
        Vec<i8>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt8List)))]
    UInt8List(
        #[br(map = |x: TableDataList<u8>| x.data)]
        #[bw(map = |x: &Vec<u8>| TableDataList::<u8> { data: x.clone()})]
        Vec<u8>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int16List)))]
    Int16List(
        #[br(map = |x: TableDataList<i16>| x.data)]
        #[bw(map = |x: &Vec<i16>| TableDataList::<i16> { data: x.clone()})]
        Vec<i16>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt16List)))]
    UInt16List(
        #[br(map = |x: TableDataList<u16>| x.data)]
        #[bw(map = |x: &Vec<u16>| TableDataList::<u16> { data: x.clone()})]
        Vec<u16>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int32List)))]
    Int32List(
        #[br(map = |x: TableDataList<i32>| x.data)]
        #[bw(map = |x: &Vec<i32>| TableDataList::<i32> { data: x.clone()})]
        Vec<i32>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt32List)))]
    UInt32List(
        #[br(map = |x: TableDataList<u32>| x.data)]
        #[bw(map = |x: &Vec<u32>| TableDataList::<u32> { data: x.clone()})]
        Vec<u32>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::Int64List)))]
    Int64List(
        #[br(map = |x: TableDataList<i64>| x.data)]
        #[bw(map = |x: &Vec<i64>| TableDataList::<i64> { data: x.clone()})]
        Vec<i64>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::UInt64List)))]
    UInt64List(
        #[br(map = |x: TableDataList<u64>| x.data)]
        #[bw(map = |x: &Vec<u64>| TableDataList::<u64> { data: x.clone()})]
        Vec<u64>,
    ),
    #[br(pre_assert(matches!(header.data_type, TableDataType::StringIdList)))]
    StringIdList(
        #[br(map = |x: TableDataList<u16>| x.data)]
        #[bw(map = |x: &Vec<u16>| TableDataList::<u16> { data: x.clone()})]
        Vec<u16>,
    ),
}

impl TableData {
    pub fn byte_len(&self) -> usize {
        match self {
            TableData::Int8(_) => 1,
            TableData::UInt8(_) => 1,
            TableData::Int16(_) => 2,
            TableData::UInt16(_) => 2,
            TableData::Int32(_) => 4,
            TableData::UInt32(_) => 4,
            TableData::Int64(_) => 8,
            TableData::UInt64(_) => 8,
            TableData::StringId(_) => 2,
            TableData::Str(str) => gamma_length(str.len() as u32) as usize + str.len(),
            TableData::Struct(list) => {
                gamma_length(list.data.len() as u32) as usize
                    + list
                        .data
                        .iter()
                        .map(|props: &Vec<(String, TableData)>| {
                            props.iter().map(|x| x.1.byte_len()).sum::<usize>()
                        })
                        .sum::<usize>()
            }
            TableData::Int8List(list) => gamma_length(list.len() as u32) as usize + list.len(),
            TableData::UInt8List(list) => gamma_length(list.len() as u32) as usize + list.len(),
            TableData::Int16List(list) => gamma_length(list.len() as u32) as usize + list.len() * 2,
            TableData::UInt16List(list) => {
                gamma_length(list.len() as u32) as usize + list.len() * 2
            }
            TableData::Int32List(list) => gamma_length(list.len() as u32) as usize + list.len() * 4,
            TableData::UInt32List(list) => {
                gamma_length(list.len() as u32) as usize + list.len() * 4
            }
            TableData::Int64List(list) => gamma_length(list.len() as u32) as usize + list.len() * 8,
            TableData::UInt64List(list) => {
                gamma_length(list.len() as u32) as usize + list.len() * 8
            }
            TableData::StringIdList(list) => {
                gamma_length(list.len() as u32) as usize + list.len() * 2
            }
        }
    }
}

#[binrw]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[br(import(header: &Vec<TableHeaderProperty>))]
pub struct TableStruct {
    #[br(temp)]
    #[bw(calc = Gamma { value: data.len().try_into().unwrap() })]
    size: Gamma,
    #[br(parse_with = |r,e,a| core::iter::repeat_with(|| new_args_iter_with(header, parse)(r,e,a))
                .take(size.value as usize)
                .collect())]
    #[bw(map = |data| -> Vec<Vec<TableData>> { data.iter().map(|x| x.iter().map(|x| x.1.clone()).collect()).collect() })]
    pub data: Vec<Vec<(String, TableData)>>,
}

fn parse<Reader: Read + Seek>(
    reader: &mut Reader,
    endian: Endian,
    props: &TableHeaderProperty,
) -> BinResult<(String, TableData)> {
    Ok((
        props.key.to_string(),
        TableData::read_options(reader, endian, (props,))?,
    ))
}

#[binrw]
#[derive(Debug)]
pub struct TableString {
    #[br(temp)]
    #[bw(calc = Gamma { value: (value.len()).try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    value: String,
}

#[binrw]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TableDataList<T>
where
    T: for<'a> BinRead<Args<'a> = ()> + for<'a> BinWrite<Args<'a> = ()> + 'static,
{
    #[br(temp)]
    #[bw(calc = Gamma { value: data.len().try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value)]
    data: Vec<T>,
}

#[binrw]
#[brw(repr(u8))]
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum SleType {
    Int8 = 1,
    UInt8 = 2,
    Int16 = 3,
    UInt16 = 4,
    Int32 = 5,
    UInt32 = 6,
    Int64 = 7,
    UInt64 = 8,
    StringId = 9,
    Str = 0b11010,
    Struct = 0b11011,
    Int8List = 0b10001,
    UInt8List = 0b10010,
    Int16List = 0b10011,
    UInt16List = 0b10100,
    Int32List = 0b10101,
    UInt32List = 0b10110,
    Int64List = 0b10111,
    UInt64List = 0b11000,
    StringIdList = 0b11001,
}

macro_rules! named_unit_variant {
    ($variant:ident) => {
        pub mod $variant {
            pub fn serialize<S>(serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(stringify!($variant))
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = ();

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str(concat!("\"", stringify!($variant), "\""))
                    }

                    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                        if value == stringify!($variant) {
                            Ok(())
                        } else {
                            Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
                        }
                    }
                }
                deserializer.deserialize_str(V)
            }
        }
    };
}

#[allow(non_snake_case)]
mod named {
    named_unit_variant!(Int8);
    named_unit_variant!(UInt8);
    named_unit_variant!(Int16);
    named_unit_variant!(UInt16);
    named_unit_variant!(Int32);
    named_unit_variant!(UInt32);
    named_unit_variant!(Int64);
    named_unit_variant!(UInt64);
    named_unit_variant!(StringId);
    named_unit_variant!(Str);
    named_unit_variant!(Int8List);
    named_unit_variant!(UInt8List);
    named_unit_variant!(Int16List);
    named_unit_variant!(UInt16List);
    named_unit_variant!(Int32List);
    named_unit_variant!(UInt32List);
    named_unit_variant!(Int64List);
    named_unit_variant!(UInt64List);
    named_unit_variant!(StringIdList);
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TableDataType {
    #[serde(with = "named::Int8")]
    Int8,
    #[serde(with = "named::UInt8")]
    UInt8,
    #[serde(with = "named::Int16")]
    Int16,
    #[serde(with = "named::UInt16")]
    UInt16,
    #[serde(with = "named::Int32")]
    Int32,
    #[serde(with = "named::UInt32")]
    UInt32,
    #[serde(with = "named::Int64")]
    Int64,
    #[serde(with = "named::UInt64")]
    UInt64,
    #[serde(with = "named::StringId")]
    StringId,
    #[serde(with = "named::Str")]
    Str,
    Struct(Vec<TableHeaderProperty>),
    #[serde(with = "named::Int8List")]
    Int8List,
    #[serde(with = "named::UInt8List")]
    UInt8List,
    #[serde(with = "named::Int16List")]
    Int16List,
    #[serde(with = "named::UInt16List")]
    UInt16List,
    #[serde(with = "named::Int32List")]
    Int32List,
    #[serde(with = "named::UInt32List")]
    UInt32List,
    #[serde(with = "named::Int64List")]
    Int64List,
    #[serde(with = "named::UInt64List")]
    UInt64List,
    #[serde(with = "named::StringIdList")]
    StringIdList,
}
