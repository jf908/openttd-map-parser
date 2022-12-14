use crate::chtable::{ChSparseTableElement, ChTableElement, StructHeader, TableHeaderProperty};
use crate::gamma::{gamma_length, parse_gamma, write_gamma, Gamma};
use crate::helpers::{until_magic, until_magic_with};
use binrw::{
    binrw,
    io::{Cursor, Read, Result, Write},
    until_eof, BinRead, BinResult, BinWrite,
};
use modular_bitfield::{bitfield, specifiers::B4};
use xz2::{read::XzDecoder, write::XzEncoder};

#[binrw]
#[derive(Debug)]
pub enum CompressionType {
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
    // Compressed with zstd (from JGR, not in vanilla OpenTTD)
    #[brw(magic = b"OTTS")]
    OTTS,
}

fn compress_save(compression_type: &CompressionType, blob: &Vec<u8>) -> Result<Vec<u8>> {
    match compression_type {
        CompressionType::OTTD => {
            panic!("Old save file not supported")
        }
        CompressionType::OTTN => Ok(blob.clone()),
        CompressionType::OTTZ => {
            panic!("zlib compression not supported (yet)")
        }
        CompressionType::OTTX => {
            let mut buffer: Vec<u8> = Vec::new();

            {
                let mut encoder = XzEncoder::new(&mut buffer, 2);
                encoder.write(&blob).unwrap();
            }

            Ok(buffer)
        }
        CompressionType::OTTS => zstd::stream::encode_all(&mut Cursor::new(blob), 0),
    }
}

fn decompress_save(compression_type: &CompressionType, blob: Vec<u8>) -> Result<Vec<u8>> {
    match compression_type {
        CompressionType::OTTD => {
            panic!("Old save file not supported")
        }
        CompressionType::OTTN => Ok(blob),
        CompressionType::OTTZ => {
            panic!("zlib compression not supported (yet)")
        }
        CompressionType::OTTX => {
            let mut buffer = Vec::new();
            XzDecoder::new(Cursor::new(&blob)).read_to_end(&mut buffer)?;
            Ok(buffer)
        }
        CompressionType::OTTS => zstd::stream::decode_all(&mut Cursor::new(blob)),
    }
}

#[binrw::parser(reader, endian)]
fn chunk_reader(compression_type: &CompressionType) -> BinResult<Vec<Chunk>> {
    match compression_type {
        CompressionType::OTTD => {
            panic!("Old save file not supported")
        }
        CompressionType::OTTN => Chunks::read_options(reader, endian, ()),
        CompressionType::OTTZ => {
            panic!("zlib compression not supported (yet)")
        }
        CompressionType::OTTX => {
            let mut buffer = Vec::new();
            let mut new_reader = XzDecoder::new(reader);
            new_reader.read_to_end(&mut buffer)?;
            Chunks::read_options(&mut Cursor::new(&mut buffer), endian, ())
        }
        CompressionType::OTTS => {
            let decoder = zstd::stream::decode_all(reader)?;
            Chunks::read_options(&mut Cursor::new(decoder), endian, ())
        }
    }
    .map(|c| c.chunks)
}

// Note that this function does not write the 4 byte terminator at the end of the file
#[binrw::writer(writer, endian)]
fn chunk_writer(chunks: &Vec<Chunk>, compression_type: &CompressionType) -> BinResult<()> {
    match compression_type {
        CompressionType::OTTD => {
            panic!("Old save file not supported")
        }
        CompressionType::OTTN => {
            chunks.write_options(writer, endian, ())?;
            // Terminator
            0u32.write_options(writer, endian, ())
        }
        CompressionType::OTTZ => {
            panic!("zlib compression not supported (yet)")
        }
        CompressionType::OTTX => {
            let mut buffer: Vec<u8> = Vec::new();

            {
                let mut writer = Cursor::new(&mut buffer);
                chunks.write_options(&mut writer, endian, ())?;
                // Terminator
                0u32.write_options(&mut writer, endian, ())?;
            }

            let mut encoder = XzEncoder::new(writer, 2);
            encoder.write_all(&mut buffer)?;

            Ok(())
        }
        CompressionType::OTTS => {
            let mut buffer: Vec<u8> = Vec::new();

            {
                let mut writer = Cursor::new(&mut buffer);
                chunks.write_options(&mut writer, endian, ())?;
                // Terminator
                0u32.write_options(&mut writer, endian, ())?;
            }

            zstd::stream::copy_encode(&mut Cursor::new(buffer), writer, 0)?;

            Ok(())
        }
    }
}

#[binrw]
struct RawChunk {
    #[br(parse_with = until_eof)]
    data: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct Save {
    pub compression_type: CompressionType,
    // The next two bytes indicate which savegame version used.
    pub version: u16,
    // The next two bytes can be ignored, and were only used in really old savegames.
    _ignore: u16,
    // Wish I could use map_stream here from the new PR but no rust LZMA decompressers support Read + Seek :(
    #[br(parse_with = |r,e,_: ()| chunk_reader(r, e, &compression_type))]
    #[bw(write_with = |r,e,d,_: ()| chunk_writer(r, e, d, &compression_type))]
    pub chunks: Vec<Chunk>,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct OuterSave {
    pub compression_type: CompressionType,
    // The next two bytes indicate which savegame version used.
    pub version: u16,
    // The next two bytes can be ignored, and were only used in really old savegames.
    _ignore: u16,
    #[br(parse_with = until_eof, try_map = |blob: Vec<u8>| decompress_save(&compression_type, blob))]
    #[bw(try_map = |blob: &Vec<u8>| compress_save(&compression_type, blob))]
    pub data: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct Chunks {
    #[br(parse_with = until_eof)]
    pub chunks: Vec<Chunk>,
    #[br(temp)]
    #[bw(calc = 0)]
    terminator: u32,
}

#[binrw]
#[derive(Debug)]
#[brw(big)]
pub struct Chunk {
    pub tag: [u8; 4],
    #[br(temp)]
    #[bw(calc = ChunkType::get_chunk_type(value))]
    chunk_type: ChunkType,
    #[br(args { chunk_type })]
    pub value: ChunkValue,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct ChArrayElement {
    // Actual length = size - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (data.len() + 1).try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value.saturating_sub(1))]
    pub data: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct ChSparseArrayElement {
    // Actual length = length - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (TryInto::<u32>::try_into(data.len() + 1).unwrap()) + gamma_length(*index) })]
    size: Gamma,
    #[br(parse_with = parse_gamma)]
    #[bw(write_with = write_gamma)]
    pub index: u32,
    #[br(count = size.value.saturating_sub(1 + gamma_length(index)))]
    pub data: Vec<u8>,
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
                riff_size = (data.len() >> 24) as u8;
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
pub enum ChunkValue {
    #[br(pre_assert(chunk_type.chunk_type() == 0))]
    ChRiff {
        #[br(temp, map = |x: [u8;3]| ((x[0] as u32) << 16) | ((x[1] as u32) << 8) | (x[2] as u32) | (((chunk_type.riff_size()) as u32) << 24))]
        #[bw(calc = data.len().try_into().unwrap(), map = |x: u32| { let bytes = x.to_be_bytes(); [bytes[1], bytes[2], bytes[3]] })]
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
        // This could be optimized by counting rather than doing another clone and into
        #[bw(calc = Gamma { value: (Into::<StructHeader>::into(header.clone()).byte_len() + 1).try_into().unwrap() })]
        header_size: Gamma,
        #[br(map = |header: StructHeader| header.into())]
        #[bw(map = |props: &Vec<TableHeaderProperty>| -> StructHeader { props.clone().into() })]
        header: Vec<TableHeaderProperty>,
        #[br(parse_with = until_magic_with(0u8, |r,e,_: ()| ChTableElement::read_options(r, e, (&header,)), |reader, endian, arg| {
            u8::read_options(reader, endian, arg)
        }))]
        elements: Vec<ChTableElement>,
        #[br(temp, ignore)]
        #[bw(calc = 0)]
        terminator: u8,
    },
    #[br(pre_assert(chunk_type.chunk_type() == 4))]
    ChSparseTable {
        #[bw(calc = Gamma { value: (Into::<StructHeader>::into(header.clone()).byte_len() + 1).try_into().unwrap() })]
        header_size: Gamma,
        #[br(map = |header: StructHeader| header.into())]
        #[bw(map = |props: &Vec<TableHeaderProperty>| -> StructHeader { props.clone().into() })]
        header: Vec<TableHeaderProperty>,
        #[br(parse_with = until_magic_with(0u8, |r,e,_: ()| ChSparseTableElement::read_options(r, e, (&header,)), |reader, endian, arg| {
            u8::read_options(reader, endian, arg)
        }))]
        elements: Vec<ChSparseTableElement>,
        #[br(temp, ignore)]
        #[bw(calc = 0)]
        terminator: u8,
    },
}

#[cfg(test)]
mod tests {
    use binrw::{BinReaderExt, BinWrite};
    use std::{
        fs::File,
        io::{Cursor, Result},
    };

    use crate::save::{Chunks, OuterSave};

    #[test]
    fn parse_and_write_tiny() -> Result<()> {
        let mut f = File::open("tests/tiny.sav")?;

        let outer: OuterSave = f.read_ne().unwrap();
        let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

        let mut d = vec![];
        let mut writer = Cursor::new(&mut d);
        Chunks::write(&chunk, &mut writer).unwrap();
        assert_eq!(&outer.data, &d);

        Ok(())
    }

    #[test]
    fn parse_and_write_new_tiny() -> Result<()> {
        let mut f = File::open("tests/TinyVanillaTest.sav")?;

        let outer: OuterSave = f.read_ne().unwrap();
        let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

        let mut d = vec![];
        let mut writer = Cursor::new(&mut d);
        Chunks::write_be(&chunk, &mut writer).unwrap();
        assert_eq!(&outer.data, &d);

        // Useful for testing something wrong
        // let mut f = File::create("AWrite.sav")?;
        // f.write_all(&outer.data)?;
        // let mut f = File::create("BWrite.sav")?;
        // f.write_all(&d)?;

        Ok(())
    }
}
