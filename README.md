# OpenTTD Map Parser (and Writer)

This library is a 🔥blazing🔥 fast WIP map parser and writer for OpenTTD written in Rust🚀🚀🚀 with ✨[binrw](https://binrw.rs/)✨.
The outer structure can be fully parsed/written but you will have to know the structure details to do more interesting things.

Right now you can use the town renamer CLI to import/export a list of town names specifically for JGR savefiles.

### Save to towns.json

```sh
cargo run --release --example town-renamer read ./game.sav
```

### towns.json to Save

```sh
cargo run --release --example town-renamer write ./game.sav ./towns.json -o new_save.sav
```

### Useful links

- [OpenTTD's Savegame Format](https://github.com/OpenTTD/OpenTTD/blob/master/docs/savegame_format.md)
- [OpenTTD's Savegame Compression](https://wiki.openttd.org/en/Archive/Manual/Settings/Savegame%20format)
- [OpenTTD Source](https://github.com/OpenTTD/OpenTTD)
- [JGR Source](https://github.com/JGRennison/OpenTTD-patches)
