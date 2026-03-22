#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
