fast-osmpbf
======
A high-performance Rust library for reading OpenStreetMap files [*.osm.pbf][https://wiki.openstreetmap.org/wiki/PBF_Format].
Also exposes NodeJS bindings. Available on cargo and npm.

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

### Rust

| Problem         | fast-osmpbf |  osmpbf  | osmpbfreader |
|-----------------|-------------|----------|--------------|
| Count ways      | 13.51 s     | 19.06 s  | 39.58 s      |
| Count addresses | 15.33 s     | 21.70 s  | 45.45 s      |

### NodeJS

| Problem         | fast-osmpbf-js | osm-pbf-parser | osm-read |
|-----------------|----------------|----------------|----------|
| Count ways      | 18.10 s        | 330.57 s       | 523.80 s |
| Count addresses | 30.04 s        | 359.06 s       | 603.33 s |

## Installation

### Rust

```bash
cargo add fast-osmpbf
```

or add this to your `Cargo.toml`:

```toml
[dependencies]
fast-osmpbf = "0.1"
```

### NodeJS

```bash
npm install fast-osmpbf-js
```

or add this to your `package.json`:

```json
"fast-osmpbf-js" = "0.1"
```

## Examples
### Rust

1) Count ways.
This uses parallelization on one level. For parallelization on 2 levels, use .par_blocks() instead.
We can (but dont have to) apply an element filter beforehand. In the original Rust library version,
this does not speed up computation by a lot because blocks are loaded lazily.

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

### NodeJS

1) Count ways.
Since we dont need tags here and we know we only need ways, we apply a tag filter and an element filter beforehand,
which basicly avoids sending any tag elements, nodes and relations from Rust to NodeJS. This speeds up computation significantly.

```js
import { JsElementBlock, OsmReader, getNodeIds, getRelationMembers, getTags } from "fast-osmpbf-js"

const reader = new OsmReader("./scripts/osm-data/germany-latest.osm.pbf")
const relevantTags: string[] = []
const relevantElements: JsElementFilter = {
    nodes: false,
    ways: true,
    relations: false,
}
const stream = reader.streamBlocks(relevantElements, relevantTags)

async function main() {
    let totalItems = 0
    let block: JsElementBlock | null = null
    while ((block = await stream.next()) !== null) {
      const { ids } = block
      totalItems += ids.length
    }

    return totalItems
}
```

2) Count elements that have full addresses.
Since we know which tags we need here, we apply a tag filter and speed up computation here as well.

```js
import { JsElementBlock, OsmReader, getTags } from "fast-osmpbf-js"

const reader = new OsmReader("./scripts/osm-data/germany-latest.osm.pbf")
const relevantTags = ["addr:city", "addr:postcode", "addr:street", "addr:housenumber"]
const stream = reader.streamBlocks(relevantTags)

async function main() {
    let totalItems = 0
    let block: JsElementBlock | null = null
    while ((block = await stream.next()) !== null) {
        const { ids } = block
        const blockLength = ids.length

        for (let i = 0; i < blockLength; i++) {
            const tags = getTags(block, i)
            if (tags.length === relevantTags.length) {
                totalItems++
            }
        }
    }

    return totalItems
}
```

## License

This project is licensed under

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)
