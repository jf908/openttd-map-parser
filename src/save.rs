use crate::chtable::{ChSparseTableElement, ChTableElement, StructHeader, TableHeaderProperty};
use crate::gamma::{gamma_length, parse_gamma, write_gamma, Gamma};
use crate::helpers::{until_magic, until_magic_with};
#[cfg(feature = "lzma-rs")]
use binrw::io::BufReader;
#[cfg(feature = "xz2")]
use binrw::io::{Read, Write};
use binrw::{
    binrw,
    helpers::until_eof,
    io::{Cursor, Error, ErrorKind, Result},
    BinRead, BinResult, BinWrite,
};
#[cfg(feature = "lzma-rs")]
use lzma_rs::{xz_compress, xz_decompress};
use modular_bitfield::{bitfield, specifiers::B4};
use serde::{Deserialize, Serialize};
#[cfg(feature = "xz2")]
use xz2::{read::XzDecoder, write::XzEncoder};

#[binrw]
#[derive(Debug, Deserialize, Serialize)]
pub enum CompressionType {
    /// Compressed with LZO (deprecated, only really old savegames would use this).
    #[brw(magic = b"OTTD")]
    OTTD,
    /// No compression.
    #[brw(magic = b"OTTN")]
    OTTN,
    /// Compressed with zlib.
    #[brw(magic = b"OTTZ")]
    OTTZ,
    /// Compressed with LZMA.
    #[brw(magic = b"OTTX")]
    OTTX,
    /// Compressed with zstd (from JGR, not in vanilla OpenTTD)
    #[brw(magic = b"OTTS")]
    OTTS,
}

/// Performs save file compression on a byte slice of the compressed data
fn compress_save(compression_type: &CompressionType, blob: &Vec<u8>) -> Result<Vec<u8>> {
    match compression_type {
        CompressionType::OTTD => Err(Error::new(ErrorKind::Other, "Old save file not supported")),
        CompressionType::OTTN => Ok(blob.clone()),
        CompressionType::OTTZ => Err(Error::new(ErrorKind::Other, "zlib not supported (yet)")),
        CompressionType::OTTX => {
            cfg_if::cfg_if! {
                if #[cfg(feature = "xz2")] {
                    let mut buffer: Vec<u8> = Vec::new();
                    {
                        let mut encoder = XzEncoder::new(&mut buffer, 2);
                        encoder.write(&blob).unwrap();
                    }
                    Ok(buffer)
                } else if #[cfg(feature = "lzma-rs")] {
                    let mut buffer: Vec<u8> = Vec::new();
                    {
                        xz_compress(&mut Cursor::new(blob), &mut buffer)?;
                    }
                    Ok(buffer)
                } else {
                    Err(Error::new(ErrorKind::Other, "Not compiled with LZMA support"))
                }
            }
        }
        CompressionType::OTTS => {
            #[cfg(not(feature = "zstd"))]
            return Err(Error::new(
                ErrorKind::Other,
                "Not compiled with zstd support",
            ));
            #[cfg(feature = "zstd")]
            zstd::stream::encode_all(&mut Cursor::new(blob), 0)
        }
    }
}

/// Performs save file decompression on a byte slice of the decompressed data
fn decompress_save(compression_type: &CompressionType, blob: Vec<u8>) -> Result<Vec<u8>> {
    match compression_type {
        CompressionType::OTTD => Err(Error::new(ErrorKind::Other, "Old save file not supported")),
        CompressionType::OTTN => Ok(blob),
        CompressionType::OTTZ => Err(Error::new(ErrorKind::Other, "zlib not supported (yet)")),
        CompressionType::OTTX => {
            cfg_if::cfg_if! {
                if #[cfg(feature = "xz2")] {
                    let mut buffer = Vec::new();
                    XzDecoder::new(Cursor::new(&blob)).read_to_end(&mut buffer)?;
                    Ok(buffer)
                } else if #[cfg(feature = "lzma-rs")] {
                    let mut buffer = Vec::new();
                    xz_decompress(&mut Cursor::new(blob), &mut buffer).map_err(lzma_error_to_io)?;
                    Ok(buffer)
                } else {
                    Err(Error::new(ErrorKind::Other, "Not compiled with LZMA support"))
                }
            }
        }
        CompressionType::OTTS => {
            #[cfg(not(feature = "zstd"))]
            return Err(Error::new(
                ErrorKind::Other,
                "Not compiled with zstd support",
            ));
            #[cfg(feature = "zstd")]
            zstd::stream::decode_all(&mut Cursor::new(blob))
        }
    }
}

/// A parser that performs save file decompression on data taken from a reader
#[binrw::parser(reader, endian)]
fn chunk_reader(compression_type: &CompressionType) -> BinResult<Vec<Chunk>> {
    match compression_type {
        CompressionType::OTTD => Err(binrw::Error::Io(Error::new(ErrorKind::Other, "Old save file not supported"))),
        CompressionType::OTTN => Chunks::read_options(reader, endian, ()),
        CompressionType::OTTZ => Err(binrw::Error::Io(Error::new(ErrorKind::Other, "zlib not supported (yet)"))),
        CompressionType::OTTX => {
            cfg_if::cfg_if! {
                if #[cfg(feature = "xz2")] {
                    let mut buffer = Vec::new();
                    let mut new_reader = XzDecoder::new(reader);
                    new_reader.read_to_end(&mut buffer)?;
                    Chunks::read_options(&mut Cursor::new(&mut buffer), endian, ())
                } else if #[cfg(feature = "lzma-rs")] {
                    let mut buffer = Vec::new();
                    let mut buf_reader = BufReader::new(reader);
                    xz_decompress(&mut buf_reader, &mut buffer).map_err(lzma_error_to_io)?;
                    Chunks::read_options(&mut Cursor::new(&mut buffer), endian, ())
                } else {
                    return Err(binrw::Error::Io(Error::new(ErrorKind::Other, "Not compiled with LZMA support")));
                }
            }
        }

        CompressionType::OTTS => {
            #[cfg(not(feature = "zstd"))]
            return Err(binrw::Error::Io(Error::new(ErrorKind::Other, "Not compiled with zstd support")));
            #[cfg(feature = "zstd")]
            Chunks::read_options(&mut Cursor::new(zstd::stream::decode_all(reader)?), endian, ())
        }
    }
    .map(|c| c.chunks)
}

/// A writer that performs save file compression on parsed data.
/// Includes the 0u32 terminator before compressing.
#[binrw::writer(writer, endian)]
fn chunk_writer(chunks: &Vec<Chunk>, compression_type: &CompressionType) -> BinResult<()> {
    match compression_type {
        CompressionType::OTTD => Err(binrw::Error::Io(Error::new(
            ErrorKind::Other,
            "Old save file not supported",
        ))),
        CompressionType::OTTN => {
            chunks.write_options(writer, endian, ())?;
            // Terminator
            0u32.write_options(writer, endian, ())
        }
        CompressionType::OTTZ => Err(binrw::Error::Io(Error::new(
            ErrorKind::Other,
            "zlib not supported (yet)",
        ))),
        CompressionType::OTTX => {
            let mut buffer: Vec<u8> = Vec::new();

            {
                let mut writer = Cursor::new(&mut buffer);
                chunks.write_options(&mut writer, endian, ())?;
                // Terminator
                0u32.write_options(&mut writer, endian, ())?;
            }

            cfg_if::cfg_if! {
                if #[cfg(feature = "xz2")] {
                    let mut encoder = XzEncoder::new(writer, 2);
                    encoder.write_all(&mut buffer)?;
                    Ok(())
                } else if #[cfg(feature = "lzma-rs")] {
                    xz_compress(&mut Cursor::new(buffer), writer)?;
                    Ok(())
                } else {
                    Err(binrw::Error::Io(Error::new(ErrorKind::Other, "Not compiled with LZMA support")))
                }
            }
        }
        CompressionType::OTTS => {
            cfg_if::cfg_if! {
                if #[cfg(feature = "zstd")] {
                    let mut buffer: Vec<u8> = Vec::new();

                    {
                        let mut writer = Cursor::new(&mut buffer);
                        chunks.write_options(&mut writer, endian, ())?;
                        // Terminator
                        0u32.write_options(&mut writer, endian, ())?;
                    }

                    zstd::stream::copy_encode(&mut Cursor::new(buffer), writer, 0)?;

                    Ok(())
                } else {
                    Err(binrw::Error::Io(Error::new(ErrorKind::Other, "Not compiled with zstd support")))
                }
            }
        }
    }
}

#[binrw]
#[brw(big)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Save {
    pub compression_type: CompressionType,
    // The next two bytes indicate which savegame version used.
    pub version: u16,
    // The next two bytes can be ignored, and were only used in really old savegames.
    _ignore: u16,
    // Wish I could use map_stream here from the new PR but no rust LZMA decompressers support Read + Seek :(
    #[br(parse_with = |r,e,_: ()| chunk_reader(r, e, (&compression_type,)))]
    #[bw(write_with = |r,e,d,_: ()| chunk_writer(r, e, d, (&compression_type,)))]
    #[serde(with = "chunk")]
    pub chunks: Vec<Chunk>,
}

impl Save {
    pub fn get(&self, tag: &[u8; 4]) -> Option<&ChunkValue> {
        self.chunks
            .iter()
            .find(|x| &x.tag == tag)
            .map(|chunk| &chunk.value)
    }
}

mod chunk {

    use super::{Chunk, ChunkValue};
    use serde::{
        de::{MapAccess, Visitor},
        ser::SerializeMap,
        Deserializer, Serializer,
    };

    pub fn serialize<S>(chunks: &Vec<Chunk>, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(chunks.len()))?;
        for chunk in chunks {
            map.serialize_entry(&String::from_utf8_lossy(&chunk.tag), &chunk.value)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> core::result::Result<Vec<Chunk>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MapVisitor;

        impl<'de> Visitor<'de> for MapVisitor {
            type Value = Vec<Chunk>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a chunk value")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = Vec::with_capacity(access.size_hint().unwrap_or(0));

                while let Some((key, value)) = access.next_entry::<String, ChunkValue>()? {
                    let tag = key
                        .as_bytes()
                        .try_into()
                        .map_err(|_| serde::de::Error::custom("Map tag must be 4 chars"))?;
                    values.push(Chunk { tag, value });
                }

                Ok(values)
            }
        }

        deserializer.deserialize_map(MapVisitor)
    }
}

#[binrw]
#[brw(big)]
#[derive(Debug, Deserialize, Serialize)]
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
    _terminator: u32,
}

#[binrw]
#[brw(big)]
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize, Serialize)]
pub struct ChArrayElement {
    // Actual length = size - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (data.len() + 1).try_into().unwrap() })]
    size: Gamma,
    #[br(count = size.value.saturating_sub(1))]
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[binrw]
#[brw(big)]
#[derive(Debug, Deserialize, Serialize)]
pub struct ChSparseArrayElement {
    // Actual length = length - 1
    #[br(temp)]
    #[bw(calc = Gamma { value: (TryInto::<u32>::try_into(data.len() + 1).unwrap()) + gamma_length(*index) })]
    size: Gamma,
    #[br(parse_with = parse_gamma)]
    #[bw(write_with = write_gamma)]
    pub index: u32,
    #[br(count = size.value.saturating_sub(1 + gamma_length(index)))]
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[bitfield]
#[binrw]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ChunkValue {
    #[br(pre_assert(chunk_type.chunk_type() == 0))]
    ChRiff {
        #[br(temp, map = |x: [u8;3]| ((x[0] as u32) << 16) | ((x[1] as u32) << 8) | (x[2] as u32) | (((chunk_type.riff_size()) as u32) << 24))]
        #[bw(calc = data.len().try_into().unwrap(), map = |x: u32| { let bytes = x.to_be_bytes(); [bytes[1], bytes[2], bytes[3]] })]
        size: u32,
        #[br(count = size)]
        #[serde(with = "serde_bytes")]
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
        _header_size: Gamma,
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
        _header_size: Gamma,
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

#[cfg(feature = "lzma-rs")]
fn lzma_error_to_io(error: lzma_rs::error::Error) -> Error {
    match error {
        lzma_rs::error::Error::IoError(e) => e,
        lzma_rs::error::Error::HeaderTooShort(e) => e,
        lzma_rs::error::Error::LzmaError(str) => Error::new(ErrorKind::Other, str),
        lzma_rs::error::Error::XzError(str) => Error::new(ErrorKind::Other, str),
    }
}

#[cfg(test)]
mod tests {
    use binrw::{BinReaderExt, BinWrite};
    use std::{
        fs::File,
        io::{Cursor, Result},
    };

    use crate::save::{Chunks, OuterSave, Save};

    #[test]
    fn parse_and_write_outer_tiny() -> Result<()> {
        let mut f = File::open("./BIG.sav")?;

        let outer: OuterSave = f.read_ne().unwrap();
        let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

        let mut d = vec![];
        let mut writer = Cursor::new(&mut d);
        Chunks::write(&chunk, &mut writer).unwrap();
        assert_eq!(&outer.data, &d);

        Ok(())
    }

    #[test]
    fn parse_and_write_outer_new_tiny() -> Result<()> {
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

    #[test]
    fn serialize() -> Result<()> {
        let mut f = File::open("tests/TinyVanillaTest.sav")?;
        let save: Save = f.read_ne().unwrap();

        let json = serde_json::to_string(&save)?;

        let value: Save = serde_json::from_str(&json)?;

        // Not an exact test but at least we know there are no errors
        assert_eq!(save.chunks.len(), value.chunks.len());

        Ok(())
    }
}
