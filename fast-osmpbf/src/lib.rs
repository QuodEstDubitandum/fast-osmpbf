#![doc = include_str!("../README.md")]

include!(concat!(env!("OUT_DIR"), "/proto/mod.rs"));

/// Contains Element and corresponding Iterator
pub mod element;
/// Handles parsing .osm.pbf files
pub mod parser;
/// Prelude
pub mod prelude;
/// Contains Reader and methods to apply filters
pub mod reader;

pub use element::*;
pub use osmdata::*;
pub use osmformat::*;
pub use reader::*;
