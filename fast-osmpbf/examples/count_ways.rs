// Count the number of ways in an .osm.pbf file

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
