#[cfg(target_arch = "wasm32")]
use binrw::{io::Cursor, BinReaderExt, BinWrite};
#[cfg(target_arch = "wasm32")]
use gloo_utils::format::JsValueSerdeExt;
#[cfg(target_arch = "wasm32")]
use save::{OuterSave, Save};
#[cfg(target_arch = "wasm32")]
use serde::Serialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

pub mod charray;
pub mod chtable;
pub mod gamma;
pub mod helpers;
pub mod jgr;
pub mod save;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_outer_file(data: &[u8]) -> JsValue {
    let mut c = Cursor::new(data);
    let output: OuterSave = c.read_ne().unwrap();

    serde_wasm_bindgen::to_value(&output).unwrap()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_file(data: &[u8]) -> JsValue {
    console_error_panic_hook::set_once();
    let mut c = Cursor::new(data);
    let output: Save = c.read_ne().unwrap();

    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    output.serialize(&serializer).unwrap()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_file_json(data: &[u8]) -> JsValue {
    console_error_panic_hook::set_once();
    let mut c = Cursor::new(data);
    let output: Save = c.read_ne().unwrap();
    JsValue::from_serde(&output).unwrap()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn write_file(data: JsValue) -> Box<[u8]> {
    console_error_panic_hook::set_once();
    let save: Save = serde_wasm_bindgen::from_value(data).unwrap();
    let mut buffer = Vec::new();
    {
        let mut writer = Cursor::new(&mut buffer);
        Save::write_be(&save, &mut writer).unwrap();
    }

    buffer.into_boxed_slice()
}
