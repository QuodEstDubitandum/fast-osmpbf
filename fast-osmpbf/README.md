fast-osmpbf
======
A high-performance Rust library for reading OpenStreetMap files ([*.osm.pbf](https://wiki.openstreetmap.org/wiki/PBF_Format)).
This library is focused on performance and provides pretty high-level data. if you need some lower level metadata,
you might want to check out other libraries such as [osmpbf](https://crates.io/crates/osmpbf).

[![Crates.io](https://img.shields.io/crates/v/osmpbf.svg)](https://crates.io/crates/fast-osmpbf)
[![Documentation](https://docs.rs/fast-osmpbf/badge.svg)](https://docs.rs/fast-osmpbf)

## Examples

1) Count ways.
We can (but dont have to) apply an element filter beforehand. Saves a little bit of computation.

```rust
use fast_osmpbf::*;

fn main() {
    let arg = std::env::args_os()
        .nth(1)
        .expect("need a *.osm.pbf file as argument");
    let path = std::path::Path::new(&arg);
    let reader = OsmReader::from_path(path).expect("Invalid file path");

    // apply filter
    reader
        .apply_element_filter(ElementFilter {
            nodes: false,
            relations: false,
            ways: true,
        })
        .expect("Invalid element filter");

    // iterate using .blocks() (Parallelization happens, but only for one decoding step)
    let mut way_counter = 0;
    reader.blocks().for_each(|block| match block {
        ElementBlock::WayBlock(block) => {
            for _way in block.iter() {
                way_counter += 1;
            }
        }
        _ => (),
    });
    println!("Ways: {:?}", way_counter);
}
```

2) Count elements that have full addresses.
We apply a filter on tags beforehand. This filter does not only filtering on tags, it actually speeds up computing by using a cache.
If you iterate over tags and you know you only need certain tags, apply the filter beforehand.

```rust
use fast_osmpbf::*;
use fast_osmpbf::prelude::*;

pub fn main() {
    let arg = std::env::args_os()
        .nth(1)
        .expect("need a *.osm.pbf file as argument");
    let path = std::path::Path::new(&arg);
    let reader = OsmReader::from_path(path).expect("Invalid file path");

    // apply filter
    reader
        .apply_tag_filter(&[
            "addr:city",
            "addr:postcode",
            "addr:street",
            "addr:housenumber",
        ])
        .expect("Invalid filter applied");

    // iterate using .par_blocks() (Parallelization happens on 2 decoding steps)
    let address_counter: usize = reader
        .par_blocks()
        .map(|block| match block {
            ElementBlock::DenseNodeBlock(block) => block
                .iter()
                .filter(|node| node.tags().has_all_filter_keys())
                .count(),
            ElementBlock::NodeBlock(block) => block
                .iter()
                .filter(|node| node.tags().has_all_filter_keys())
                .count(),
            ElementBlock::WayBlock(block) => block
                .iter()
                .filter(|way| way.tags().has_all_filter_keys())
                .count(),
            ElementBlock::RelationBlock(block) => block
                .iter()
                .filter(|rel| rel.tags().has_all_filter_keys())
                .count(),
        })
        .sum();
    println!("Addresses: {:?}", address_counter);
}
```

## Benchmarks

- CPU: Intel(R) Core(TM) i5-9600K CPU @ 3.70GHz
- Memory: 16GB
- OS: Linux (Ubuntu)
- Dataset: `germany-latest.osm.pbf` (~4.6GB)

```
Hyperfine was used to benchmark with 10 runs each.
What you see here is the mean time of these 10 runs.
All problems were run using parallelization if the library offered a way to do it.
```


| Problem         | fast-osmpbf |  osmpbf  | osmpbfreader |
|-----------------|-------------|----------|--------------|
| Count ways      | 13.51 s     | 19.06 s  | 39.58 s      |
| Count addresses | 15.33 s     | 21.70 s  | 45.45 s      |


## License

This project is licensed under

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)
