use binrw::{
    binrw,
    io::{Read, Seek},
    BinRead, BinResult,
};

// Read a gamma value which can be 1-5 bytes of data to represent a u32
//   0xxxxxxx
//   10xxxxxx xxxxxxxx
//   110xxxxx xxxxxxxx xxxxxxxx
//   1110xxxx xxxxxxxx xxxxxxxx xxxxxxxx
//   11110--- xxxxxxxx xxxxxxxx xxxxxxxx xxxxxxxx
pub fn parse_gamma<R: Read + Seek>(reader: &mut R, ro: binrw::Endian, _: ()) -> BinResult<u32> {
    let first_byte: u8 = u8::read_options(reader, ro, ())?;

    match first_byte {
        x if x & 0b10000000 == 0 => Ok(x.into()),
        x if x & 0b01000000 == 0 => {
            let next_byte: u16 = u8::read_options(reader, ro, ())?.into();
            Ok((((x as u16) & !(0b10000000)) << 8 | next_byte).into())
        }
        x if x & 0b00100000 == 0 => {
            let next_bytes: u32 = u16::read_options(reader, ro, ())?.into();
            Ok(((x as u32) & !(0b11000000)) << 16 | next_bytes)
        }
        x if x & 0b00010000 == 0 => {
            let next_byte: u32 = u8::read_options(reader, ro, ())?.into();
            let next_bytes: u32 = u16::read_options(reader, ro, ())?.into();
            Ok(((x as u32) & !(0b11100000)) << 24 | (next_byte << 16) | next_bytes)
        }
        _ => Ok(u32::read_options(reader, ro, ())?),
    }
}

#[binrw::writer(writer)]
pub fn write_gamma(i: &u32) -> BinResult<()> {
    if i >= &(1 << 7) {
        if i >= &(1 << 14) {
            if i >= &(1 << 21) {
                if i >= &(1 << 28) {
                    writer.write(&[0xF0, (i >> 24) as u8])?;
                } else {
                    writer.write(&[0xE0 | ((i >> 24) as u8)])?;
                }
                writer.write(&[(i >> 16) as u8])?;
            } else {
                writer.write(&[(0xC0 | ((i >> 16) as u8))])?;
            }
            writer.write(&[((i >> 8) as u8)])?;
        } else {
            writer.write(&[(0x80 | ((i >> 8) as u8))])?;
        }
    }
    writer.write(&[*i as u8])?;

    Ok(())
}

pub fn gamma_length(i: u32) -> u32 {
    1 + (if i >= (1 << 7) { 1 } else { 0 })
        + (if i >= (1 << 14) { 1 } else { 0 })
        + (if i >= (1 << 21) { 1 } else { 0 })
        + (if i >= (1 << 28) { 1 } else { 0 })
}

#[binrw]
#[derive(Debug)]
pub struct Gamma {
    #[br(parse_with = parse_gamma)]
    #[bw(write_with = write_gamma)]
    pub value: u32,
}

#[cfg(test)]
mod tests {
    use binrw::Endian;
    use std::io::Cursor;

    use crate::gamma::parse_gamma;

    use super::{gamma_length, write_gamma};

    #[test]
    fn parse_gamma_test() {
        let read_options = Endian::Big;

        let mut reader = Cursor::new(b"\x09");
        let gamma = parse_gamma(&mut reader, read_options, ()).unwrap();
        assert_eq!(gamma, 9);

        let mut reader = Cursor::new(0b10000001_00001000u16.to_be_bytes());
        let gamma = parse_gamma(&mut reader, read_options, ()).unwrap();
        assert_eq!(gamma, 264);

        let mut reader = Cursor::new(
            0b11111000_00001000_00000000_00000000_00000000_00000000_00000000_00000000u64
                .to_be_bytes(),
        );
        let gamma = parse_gamma(&mut reader, read_options, ()).unwrap();
        assert_eq!(gamma, 134217728);
    }

    #[test]
    fn write_gamma_test() {
        {
            let mut data = vec![];
            let mut writer = Cursor::new(&mut data);
            write_gamma(&3, &mut writer, Endian::Big, ()).unwrap();
            assert_eq!(data, &[3]);
        }

        {
            let mut data = vec![];
            let mut writer = Cursor::new(&mut data);
            write_gamma(&179192378, &mut writer, Endian::Big, ()).unwrap();

            writer.set_position(0);
            let gamma = parse_gamma(&mut writer, Endian::Big, ()).unwrap();

            assert_eq!(gamma, 179192378);
        }
    }

    #[test]
    fn gamma_length_test() {
        assert_eq!(gamma_length(150u32), 2);
    }
}
