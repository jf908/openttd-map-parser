# OpenTTD Map Parser (and Writer)

This library is a map parser and writer for OpenTTD written in Rust with ✨[binrw](https://binrw.rs/)✨.

## Examples

### Town Renamer

You can use the town renamer CLI to import/export a list of town names specifically for JGR savefiles.

#### Save to towns.json

```sh
cargo run --release --example town-renamer read ./game.sav
```

#### towns.json to Save

```sh
cargo run --release --example town-renamer write ./game.sav ./towns.json -o new_save.sav
```

## ImHex

In `imhex/ottd-savefile.hexpat`, you'll find a pattern that you can load into ImHex to visualize an OpenTTD savefile in hex. This will not work if the save file is compressed so you will have to disable compression in OTTD or decompress it with this library.

## Wasm

With wasm pack (installed with `cargo install wasm-pack`)

```sh
wasm-pack build --release --target web -- --no-default-features --features lzma-rs
```

## Useful links

- [OpenTTD's Savegame Format](https://github.com/OpenTTD/OpenTTD/blob/master/docs/savegame_format.md)
- [OpenTTD's Savegame Compression](https://wiki.openttd.org/en/Archive/Manual/Settings/Savegame%20format)
- [OpenTTD Source](https://github.com/OpenTTD/OpenTTD)
- [JGR Source](https://github.com/JGRennison/OpenTTD-patches)

## Licenses

This software is licensed under GNU General Public License version 2.0.

Data structures derived from https://github.com/OpenTTD/OpenTTD and https://github.com/JGRennison/OpenTTD-patches are also licensed under GPL 2.0.
