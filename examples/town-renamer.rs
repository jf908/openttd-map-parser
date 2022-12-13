// This only works on JGR patch pack save files for now

use std::{
    fs::File,
    io::{Cursor, Result, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use ottd_map_parser::{
    charray, jgr,
    save::{ChArrayElement, Chunk, ChunkValue, Save},
};

use binrw::{args, BinReaderExt, BinWrite};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(version, about = "A tool for renaming towns in OpenTTD savefiles.")]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Read {
        #[arg(value_name = "SAVEFILE")]
        save: PathBuf,
        #[arg(short, long, default_value_t = String::from("towns.json"))]
        output: String,
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },
    Write {
        #[arg(value_name = "SAVEFILE")]
        save: PathBuf,
        #[arg(value_name = "JSONFILE")]
        json: PathBuf,
        #[arg(short, long, default_value_t = String::from("out.sav"))]
        output: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Town {
    x: u32,
    y: u32,
    name: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.action {
        Action::Read {
            save,
            output,
            pretty,
        } => {
            let mut f = File::open(save)?;

            let save: Save = f.read_ne().unwrap();

            let slxi = save
                .chunks
                .iter()
                .find(|x| &x.tag == b"SLXI")
                .map(|slxi_chunk| match &slxi_chunk.value {
                    ChunkValue::ChRiff { data } => {
                        let c = &mut Cursor::new(&data);
                        let slxi: jgr::SLXI = c.read_ne().unwrap();
                        slxi
                    }
                    _ => {
                        panic!("SLXI wasn't a CH_RIFF")
                    }
                })
                .unwrap_or_default();

            let maps = save.chunks.iter().find(|x| &x.tag == b"MAPS").unwrap();
            let map_info: charray::Maps = match &maps.value {
                ChunkValue::ChRiff { data } => Cursor::new(data).read_ne().unwrap(),
                ChunkValue::ChTable { elements, .. } => {
                    Cursor::new(&elements[0].data).read_ne().unwrap()
                }
                _ => {
                    panic!("Something wrong with MAPS")
                }
            };

            let city_chunk = save.chunks.iter().find(|x| &x.tag == b"CITY").unwrap();

            let cities: Vec<charray::City> = match &city_chunk.value {
                ChunkValue::ChArray { elements } => elements
                    .iter()
                    .map(|el| {
                        Cursor::new(&el.data)
                            .read_ne_args::<charray::City>(args! { slxi: &slxi })
                            .unwrap()
                    })
                    .collect(),
                _ => {
                    panic!("Currently only supports old/JGR maps")
                }
            };

            let towns: Vec<Town> = cities
                .iter()
                .map(|c| Town {
                    x: c.xy % map_info.dim_x,
                    y: c.xy / map_info.dim_x,
                    name: c.name.to_string(),
                })
                .collect();

            let mut new_file = File::create(output)?;
            let json = if pretty {
                serde_json::to_string_pretty(&towns)
            } else {
                serde_json::to_string_pretty(&towns)
            }?;
            new_file.write_all(json.as_bytes())?;
        }
        Action::Write { save, json, output } => {
            let mut f = File::open(save)?;

            let json_file = File::open(json)?;
            let towns: Vec<Town> = serde_json::from_reader(json_file)?;

            let mut save: Save = f.read_ne().unwrap();

            let slxi = save
                .chunks
                .iter()
                .find(|x| &x.tag == b"SLXI")
                .map(|slxi_chunk| match &slxi_chunk.value {
                    ChunkValue::ChRiff { data } => {
                        let c = &mut Cursor::new(&data);
                        let slxi: jgr::SLXI = c.read_ne().unwrap();
                        slxi
                    }
                    _ => {
                        panic!("SLXI wasn't a CH_RIFF")
                    }
                })
                .unwrap_or_default();

            save.chunks = save
                .chunks
                .into_iter()
                .map(|x| {
                    if &x.tag == b"CITY" {
                        Chunk {
                            value: ChunkValue::ChArray {
                                elements: match x.value {
                                    ChunkValue::ChArray { elements } => {
                                        std::iter::zip(elements, &towns)
                                            .map(|(e, t)| {
                                                let mut city = Cursor::new(e.data)
                                                    .read_ne_args::<charray::City>(
                                                        args! { slxi: &slxi },
                                                    )
                                                    .unwrap();

                                                city.name = t.name.to_string();

                                                let mut new_data = Vec::new();
                                                city.write_args(
                                                    &mut Cursor::new(&mut new_data),
                                                    args! { slxi: &slxi },
                                                )
                                                .unwrap();
                                                ChArrayElement { data: new_data }
                                            })
                                            .collect()
                                    }
                                    _ => {
                                        panic!("Currently only supports old/JGR maps")
                                    }
                                },
                            },
                            ..x
                        }
                    } else {
                        x
                    }
                })
                .collect();

            let mut out_file = File::create(output)?;
            save.write(&mut out_file).unwrap();
        }
    }

    Ok(())
}
