use quick_protobuf::{BytesReader, MessageRead};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{
    parser::OsmParser, BlobHeader, ElementBlock, ElementBlockIter, ElementFilter, ELEMENT_FILTER,
    TAG_KEYS_FILTER, TAG_KEYS_FILTER_COUNT,
};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    sync::Arc,
};

const BUF_SIZE: usize = 1024 * 1024; // 1MB
const MAX_BLOB_SIZE: usize = 1 * 1024 * 1024; // 32MB
const MAX_HEADER_SIZE: usize = 64 * 1024; // 64KB
const MAX_Q_ELEMENTS: usize = 1_000;
const MAX_TAGS: usize = 8;

/// Reader that reads bytes from .osm.pbf file and passes them on to the parser
#[derive(Debug)]
pub struct OsmReader {
    reader: BufReader<File>,
    header: Vec<u8>,
    blob: Vec<u8>,
}

impl OsmReader {
    /// Creates a new OsmReader from a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path_ref = path.as_ref();
        let path = path.as_ref().to_string_lossy().to_string();
        if !path.ends_with(".osm.pbf") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File {} is not a .osm.pbf file", path),
            ));
        }
        let file = File::open(path_ref)?;
        let reader = BufReader::with_capacity(BUF_SIZE, file);

        Ok(Self {
            reader,
            header: Vec::with_capacity(MAX_HEADER_SIZE),
            blob: Vec::with_capacity(MAX_BLOB_SIZE),
        })
    }

    /// Filters elements (dense_nodes, nodes, ways or relations) depending on the filter provided.
    /// If you only are interested in specific elements, I highly encourage you to use this mechanism
    /// over filtering yourself in the iterator since it not only does the filtering for you,
    /// but actually speeds up computation.
    pub fn apply_element_filter(&self, filter: ElementFilter) -> Result<(), &'static str> {
        if ELEMENT_FILTER.get().is_some() {
            return Err("You cannot apply a filter more than once");
        }

        let _ = ELEMENT_FILTER.set(filter);
        Ok(())
    }

    /// Filters out all tags (key, value) where key is not one of your provided Strings.
    /// If you only are interested in specific tags, I highly encourage you to use this mechanism
    /// over filtering yourself in the iterator since it not only does the filtering for you,
    /// but actually speeds up computation by abusing a caching mechanism.
    /// You can provide between 0 and 8 filter keys.
    pub fn apply_tag_filter(&self, tags: &[&str]) -> Result<(), &'static str> {
        if TAG_KEYS_FILTER.get().is_some() {
            return Err("You cannot apply a filter more than once");
        }

        let total_tags = tags.len();

        if total_tags > MAX_TAGS {
            return Err("Not allowed to provide more than 8 tags");
        }

        let mut leaked: Vec<&'static str> = tags
            .iter()
            .map(|t| {
                let boxed: Box<str> = t.to_string().into_boxed_str();
                let s: &'static mut str = Box::leak(boxed);
                &*s // coerce &'static mut str → &'static str
            })
            .collect();

        leaked.sort_unstable();

        let _ = TAG_KEYS_FILTER.set(leaked.into_boxed_slice());
        let _ = TAG_KEYS_FILTER_COUNT.set(tags.len());
        Ok(())
    }

    /// Creates a parallel iterator that yields [`ElementBlock`]
    pub fn par_blocks(self) -> impl ParallelIterator<Item = ElementBlock> {
        self.blocks().par_bridge()
    }

    /// Creates an iterator that yields [`ElementBlock`]
    pub fn blocks(self) -> ElementBlockIter {
        let num_threads = rayon::current_num_threads();
        let (blob_tx, blob_rx) = crossbeam_channel::bounded::<Arc<[u8]>>(num_threads);
        let (element_block_tx, element_block_rx) =
            crossbeam_channel::bounded::<ElementBlock>(MAX_Q_ELEMENTS);

        // Spawn a thread to continuously read blobs
        std::thread::spawn(move || {
            let mut reader = self;
            while let Ok(Some(blob)) = reader.next_blob() {
                if blob_tx.send(blob).is_err() {
                    break;
                }
            }
        });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .expect("Failed to create thread pool");

        // Spawn parsing tasks inside the pool
        std::thread::spawn(move || {
            pool.install(|| {
                blob_rx.into_iter().par_bridge().for_each(|blob| {
                    if let Ok(element_blocks) = OsmParser::deserialize_blob(blob) {
                        for block in element_blocks {
                            if element_block_tx.send(block).is_err() {
                                return;
                            }
                        }
                    }
                });
            });
        });

        ElementBlockIter {
            rx: element_block_rx,
        }
    }

    // Sequential operation - raw blobs have different sizes, need to look at length prefix and blob header first to know exact size
    fn next_blob(&mut self) -> std::io::Result<Option<Arc<[u8]>>> {
        let mut prefix = [0u8; 4];

        // Read length prefix (always 4 bytes)
        if self.reader.read_exact(&mut prefix).is_err() {
            return Ok(None); // EOF
        }

        let header_size = u32::from_be_bytes(prefix) as usize;
        if header_size > self.header.capacity() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "BlobHeader size exceeds limit of 64KB. File corrupt?",
            ));
        }

        if self.header.len() < header_size {
            self.header.resize(header_size, 0);
        }
        self.reader.read_exact(&mut self.header[..header_size])?;

        // Deserialize blob header to get size of blob
        let mut reader = BytesReader::from_bytes(&self.header[..header_size]);
        let header = BlobHeader::from_reader(&mut reader, &self.header[..header_size])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let blob_size = header.datasize as usize;

        // Skip everything that is not actual relevant data
        if header.type_pb != "OSMData" {
            self.reader.seek_relative(blob_size as i64)?;
            return self.next_blob();
        }

        if self.blob.len() < blob_size {
            // grow buffer slightly larger to reduce repeated reallocs
            let new_capacity = (blob_size * 2) as usize;
            self.blob.resize(new_capacity, 0);
        }

        if self.blob.len() < blob_size {
            self.blob.resize(blob_size, 0);
        }
        self.reader.read_exact(&mut self.blob[..blob_size])?;
        let blob_slice: Arc<[u8]> = Arc::from(&self.blob[..blob_size]);

        return Ok(Some(blob_slice));
    }
}
