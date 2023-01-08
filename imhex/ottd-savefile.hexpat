#include <std/mem.pat>

#pragma endian big
#pragma pattern_limit 9999999

char compression_type[0x04] @ 0x00;
u16 version @ 0x04;
u16 ignore @ 0x06;

bitfield ChunkType {
    type: 4;
    rest_size: 4;
};

struct Gamma {
    u8 first_byte;

    if (first_byte & 0b11110000 == 0b11110000) {
        u32 value;
    } else if(first_byte & 0b11100000 == 0b11100000) {
        u24 value;
    } else if(first_byte & 0b11000000 == 0b11000000) {
        u16 value;
    } else if(first_byte & 0b10000000 == 0b10000000) {
        u8 value;
    }
} [[single_color]];

fn gamma_value(Gamma value) {
    if ((value.first_byte & 0b11110000) == 0b11110000) {
        return value.value;
    } else if((value.first_byte & 0b11100000) == 0b11100000) {
        return value.value + ((0b00011111 & value.first_byte) << 24);
    } else if((value.first_byte & 0b11000000) == 0b11000000) {
        return value.value + ((0b00111111 & value.first_byte) << 16);
    } else if((value.first_byte & 0b10000000) == 0b10000000) {
        return value.value + ((0b01111111 & value.first_byte) << 8);
    }
    
    return value.first_byte;
};

fn gamma_size(Gamma value) {
    if (value.first_byte & 0b11110000 == 0b11110000) {
        return 5;
    } else if(value.first_byte & 0b11100000 == 0b11100000) {
        return 4;
    } else if(value.first_byte & 0b11000000 == 0b11000000) {
        return 3;
    } else if(value.first_byte & 0b10000000 == 0b10000000) {
        return 2;
    }
    
    return 1;
};

enum SleType: u8 {
    Null = 0,
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
};

struct TableHeaderProperty {
    SleType type;
    
    if (type == 0) {
        break;
    } else {
        Gamma length;
        char key[length.first_byte];
    }
};

struct CH_TABLE {
    Gamma header_size;
    u8 data[gamma_value(header_size) - 1];
};

struct CH_ARRAY {
    Gamma size;
    if(size.first_byte > 0) {
        u8 data[gamma_value(size) - 1];
    }
};

struct CH_SPARSE_ARRAY {
    Gamma size;
    if(size.first_byte != 0) {
        Gamma index;
        u8 data[gamma_value(size) - 1 - gamma_size(index)];
    } else {
        break;
    }
};

struct Chunk {
    char tag[0x04];
    ChunkType type;
  
    if (type.type == 0x3) {
        CH_TABLE data;
        CH_ARRAY array[while(std::mem::read_unsigned($, 1) != 0x00)];
        u8 terminator;
    } else if (type.type == 0x0) {
        u24 length;
        u8 data[length + (type.rest_size << 24)];
    } else if (type.type == 0x1) {
        CH_ARRAY array[while(std::mem::read_unsigned($, 1) != 0x00)];
        u8 terminator;
    } else if (type.type == 0x2) {
        CH_SPARSE_ARRAY array[while(std::mem::read_unsigned($, 1) != 0x00)];
        u8 terminator;
    } else if (type.type == 0x4) {
        CH_TABLE data;
        CH_SPARSE_ARRAY array[while(std::mem::read_unsigned($, 1) != 0x00)];
        u8 terminator;
    }
};

Chunk chunk[while(std::mem::read_unsigned($, 4) != 0x00)] @ 0x08;
u32 file_terminator @ $;

