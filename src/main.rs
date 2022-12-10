use std::{
    fs::File,
    io::{Cursor, Read, Result, Write},
};

use openttd_map_parser::{
    charray, jgr,
    save::{ChunkValue, Save},
};

use binrw::{args, binrw, until_eof, BinRead, BinReaderExt, BinWrite, NamedArgs};

fn main() -> Result<()> {
    let mut f = File::open("TinyJGRTest1.sav")?;

    let save: Save = f.read_ne().unwrap();

    // let mut out_file = File::create("TutorialOut.sav")?;
    // outer.write(&mut out_file).unwrap();

    // println!("Size:{}", outer.data.len());

    println!(
        "{:?}",
        save.chunks
            .iter()
            .map(|x| String::from_utf8_lossy(&x.tag).to_string())
            .collect::<Vec<String>>()
            .join(",")
    );

    let array = save
        .chunks
        .iter()
        .find(|x| &x.tag == b"SLXI")
        .expect("Couldn't find SLXI, not JGR map?");

    let slxi = match &array.value {
        ChunkValue::ChRiff { data, .. } => {
            let mut c = &mut Cursor::new(&data);
            let slxi: jgr::SLXI = c.read_ne().unwrap();
            slxi
        }
        _ => {
            panic!("SLXI wasn't a CH_RIFF")
        }
    };

    let array = save
        .chunks
        .iter()
        .find(|x| String::from_utf8_lossy(&x.tag) == "PLYR")
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

            println!("{:?}", header);
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
