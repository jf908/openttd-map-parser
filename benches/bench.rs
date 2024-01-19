use binrw::{BinReaderExt, BinWrite};
use criterion::{criterion_group, criterion_main, Criterion};
use ottd_map_parser::save::{Chunks, OuterSave};
use std::{
    fs::File,
    io::{Cursor, Result},
};

fn parse_file() -> Result<()> {
    let mut f = File::open("./BIG.sav")?;

    let outer: OuterSave = f.read_ne().unwrap();
    let chunk: Chunks = Cursor::new(&outer.data).read_ne().unwrap();

    let mut d = vec![];
    let mut writer = Cursor::new(&mut d);
    Chunks::write(&chunk, &mut writer).unwrap();

    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse-big-file");
    group.sample_size(10);
    group.bench_function("parse-file", |b| b.iter(|| parse_file()));
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
