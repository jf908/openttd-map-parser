use std::io::{Cursor, Read, Result, Write};

use crate::gamma::{gamma_length, parse_gamma, write_gamma, Gamma};
use crate::helpers::{until_magic, until_magic_with};
use binrw::helpers::args_iter;
use binrw::{args, binrw, until_eof, BinRead, BinResult, BinWrite, Endian};

#[binrw]
#[brw(big)]
#[br(import(header: &Vec<TableHeaderProperty>))]
#[derive(Debug)]
pub struct ChTableElement {
    // Actual length = size - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (data.iter().map(|x| x.byte_len()).sum::<usize>() + 1).try_into().unwrap() })]
    size: Gamma,
    #[br(parse_with = args_iter(header.iter().map(|prop: &TableHeaderProperty| -> <TableData as BinRead>::Args<'_> { args! { sle_type: prop.data_type } })))]
    pub data: Vec<TableData>,
    #[br(count = (size.value - 1) as usize - (data.iter().map(|x| x.byte_len()).sum::<usize>()))]
    pub leftover: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct StructHeader {
    #[br(parse_with = until_magic(0u8))]
    #[bw(pad_after = 1)]
    properties: Vec<StructHeaderProperty>,
    #[br(count = properties.iter().filter(|x| x.data_type == SleType::Struct).count())]
    sub_headers: Vec<StructHeader>,
}

impl From<StructHeader> for Vec<TableHeaderProperty> {
    fn from(header: StructHeader) -> Self {
        let StructHeader {
            properties,
            sub_headers,
        } = header;

        let mut i = 0;
        properties
            .into_iter()
            .map(|x| {
                let key = x.key;
                match x.data_type {
                    SleType::Struct => {
                        i += 1;
                        TableHeaderProperty {
                            data_type: SleType::Struct,
                            layout: Some(sub_headers[i - 1].into()),
                            key,
                        }
                    }
                    x => TableHeaderProperty {
                        data_type: x,
                        layout: None,
                        key,
                    },
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct TableHeaderProperty {
    data_type: SleType,
    layout: Option<Vec<TableHeaderProperty>>,
    key: String,
}

impl StructHeader {
    pub fn byte_len(&self) -> usize {
        self.properties.iter().map(|x| x.byte_len()).sum::<usize>()
            + 1 // Terminator
            + self.sub_headers.iter().map(|x| x.byte_len()).sum::<usize>()
    }
}

#[binrw]
#[br(import(header: &TableHeaderProperty))]
#[derive(Debug, Clone)]
pub enum TableData {
    #[br(pre_assert(header.data_type == SleType::Int8))]
    Int8(i8),
    #[br(pre_assert(header.data_type == SleType::UInt8))]
    UInt8(u8),
    #[br(pre_assert(header.data_type == SleType::Int16))]
    Int16(i16),
    #[br(pre_assert(header.data_type == SleType::UInt16))]
    UInt16(u16),
    #[br(pre_assert(header.data_type == SleType::Int32))]
    Int32(i32),
    #[br(pre_assert(header.data_type == SleType::UInt32))]
    UInt32(u32),
    #[br(pre_assert(header.data_type == SleType::Int64))]
    Int64(i64),
    #[br(pre_assert(header.data_type == SleType::UInt64))]
    UInt64(u64),
    #[br(pre_assert(header.data_type == SleType::StringId))]
    StringId(u32),
    #[br(pre_assert(header.data_type == SleType::Str))]
    Str(TableDataList<u8>),
    #[br(pre_assert(header.data_type == SleType::Struct))]
    Struct(TableDataList<u8>),
    #[br(pre_assert(header.data_type == SleType::Int8List))]
    Int8List(TableDataList<i8>),
    #[br(pre_assert(header.data_type == SleType::UInt8List))]
    UInt8List(TableDataList<u8>),
    #[br(pre_assert(header.data_type == SleType::Int16List))]
    Int16List(TableDataList<i16>),
    #[br(pre_assert(header.data_type == SleType::UInt16List))]
    UInt16List(TableDataList<u16>),
    #[br(pre_assert(header.data_type == SleType::Int32List))]
    Int32List(TableDataList<i32>),
    #[br(pre_assert(header.data_type == SleType::UInt32List))]
    UInt32List(TableDataList<u32>),
    #[br(pre_assert(header.data_type == SleType::Int64List))]
    Int64List(TableDataList<i64>),
    #[br(pre_assert(header.data_type == SleType::UInt64List))]
    UInt64List(TableDataList<u64>),
    #[br(pre_assert(header.data_type == SleType::StringIdList))]
    StringIdList(TableDataList<u32>),
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
            TableData::StringId(_) => 32,
            TableData::Str(list) => gamma_length(list.data.len() as u32) as usize + list.data.len(),
            TableData::Struct(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len()
            }
            TableData::Int8List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len()
            }
            TableData::UInt8List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len()
            }
            TableData::Int16List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 2
            }
            TableData::UInt16List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 2
            }
            TableData::Int32List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 4
            }
            TableData::UInt32List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 4
            }
            TableData::Int64List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 8
            }
            TableData::UInt64List(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 8
            }
            TableData::StringIdList(list) => {
                gamma_length(list.data.len() as u32) as usize + list.data.len() * 4
            }
        }
    }
}

#[binrw]
#[derive(Debug, Clone)]
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
#[derive(Debug, Copy, Clone, PartialEq)]
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
