use std::{
    fs::File,
    io::{Cursor, Read, Result, Write},
};

use binrw::{args, binrw, until_eof, BinRead, BinReaderExt, BinWrite, NamedArgs};
use gamma::{gamma_length, parse_gamma, write_gamma};
use helpers::until_magic;
use modular_bitfield::{bitfield, specifiers::B4};
use xz2::{read::XzDecoder, write::XzEncoder};

mod charray;
mod gamma;
mod helpers;
mod jgr;

#[binrw]
#[derive(Debug)]
enum CompressionType {
    // Compressed with LZO (deprecated, only really old savegames would use this).
    #[brw(magic = b"OTTD")]
    OTTD,
    // No compression.
    #[brw(magic = b"OTTN")]
    OTTN,
    // Compressed with zlib.
    #[brw(magic = b"OTTZ")]
    OTTZ,
    // Compressed with LZMA.
    #[brw(magic = b"OTTX")]
    OTTX,
}

#[binrw]
#[brw(repr(u8))]
#[derive(Debug, Clone, PartialEq)]
enum SleType {
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

fn compress_save(compression_type: &CompressionType, blob: &Vec<u8>) -> Vec<u8> {
    match compression_type {
        CompressionType::OTTD => {
            panic!("Old save file not supported")
        }
        CompressionType::OTTN => blob.clone(),
        CompressionType::OTTZ => {
            panic!("zlib compression not supported (yet)")
        }
        CompressionType::OTTX => {
            let mut buffer: Vec<u8> = Vec::new();

            {
                let mut encoder = XzEncoder::new(&mut buffer, 2);
                encoder.write(&blob).unwrap();
            }

            buffer
        }
    }
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
struct OuterContainer {
    compression_type: CompressionType,
    // The next two bytes indicate which savegame version used.
    version: u16,
    // The next two bytes can be ignored, and were only used in really old savegames.
    _ignore: u16,
    // Wish I could use map_stream here from the new PR but no rust LZMA decompressers support Read + Seek :(
    #[br(parse_with = until_eof, map = |blob: Vec<u8>| {
        match &compression_type {
            CompressionType::OTTD => {
                panic!("Old save file not supported")
            },
            CompressionType::OTTN => {
                blob
            },
            CompressionType::OTTZ => {
                panic!("zlib compression not supported (yet)")
            },
            CompressionType::OTTX => {
                let mut buffer = Vec::new();
                XzDecoder::new(Cursor::new(&blob)).read_to_end(&mut buffer).expect("Failed to decompress save");
                buffer
            }
        }
    })]
    #[bw(map = |blob: &Vec<u8>| compress_save(&compression_type, blob))]
    data: Vec<u8>,
}

#[binrw]
#[derive(Debug)]
#[brw(big)]
struct Chunk {
    tag: [u8; 4],
    // #[br(map = |x: [u8; 4]|  String::from_utf8_lossy(&x).to_string() )]
    // #[bw(map = |x| TryInto::<[u8; 4]>::try_into(x.as_bytes()).unwrap())]
    // tag: String,
    #[br(temp)]
    #[bw(calc = ChunkType::get_chunk_type(value))]
    chunk_type: ChunkType,
    #[br(args { chunk_type })]
    value: ChunkValue,
}

#[binrw]
struct Gamma {
    #[br(parse_with = parse_gamma)]
    #[bw(write_with = write_gamma)]
    value: u32,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
struct ChArrayElement {
    // Actual length = size - 1
    // #[br(parse_with = parse_gamma, temp)]
    // #[bw(write_with = write_gamma, calc = data.len().try_into().unwrap())]
    // size: u32,
    #[br(temp)]
    #[bw(calc = Gamma { value: (data.len() + 1).try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value.saturating_sub(1))]
    data: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
struct ChSparseArrayElement {
    // Actual length = length - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (TryInto::<u32>::try_into(data.len() + 1).unwrap()) + gamma_length(*index) })]
    size: Gamma,
    #[br(parse_with = parse_gamma)]
    #[bw(write_with = write_gamma)]
    pub index: u32,
    #[br(count = size.value.saturating_sub(1 + gamma_length(index)))]
    data: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
struct HeaderProperty {
    data_type: SleType,
    #[br(temp)]
    #[bw(calc = Gamma { value: key.len().try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    key: String,
}

impl HeaderProperty {
    fn byte_len(&self) -> usize {
        1 + (gamma_length(self.key.len().try_into().unwrap()) as usize) + self.key.len()
    }
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
struct StructHeader {
    // #[br(parse_with = parse_gamma)]
    // header_size: u32,
    // #[br(count = header_size.saturating_sub(1))]
    // data: Vec<u8>,
    #[br(parse_with = until_magic(0u8))]
    #[bw(pad_after = 1)]
    properties: Vec<HeaderProperty>,
    #[br(count = properties.iter().filter(|x| x.data_type == SleType::Struct).count())]
    sub_headers: Vec<StructHeader>,
}

impl StructHeader {
    fn byte_len(&self) -> usize {
        self.properties.iter().map(|x| x.byte_len()).sum::<usize>()
            + 1 // Terminator
            + self.sub_headers.iter().map(|x| x.byte_len()).sum::<usize>()
    }
}

// #[br(import { chunk_type: u8 })]
#[binrw]
#[br(import { sle_type: SleType })]
#[derive(Debug)]
enum TableData {
    #[br(pre_assert(sle_type == SleType::Int8))]
    Int8(i8),
    #[br(pre_assert(sle_type == SleType::UInt8))]
    UInt8(u8),
    #[br(pre_assert(sle_type == SleType::Int16))]
    Int16(i16),
    #[br(pre_assert(sle_type == SleType::UInt16))]
    UInt16(u16),
    #[br(pre_assert(sle_type == SleType::Int32))]
    Int32(i32),
    #[br(pre_assert(sle_type == SleType::UInt32))]
    UInt32(u32),
    #[br(pre_assert(sle_type == SleType::Int64))]
    Int64(i64),
    #[br(pre_assert(sle_type == SleType::UInt64))]
    UInt64(u64),
    #[br(pre_assert(sle_type == SleType::StringId))]
    StringId(u16),
    #[br(pre_assert(sle_type == SleType::Str))]
    Str(TableDataList<u8>),
    #[br(pre_assert(sle_type == SleType::Struct))]
    // TODO Fix
    Struct(TableDataList<u8>),
    #[br(pre_assert(sle_type == SleType::Int8List))]
    Int8List(TableDataList<i8>),
    #[br(pre_assert(sle_type == SleType::UInt8List))]
    UInt8List(TableDataList<u8>),
    #[br(pre_assert(sle_type == SleType::Int16List))]
    Int16List(TableDataList<i16>),
    #[br(pre_assert(sle_type == SleType::UInt16List))]
    UInt16List(TableDataList<u16>),
    #[br(pre_assert(sle_type == SleType::Int32List))]
    Int32List(TableDataList<i32>),
    #[br(pre_assert(sle_type == SleType::UInt32List))]
    UInt32List(TableDataList<u32>),
    #[br(pre_assert(sle_type == SleType::Int64List))]
    Int64List(TableDataList<i64>),
    #[br(pre_assert(sle_type == SleType::UInt64List))]
    UInt64List(TableDataList<u64>),
    #[br(pre_assert(sle_type == SleType::StringIdList))]
    StringIdList(TableDataList<u16>),
}

#[binrw]
#[derive(Debug)]
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

#[bitfield]
#[binrw]
#[derive(Debug, Clone, Copy)]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
pub struct ChunkType {
    chunk_type: B4,
    riff_size: B4,
}

impl ChunkType {
    fn get_chunk_type(chunk: &ChunkValue) -> ChunkType {
        let mut riff_size: u8 = 0;
        let chunk_type = match chunk {
            ChunkValue::ChRiff { data, .. } => {
                riff_size = (data.len() << 24) as u8;
                0
            }
            ChunkValue::ChArray { .. } => 1,
            ChunkValue::ChSparseArray { .. } => 2,
            ChunkValue::ChTable { .. } => 3,
            ChunkValue::ChSparseTable { .. } => 4,
        };

        ChunkType::new()
            .with_riff_size(riff_size)
            .with_chunk_type(chunk_type)
    }
}

#[binrw]
#[brw(big)]
#[br(import { chunk_type: ChunkType })]
#[derive(Debug)]
enum ChunkValue {
    #[br(pre_assert(chunk_type.chunk_type() == 0))]
    ChRiff {
        #[br(map = |x: [u8;3]| ((x[0] as u32) << 16) | ((x[1] as u32) << 8) | (x[2] as u32) | (((chunk_type.riff_size()) as u32) << 24))]
        #[bw(map = |x: &u32| { let bytes = x.to_be_bytes(); [bytes[1], bytes[2], bytes[3]] })]
        size: u32,
        #[br(count = size)]
        data: Vec<u8>,
    },
    #[br(pre_assert(chunk_type.chunk_type() == 1))]
    ChArray {
        #[br(parse_with = until_magic(0u8))]
        #[bw(pad_after = 1)]
        elements: Vec<ChArrayElement>,
    },
    #[br(pre_assert(chunk_type.chunk_type() == 2))]
    ChSparseArray {
        #[br(parse_with = until_magic(0u8))]
        #[bw(pad_after = 1)]
        elements: Vec<ChSparseArrayElement>,
    },
    #[br(pre_assert(chunk_type.chunk_type() == 3))]
    ChTable {
        #[br(temp)]
        #[bw(calc = Gamma { value: (header.byte_len() + 1).try_into().unwrap() })]
        header_size: Gamma,
        header: StructHeader,
        #[br(parse_with = until_magic(0u8))]
        #[bw(pad_after = 1)]
        elements: Vec<ChArrayElement>,
    },
    #[br(pre_assert(chunk_type.chunk_type() == 4))]
    ChSparseTable {
        #[br(temp)]
        #[bw(calc = Gamma { value: (header.byte_len() + 1).try_into().unwrap() })]
        header_size: Gamma,
        header: StructHeader,
        #[br(parse_with = until_magic(0u8))]
        #[bw(pad_after = 1)]
        elements: Vec<ChSparseArrayElement>,
    },
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
struct Chunks {
    #[br(parse_with = until_eof)]
    chunks: Vec<Chunk>,
    #[br(temp)]
    #[bw(calc = 0)]
    terminator: u32,
}

fn main() -> Result<()> {
    let mut f = File::open("TinyJGRTest1.sav")?;

    let outer: OuterContainer = f.read_ne().unwrap();

    println!("{:?}", outer.version);

    let mut out_file = File::create("TutorialOut.sav")?;
    outer.write(&mut out_file).unwrap();

    // println!("Size:{}", outer.data.len());
    let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

    println!(
        "{:?}",
        chunk
            .chunks
            .iter()
            .map(|x| String::from_utf8_lossy(&x.tag).to_string())
            .collect::<Vec<String>>()
            .join(",")
    );

    let array = chunk
        .chunks
        .iter()
        .filter(|x| &x.tag == b"SLXI")
        .last()
        .expect("Couldn't find SLXI, not JGR map?");

    let slxi = match &array.value {
        ChunkValue::ChRiff { data, .. } => {
            let mut c = &mut Cursor::new(&data);
            let slxi: jgr::SLXI = c.read_ne().unwrap();

            // println("{},{},{},{},{}", slxi.has_feature(name))

            slxi
        }
        _ => {
            panic!("SLXI wasn't a CH_RIFF")
        }
    };

    let array = chunk
        .chunks
        .iter()
        .filter(|x| String::from_utf8_lossy(&x.tag) == "CITY")
        .last()
        .unwrap();

    match &array.value {
        ChunkValue::ChArray { elements } => {
            let mut c = &mut Cursor::new(&elements[0].data);

            let mut out_file = File::create("JGR_CITY.sav")?;
            out_file.write(&elements[0].data).unwrap();

            println!("{:?}", elements[0].data);

            let new_vec: charray::City = c
                .read_ne_args::<charray::City>(args! { slxi: &slxi })
                .unwrap();

            println!("{:?}", new_vec);
        }
        ChunkValue::ChTable { header, elements } => {
            let mut c = &mut Cursor::new(&elements[0].data);

            // let new_vec: charray::City = c.read_ne().unwrap();

            // println!("{:?}", new_vec);
        }
        _ => {}
    }

    // let mut d = vec![];
    // let mut writer = Cursor::new(&mut d);
    // Chunks::write_be(&chunk, &mut writer).unwrap();
    // assert_eq!(&outer.data, &d);

    // let mut out_file = File::create("Out.sav")?;
    // OuterContainer { data: d, ..outer }
    //     .write(&mut out_file)
    //     .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use binrw::{BinReaderExt, BinWrite};
    use std::{
        fs::File,
        io::{Cursor, Result},
    };

    use crate::{Chunks, OuterContainer};

    #[test]
    fn parse_and_write_tiny() -> Result<()> {
        let mut f = File::open("tests/test.sav")?;

        let outer: OuterContainer = f.read_ne().unwrap();
        let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

        let mut d = vec![];
        let mut writer = Cursor::new(&mut d);
        Chunks::write_be(&chunk, &mut writer).unwrap();
        assert_eq!(&outer.data, &d);

        Ok(())
    }

    #[test]
    fn parse_and_write_new_tiny() -> Result<()> {
        let mut f = File::open("tests/TinyVanillaTest.sav")?;

        let outer: OuterContainer = f.read_ne().unwrap();
        let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

        let mut d = vec![];
        let mut writer = Cursor::new(&mut d);
        Chunks::write_be(&chunk, &mut writer).unwrap();
        assert_eq!(&outer.data, &d);

        Ok(())
    }
}
