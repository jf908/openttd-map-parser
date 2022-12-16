use std::{
    fs::File,
    io::{Cursor, Result},
};

use ottd_map_parser::{
    charray,
    chtable::TableData,
    save::{ChunkValue, Save},
};

use binrw::BinReaderExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Town {
    x: u32,
    y: u32,
    name: String,
}

fn main() -> Result<()> {
    let mut f = File::open("./tests/TinyVanillaTest.sav")?;

    let save: Save = f.read_ne().unwrap();

    println!(
        "Tags: {}",
        save.chunks
            .iter()
            .map(|x| String::from_utf8_lossy(&x.tag).to_string())
            .collect::<Vec<String>>()
            .join(",")
    );

    let maps = save.chunks.iter().find(|x| &x.tag == b"MAPS").unwrap();
    let map_info: charray::Maps = match &maps.value {
        ChunkValue::ChRiff { data } => Cursor::new(data).read_ne().unwrap(),
        ChunkValue::ChTable { elements, .. } => {
            let dim_x = elements[0]
                .data
                .iter()
                .find(|(k, _)| k == "dim_x")
                .and_then(|(_, v)| match &v {
                    TableData::UInt32(x) => Some(*x),
                    _ => None,
                })
                .expect("Something wrong with MAPS");

            let dim_y = elements[0]
                .data
                .iter()
                .find(|(k, _)| k == "dim_y")
                .and_then(|(_, v)| match &v {
                    TableData::UInt32(x) => Some(*x),
                    _ => None,
                })
                .expect("Something wrong with MAPS");

            charray::Maps { dim_x, dim_y }
        }
        // ChunkValue::ChTable { elements, .. } => Cursor::new(&elements[0].data).read_ne().unwrap(),
        _ => {
            panic!("Something wrong with MAPS")
        }
    };

    println!("Map Size: {}x{}", map_info.dim_x, map_info.dim_y);

    Ok(())
}
