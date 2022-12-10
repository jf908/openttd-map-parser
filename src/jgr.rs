use binrw::binrw;
use modular_bitfield::{
    bitfield,
    specifiers::{B24, B4},
};

use crate::gamma::Gamma;

#[binrw]
#[brw(big)]
#[derive(Debug, Default)]
pub struct SLXI {
    chunk_version: u32,
    flags: u32,
    #[br(temp)]
    #[bw(calc = chunks.len() as u32)]
    item_count: u32,
    #[br(count = item_count)]
    chunks: Vec<ExtendedChunk>,
}

impl SLXI {
    pub fn has_feature(&self, name: &str) -> bool {
        self.chunks.iter().any(|sub_chunk| sub_chunk.name == name)
    }
}

#[bitfield(bits = 32)]
#[binrw]
#[derive(Debug, Clone, Copy)]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
struct SlxiSubChunkFlags {
    #[skip]
    __: B24,
    ignorable_unknown: bool,
    ignorable_version: bool,
    extra_data_present: bool,
    chunk_id_list_present: bool,
    #[skip]
    __: B4,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct ExtendedChunk {
    flags: SlxiSubChunkFlags,
    version: u16,
    #[br(temp)]
    #[bw(calc = Gamma { value: name.len().try_into().unwrap() })]
    name_size: Gamma,
    #[br(count = name_size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    name: String,
    #[brw(if(flags.extra_data_present()))]
    #[br(temp)]
    #[bw(calc = extra_data.as_ref().map_or(0, |x| x.len()) as u32)]
    extra_data_length: u32,
    #[brw(if(flags.extra_data_present()))]
    #[br(count = extra_data_length)]
    extra_data: Option<Vec<u8>>,
    #[brw(if(flags.chunk_id_list_present()))]
    #[br(temp)]
    #[bw(calc = chunk_list.as_ref().map_or(0, |x| x.len()) as u32)]
    chunk_count: u32,
    #[brw(if(flags.chunk_id_list_present()))]
    #[br(count = chunk_count)]
    chunk_list: Option<Vec<u32>>,
}
