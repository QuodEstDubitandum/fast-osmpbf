fast-osmpbf-js
======
A library for reading OpenStreetMap files ([*.osm.pbf](https://wiki.openstreetmap.org/wiki/PBF_Format)).
It is using NodeJS bindings for the high-performance Rust library [fast-osmpbf](https://github.com/QuodEstDubitandum/fast-osmpbf).

[![npm](https://img.shields.io/npm/v/fast-osmpbf-js.svg)](https://www.npmjs.com/package/fast-osmpbf-js)

## Installation

```bash
npm install fast-osmpbf-js
```

or add this to your `package.json`:

```json
"fast-osmpbf-js": "0.1"
```

## Examples

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

## Structure

The stream gives you a `JsElementBlock` object to work with.
It is basicly a collection of elements (`Node`, `Way` or `Relation`).
Most of the information can be read from that. Though sometimes its still encoded for performance reasons,
so you need to call one of my helper functions that decodes it.

Using the 2 filters like described in the examples above also can help with performance (and filtering).

```js
export interface JsElementBlock {
  ids: BigInt64Array
  elementType: string // Node, Way or Relation
  nodeIds?: [BigInt64Array, Uint32Array] // encoded, dont use, instead use the getNodeIds function
  latitudes?: Float64Array
  longitudes?: Float64Array
  relationMembers?: [BigInt64Array, Uint8Array, Int32Array, Uint32Array] // encoded, dont use, instead use the getRelationMembers function
  denseTags?: [Uint32Array, Uint32Array, Uint32Array] // encoded, dont use, instead use getTags function
  tags?: [Uint32Array, Uint32Array, Uint32Array] // encoded, dont use, instead use getTags function
  stringTable: Array<string> // probably irrelevant for you, needed for functions to decode encoded data
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


| Problem         | fast-osmpbf-js | osm-pbf-parser | osm-read |
|-----------------|----------------|----------------|----------|
| Count ways      | 18.10 s        | 330.57 s       | 523.80 s |
| Count addresses | 30.04 s        | 359.06 s       | 603.33 s |


## License

This project is licensed under

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)
