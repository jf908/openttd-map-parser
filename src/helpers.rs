use binrw::{
    io::{Read, Seek, SeekFrom},
    BinRead, BinResult, Endian,
};

#[binrw::parser(reader, endian)]
fn default_reader<'a, T: BinRead>(args: T::Args<'a>, ...) -> BinResult<T>
where
    T::Args<'a>: Clone,
{
    let mut value = T::read_options(reader, endian, args.clone())?;
    value.after_parse(reader, endian, args)?;
    Ok(value)
}

pub fn until_magic<Reader, T, B, Arg, Ret>(
    magic: B,
) -> impl Fn(&mut Reader, Endian, Arg) -> BinResult<Ret>
where
    T: for<'a> BinRead<Args<'a> = Arg>,
    B: for<'a> BinRead<Args<'a> = ()>
        + core::fmt::Debug
        + PartialEq
        + Sync
        + Send
        + Clone
        + Copy
        + 'static,
    Reader: Read + Seek,
    Arg: Clone,
    Ret: FromIterator<T>,
{
    until_magic_with(magic, default_reader, |reader, endian, arg| {
        B::read_options(reader, endian, arg)
    })
}

pub fn until_magic_with<Reader, T, B, Arg, ReadFn, ReadFn2, Ret>(
    magic: B,
    read: ReadFn,
    read2: ReadFn2,
) -> impl Fn(&mut Reader, Endian, Arg) -> BinResult<Ret>
where
    Reader: Read + Seek,
    B: for<'a> BinRead<Args<'a> = ()>
        + core::fmt::Debug
        + PartialEq
        + Sync
        + Send
        + Clone
        + Copy
        + 'static,
    Arg: Clone,
    ReadFn2: Fn(&mut Reader, Endian, ()) -> BinResult<B>,
    ReadFn: Fn(&mut Reader, Endian, Arg) -> BinResult<T>,
    Ret: FromIterator<T>,
{
    move |reader, endian, args| {
        core::iter::from_fn(|| {
            let stored_position = match reader.stream_position() {
                Ok(val) => val,
                Err(err) => return Some(Err(err.into())),
            };

            match read2(reader, endian, ()) {
                Ok(value) => {
                    if magic == value {
                        return None;
                    }
                }
                Err(err) => {
                    return Some(Err(err.into()));
                }
            }

            if let Err(err) = reader.seek(SeekFrom::Start(stored_position)) {
                return Some(Err(err.into()));
            }

            match read(reader, endian, args.clone()) {
                Ok(value) => Some(Ok(value)),
                err => Some(err),
            }
        })
        .fuse()
        .collect()
    }
}

pub fn new_args_iter_with<Reader, T, Arg, Ret, It, ReadFn>(
    it: It,
    read: ReadFn,
) -> impl FnOnce(&mut Reader, Endian, ()) -> BinResult<Ret>
where
    Reader: Read + Seek,
    Arg: Clone,
    Ret: FromIterator<T>,
    It: IntoIterator<Item = Arg>,
    ReadFn: Fn(&mut Reader, Endian, Arg) -> BinResult<T>,
{
    move |reader, options, _| {
        it.into_iter()
            .map(|arg| read(reader, options, arg))
            .collect()
    }
}
