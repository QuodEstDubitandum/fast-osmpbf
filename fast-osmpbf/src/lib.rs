#![doc = include_str!("../README.md")]

include!(concat!(env!("OUT_DIR"), "/proto/mod.rs"));

/// Contains Element and corresponding Iterator
pub mod element;
/// Handles parsing .osm.pbf files
pub mod parser;
/// Prelude
pub mod prelude;
/// Contains
pub mod reader;
/// Contains function to run simd calculations
pub mod simd;

pub use element::*;
pub use osmdata::*;
pub use osmformat::*;
pub use reader::*;
