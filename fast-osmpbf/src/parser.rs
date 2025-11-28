use crate::{
    Blob, DenseNodeBlock, ElementBlock, NodeBlock, PrimitiveBlock, RelationBlock, WayBlock,
    ELEMENT_FILTER, TAG_KEYS_FILTER,
};
use quick_protobuf::{BytesReader, MessageRead};
use std::{borrow::Cow, io::Read, sync::Arc};

pub(crate) struct OsmParser;
impl OsmParser {
    /// Deserialize blob_slices into a Blob.
    /// Then decompresses the blob if its stored in a compressed state.
    /// Then parses ElementBlocks inside the decompressed blob.
    pub(crate) fn deserialize_blob(blob_slice: Arc<[u8]>) -> std::io::Result<Vec<ElementBlock>> {
        // Deserialize blob
        let mut reader = BytesReader::from_bytes(&blob_slice);
        let blob = Blob::from_reader(&mut reader, &blob_slice)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // either take the raw_size if available or use 2 * compressed_size as heuristic
        let size = match blob.raw_size {
            Some(raw_size) => raw_size as usize,
            None => blob_slice.len() * 2,
        };
        let mut decompressed_blob: Vec<u8> = Vec::with_capacity(size);
        if let Some(raw) = &blob.raw {
            decompressed_blob.extend_from_slice(raw);
        } else if let Some(zlib) = &blob.zlib_data {
            let mut decoder = flate2::read::ZlibDecoder::new(&zlib[..]);
            decoder.read_to_end(&mut decompressed_blob)?;
        } else if let Some(lzma) = &blob.lzma_data {
            let mut decoder = xz2::read::XzDecoder::new(&lzma[..]);
            decoder.read_to_end(&mut decompressed_blob)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Empty OSMData blob",
            ));
        };

        return Self::parse_blob(&decompressed_blob);
    }
    // Processes a blob in parallel using rayon (one task per PrimitiveGroup)
    fn parse_blob(blob: &[u8]) -> std::io::Result<Vec<ElementBlock>> {
        let mut reader = BytesReader::from_bytes(blob);
        let block = PrimitiveBlock::from_reader(&mut reader, blob)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let table: Vec<Cow<'static, [u8]>> = block
            .stringtable
            .s
            .into_iter()
            .map(|s| Cow::Owned(s.to_vec()))
            .collect();
        let stringtable = Arc::new(table);
        let cached_tag_ids = match TAG_KEYS_FILTER.get() {
            Some(_) => Self::get_tag_ids(&stringtable),
            None => Arc::new(Vec::with_capacity(0)),
        };

        let element_filter = ELEMENT_FILTER.get();

        let element_count: usize = block
            .primitivegroup
            .iter()
            .map(|g| {
                g.nodes.len()
                    + g.ways.len()
                    + g.relations.len()
                    + g.dense.as_ref().map_or(0, |d| d.id.len())
            })
            .sum();
        let mut elements: Vec<ElementBlock> = Vec::with_capacity(element_count);

        for group in block.primitivegroup {
            if let Some(dense_nodes) = group.dense {
                if element_filter.map_or(true, |f| f.nodes) {
                    let table = Arc::clone(&stringtable);
                    elements.push(ElementBlock::DenseNodeBlock(DenseNodeBlock {
                        table,
                        cached_tag_ids: Arc::clone(&cached_tag_ids),
                        granularity: block.granularity,
                        lat_offset: block.lat_offset,
                        lon_offset: block.lon_offset,
                        kv_offsets: Self::compute_offsets(
                            &dense_nodes.keys_vals,
                            dense_nodes.id.len(),
                        ),
                        nodes: Arc::from(dense_nodes),
                    }));
                }
            }
            if !group.nodes.is_empty() {
                if element_filter.map_or(true, |f| f.nodes) {
                    let table = Arc::clone(&stringtable);
                    elements.push(ElementBlock::NodeBlock(NodeBlock {
                        nodes: Arc::from(group.nodes),
                        cached_tag_ids: Arc::clone(&cached_tag_ids),
                        table,
                    }));
                }
            }

            if !group.ways.is_empty() {
                if element_filter.map_or(true, |f| f.ways) {
                    let table = Arc::clone(&stringtable);
                    elements.push(ElementBlock::WayBlock(WayBlock {
                        ways: Arc::from(group.ways),
                        cached_tag_ids: Arc::clone(&cached_tag_ids),
                        table,
                    }));
                }
            }

            if !group.relations.is_empty() {
                if element_filter.map_or(true, |f| f.relations) {
                    let table = Arc::clone(&stringtable);
                    elements.push(ElementBlock::RelationBlock(RelationBlock {
                        relations: Arc::from(group.relations),
                        cached_tag_ids: Arc::clone(&cached_tag_ids),
                        table,
                    }));
                }
            }
        }

        Ok(elements)
    }

    // Gets tag ids from stringtable if corresponding value is in TAG_KEYS_CACHE
    fn get_tag_ids(table: &[Cow<'_, [u8]>]) -> Arc<Vec<u32>> {
        Arc::new(
            table
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let key = unsafe { std::str::from_utf8_unchecked(s) };
                    let cache = TAG_KEYS_FILTER.get().unwrap();

                    // Branchless linear scan for ≤8 elements
                    if cache.iter().any(|&k| k == key) {
                        Some(i as u32)
                    } else {
                        None
                    }
                })
                .collect::<Vec<u32>>(),
        )
    }

    // Computes offsets for keys_vals in DenseNodes
    // key_vals looks like [k, v, k, v, k, v, ..., 0, k, v, k, v ... 0 ...]
    fn compute_offsets(keys_vals: &[i32], node_count: usize) -> Vec<usize> {
        let mut offsets = Vec::with_capacity(node_count + 1);
        offsets.push(0);

        let mut idx = 0;

        for _ in 0..node_count {
            while idx < keys_vals.len() && keys_vals[idx] != 0 {
                idx += 2; // skip k, v pair
            }

            if idx >= keys_vals.len() {
                // malformed, but avoid UB
                offsets.push(idx);
                continue;
            }

            idx += 1; // skip terminating zero
            offsets.push(idx);
        }

        offsets
    }
}
