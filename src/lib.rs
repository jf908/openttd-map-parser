#[cfg(target_arch = "wasm32")]
use binrw::{io::Cursor, BinReaderExt};
#[cfg(target_arch = "wasm32")]
use save::{OuterSave, Save};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
    let mut c = Cursor::new(data);
    let output: Save = c.read_ne().unwrap();

    serde_wasm_bindgen::to_value(&output).unwrap()
}
