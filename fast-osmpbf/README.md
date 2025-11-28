fast-osmpbf
======
A high-performance Rust library for reading OpenStreetMap files ([*.osm.pbf](https://wiki.openstreetmap.org/wiki/PBF_Format)).
This library is focused on performance and provides pretty high-level data. if you need some lower level metadata,
you might want to check out other libraries such as [osmpbf](https://github.com/b-r-u/osmpbf).

## Installation

```bash
cargo add fast-osmpbf
```

or add this to your `Cargo.toml`:

```toml
[dependencies]
fast-osmpbf = "0.1"
```

## Examples

1) Count ways.
This uses parallelization on one level. For parallelization on 2 levels, use .par_blocks() instead.
We can (but dont have to) apply an element filter beforehand. Saves a little bit of computation.

```rust
use fast_osmpbf::*;

fn main() {
    let arg = std::env::args_os()
        .nth(1)
        .expect("need a *.osm.pbf file as argument");
    let path = std::path::Path::new(&arg);
    let reader = OsmReader::from_path(path).expect("Invalid file path");
    reader
        .apply_element_filter(ElementFilter {
            nodes: false,
            relations: false,
            ways: true,
        })
        .expect("Invalid element filter");

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
    reader
        .apply_tag_filter(&[
            "addr:city",
            "addr:postcode",
            "addr:street",
            "addr:housenumber",
        ])
        .expect("Invalid filter applied");
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
