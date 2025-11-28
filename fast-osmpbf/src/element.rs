use crate::{DenseNodes, MemberType, Node, Relation, Way};
use crossbeam_channel::Receiver;
use std::sync::OnceLock;
use std::{borrow::Cow, sync::Arc};

pub(crate) static TAG_KEYS_FILTER: OnceLock<Box<[&'static str]>> = OnceLock::new();
pub(crate) static TAG_KEYS_FILTER_COUNT: OnceLock<usize> = OnceLock::new();

pub(crate) static ELEMENT_FILTER: OnceLock<ElementFilter> = OnceLock::new();

/// An optional filter you can apply that speeds up computation
pub struct ElementFilter {
    /// Whether [`Node`] and [`DenseNodes`] should be parsed
    pub nodes: bool,
    /// Whether [`Way`] should be parsed
    pub ways: bool,
    /// Whether [`Relation`] should be parsed
    pub relations: bool,
}

/// An ElementBlock is an enum that holds variants where each block variant
/// is a wrapper around multiple elements ([`DenseNodes`], [`Node`], [`Way`] or [`Relation`]).
///
/// For more details on OSM elements, see the [OSM wiki](https://wiki.openstreetmap.org/wiki/Elements).
#[derive(Debug)]
pub enum ElementBlock {
    /// Block of [`DenseNodes`]
    DenseNodeBlock(DenseNodeBlock),
    /// Block of [`Node`]
    NodeBlock(NodeBlock),
    /// Block of [`Way`]
    WayBlock(WayBlock),
    /// Block of [`Relation`]
    RelationBlock(RelationBlock),
}

// --------------------------- DENSE_NODE ---------------------------
// --------------------------- DENSE_NODE ---------------------------
// --------------------------- DENSE_NODE ---------------------------

/// A Wrapper to hold DenseNodes which gets lazy decoded via iterator.
/// Main use is for performance reasons. Use .iter() on it to iterate over it.
#[derive(Debug)]
pub struct DenseNodeBlock {
    pub(crate) nodes: Arc<DenseNodes>,
    pub(crate) table: Arc<Vec<Cow<'static, [u8]>>>,
    pub(crate) cached_tag_ids: Arc<Vec<u32>>,
    pub(crate) granularity: i64,
    pub(crate) lat_offset: i64,
    pub(crate) lon_offset: i64,
    pub(crate) kv_offsets: Vec<usize>,
}
impl DenseNodeBlock {
    /// Create an iter over [`DenseNodeRef`]
    pub fn iter(&self) -> impl Iterator<Item = DenseNodeRef<'_>> {
        DenseNodeIter {
            block: self,
            cached_tag_ids: &self.cached_tag_ids,
            index: 0,
            len: self.nodes.id.len(),
            prev_id: 0,
            prev_lat: 0,
            prev_lon: 0,
        }
    }
    /// Get the number of [`DenseNodeRef`]
    pub fn len(&self) -> usize {
        self.nodes.id.len()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_string_table(&self) -> Vec<String> {
        self.table
            .iter()
            .map(|cow| unsafe { std::str::from_utf8_unchecked(cow.as_ref()) }.to_owned())
            .collect()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_raw_data(&self) -> (Vec<i64>, Vec<f64>, Vec<f64>, Vec<u32>, Vec<u32>, Vec<u32>) {
        let len = self.nodes.id.len();
        let mut ids = Vec::with_capacity(len);
        let mut latitudes = Vec::with_capacity(len);
        let mut longitudes = Vec::with_capacity(len);

        let mut key_ids = Vec::with_capacity(self.nodes.keys_vals.len() / 2);
        let mut val_ids = Vec::with_capacity(self.nodes.keys_vals.len() / 2);
        let mut kv_offsets = Vec::with_capacity(self.kv_offsets.len() + 1);
        kv_offsets.push(0);

        // accumulators for delta decoding
        let mut last_id = 0i64;
        let mut last_lat = 0i64;
        let mut last_lon = 0i64;

        let use_cache = TAG_KEYS_FILTER.get().is_some();

        for node_idx in 0..len {
            // delta decode
            last_id += self.nodes.id[node_idx];
            last_lat += self.nodes.lat[node_idx];
            last_lon += self.nodes.lon[node_idx];

            ids.push(last_id);
            latitudes.push((last_lat * self.granularity + self.lat_offset) as f64 * 1e-9);
            longitudes.push((last_lon * self.granularity + self.lon_offset) as f64 * 1e-9);

            let start = self.kv_offsets[node_idx];
            let end = self
                .kv_offsets
                .get(node_idx + 1)
                .copied()
                .unwrap_or(self.nodes.keys_vals.len());

            let mut i = start;
            while i + 1 < end {
                let key = self.nodes.keys_vals[i] as u32;
                let val = self.nodes.keys_vals[i + 1] as u32;
                if !use_cache || self.cached_tag_ids.contains(&key) {
                    key_ids.push(key);
                    val_ids.push(val);
                }
                i += 2;
            }

            kv_offsets.push(key_ids.len() as u32);
        }

        (ids, latitudes, longitudes, key_ids, val_ids, kv_offsets)
    }
}

/// A Reference to a DenseNode
#[derive(Debug)]
pub struct DenseNodeRef<'a> {
    pub(crate) block: &'a DenseNodeBlock,
    pub(crate) cached_tag_ids: &'a [u32],
    pub(crate) index: usize,
    pub(crate) prev_id: i64,
    pub(crate) prev_lat: i64,
    pub(crate) prev_lon: i64,
}

impl<'a> DenseNodeRef<'a> {
    /// Get ID
    #[inline]
    pub fn id(&mut self) -> i64 {
        self.prev_id += self.block.nodes.id[self.index];
        self.prev_id
    }
    /// Get Latitude
    #[inline]
    pub fn lat(&mut self) -> f64 {
        self.prev_lat += self.block.nodes.lat[self.index];
        ((self.prev_lat * self.block.granularity + self.block.lat_offset) as f64) * 1e-9
    }
    /// Get Longitude
    #[inline]
    pub fn lon(&mut self) -> f64 {
        self.prev_lon += self.block.nodes.lon[self.index];
        ((self.prev_lon * self.block.granularity + self.block.lon_offset) as f64) * 1e-9
    }
    /// Get Iterator of (key, value) pairs
    #[inline]
    pub fn tags(&self) -> DenseNodeTagIter<'_> {
        let start = self.block.kv_offsets[self.index];
        let end = self.block.kv_offsets[self.index + 1];

        let slice = &self.block.nodes.keys_vals[start..end];
        let table = &self.block.table;

        DenseNodeTagIter {
            slice,
            table,
            pos: 0,
            cached_tag_ids: self.cached_tag_ids,
            use_cache: TAG_KEYS_FILTER.get().is_some(),
        }
    }
}
struct DenseNodeIter<'a> {
    block: &'a DenseNodeBlock,
    cached_tag_ids: &'a [u32],
    index: usize,
    len: usize,
    prev_id: i64,
    prev_lat: i64,
    prev_lon: i64,
}

impl<'a> Iterator for DenseNodeIter<'a> {
    type Item = DenseNodeRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }

        let node = &self.block.nodes;

        // Create the DenseNodeRef with current previous values
        let out = DenseNodeRef {
            block: self.block,
            index: self.index,
            cached_tag_ids: self.cached_tag_ids,
            prev_id: self.prev_id,
            prev_lat: self.prev_lat,
            prev_lon: self.prev_lon,
        };

        // Update the accumulators for the next node
        self.prev_id += node.id[self.index];
        self.prev_lat += node.lat[self.index];
        self.prev_lon += node.lon[self.index];

        self.index += 1;
        Some(out)
    }
}

// --------------------------- NODE ---------------------------
// --------------------------- NODE ---------------------------
// --------------------------- NODE ---------------------------

/// A block of [`Node`].
/// Use .iter() on it to iterate over nodes.
#[derive(Debug)]
pub struct NodeBlock {
    pub(crate) nodes: Arc<Vec<Node>>,
    pub(crate) cached_tag_ids: Arc<Vec<u32>>,
    pub(crate) table: Arc<Vec<Cow<'static, [u8]>>>,
}
impl NodeBlock {
    /// Creates an iterator over [`NodeRef`]
    pub fn iter(&self) -> impl Iterator<Item = NodeRef<'_>> {
        let mut prev_lat = 0i64;
        let mut prev_lon = 0i64;

        self.nodes.iter().map(move |node| {
            let node_ref = NodeRef {
                node,
                cached_tag_ids: &self.cached_tag_ids,
                table: &self.table,
                prev_lat,
                prev_lon,
            };

            prev_lat += node.lat;
            prev_lon += node.lon;

            node_ref
        })
    }
    /// Get the number of [`NodeRef`]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_string_table(&self) -> Vec<String> {
        self.table
            .iter()
            .map(|cow| unsafe { std::str::from_utf8_unchecked(cow.as_ref()) }.to_owned())
            .collect()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_raw_data(&self) -> (Vec<i64>, Vec<f64>, Vec<f64>, Vec<u32>, Vec<u32>, Vec<u32>) {
        let len = self.nodes.len();

        let mut ids = Vec::with_capacity(len);
        let mut lats = Vec::with_capacity(len);
        let mut lons = Vec::with_capacity(len);

        let mut key_ids = Vec::new();
        let mut val_ids = Vec::new();
        let mut kv_offsets = Vec::with_capacity(len + 1);
        kv_offsets.push(0);

        // accumulators for delta decoding
        let mut last_lat = 0i64;
        let mut last_lon = 0i64;

        let use_cache = TAG_KEYS_FILTER.get().is_some();

        for node in self.nodes.iter() {
            ids.push(node.id);

            last_lat += node.lat;
            last_lon += node.lon;
            lats.push(last_lat as f64 * 1e-9);
            lons.push(last_lon as f64 * 1e-9);

            // append all tags for this node
            for (k, v) in node.keys.iter().zip(node.vals.iter()) {
                if !use_cache || self.cached_tag_ids.contains(k) {
                    key_ids.push(*k);
                    val_ids.push(*v);
                }
            }
            kv_offsets.push(key_ids.len() as u32);
        }

        (ids, lats, lons, key_ids, val_ids, kv_offsets)
    }
}
/// A Reference to a [`Node`]
#[derive(Debug)]
pub struct NodeRef<'a> {
    node: &'a Node,
    cached_tag_ids: &'a [u32],
    table: &'a [Cow<'static, [u8]>],
    prev_lat: i64,
    prev_lon: i64,
}
impl<'a> NodeRef<'a> {
    /// Get ID
    #[inline]
    pub fn id(&self) -> i64 {
        self.node.id
    }
    /// Get Latitude
    #[inline]
    pub fn lat(&mut self) -> f64 {
        self.prev_lat += self.node.lat;
        self.prev_lat as f64 * 1e-9
    }
    /// Get Longitude
    #[inline]
    pub fn lon(&mut self) -> f64 {
        self.prev_lon += self.node.lon;
        self.prev_lon as f64 * 1e-9
    }
    /// Get Iterator over (key, value) pairs
    #[inline]
    pub fn tags(&self) -> TagIter<'_> {
        TagIter {
            keys: &self.node.keys,
            vals: &self.node.vals,
            table: self.table,
            pos: 0,
            cached_tag_ids: self.cached_tag_ids,
            use_cache: TAG_KEYS_FILTER.get().is_some(),
        }
    }
}

// --------------------------- WAY ---------------------------
// --------------------------- WAY ---------------------------
// --------------------------- WAY ---------------------------

/// A block of [`Way`].
/// Use .iter() on it to iterate over ways.
#[derive(Debug)]
pub struct WayBlock {
    pub(crate) ways: Arc<Vec<Way>>,
    pub(crate) cached_tag_ids: Arc<Vec<u32>>,
    pub(crate) table: Arc<Vec<Cow<'static, [u8]>>>,
}
impl WayBlock {
    /// Creates an iterator over [`WayRef`]
    pub fn iter(&self) -> impl Iterator<Item = WayRef<'_>> {
        self.ways.iter().map(move |way| WayRef {
            way,
            cached_tag_ids: &self.cached_tag_ids,
            table: &self.table,
        })
    }
    /// Get the number of [`WayRef`]
    pub fn len(&self) -> usize {
        self.ways.len()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_string_table(&self) -> Vec<String> {
        self.table
            .iter()
            .map(|cow| unsafe { std::str::from_utf8_unchecked(cow.as_ref()) }.to_owned())
            .collect()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_raw_data(&self) -> (Vec<i64>, Vec<u32>, Vec<u32>, Vec<u32>, Vec<i64>, Vec<u32>) {
        let len = self.ways.len();
        let mut ids = Vec::with_capacity(len);

        let mut key_ids = Vec::new();
        let mut val_ids = Vec::new();
        let mut kv_offsets = Vec::with_capacity(len + 1);
        kv_offsets.push(0);

        let mut node_ids = Vec::new();
        let mut node_offsets = Vec::with_capacity(len + 1);
        node_offsets.push(0);

        let use_cache = TAG_KEYS_FILTER.get().is_some();

        for way in self.ways.iter() {
            ids.push(way.id);

            // append all tags for this node
            for (k, v) in way.keys.iter().zip(way.vals.iter()) {
                if !use_cache || self.cached_tag_ids.contains(k) {
                    key_ids.push(*k);
                    val_ids.push(*v);
                }
            }
            kv_offsets.push(key_ids.len() as u32);

            // node_ids are delta encoded
            let mut last_node_id = 0i64;
            for delta in way.refs.iter() {
                last_node_id += *delta;
                node_ids.push(last_node_id);
            }
            node_offsets.push(node_ids.len() as u32);
        }

        (ids, key_ids, val_ids, kv_offsets, node_ids, node_offsets)
    }
}
/// A Reference to a [`Way`]
#[derive(Debug)]
pub struct WayRef<'a> {
    way: &'a Way,
    cached_tag_ids: &'a [u32],
    table: &'a [Cow<'static, [u8]>],
}
impl<'a> WayRef<'a> {
    /// Get ID
    #[inline]
    pub fn id(&self) -> i64 {
        self.way.id
    }
    /// Get Iterator over node_ids
    #[inline]
    pub fn node_ids(&self) -> impl Iterator<Item = i64> + '_ {
        let mut last_id = 0i64;
        self.way.refs.iter().map(move |delta| {
            last_id += *delta;
            last_id
        })
    }
    /// Get Iterator over (key, value) pairs
    #[inline]
    pub fn tags(&self) -> TagIter<'_> {
        TagIter {
            keys: &self.way.keys,
            vals: &self.way.vals,
            table: self.table,
            pos: 0,
            cached_tag_ids: self.cached_tag_ids,
            use_cache: TAG_KEYS_FILTER.get().is_some(),
        }
    }
}

// --------------------------- RELATION ---------------------------
// --------------------------- RELATION ---------------------------
// --------------------------- RELATION ---------------------------

/// A block of [`Relation`].
/// Use .iter() on it to iterate over relations.
#[derive(Debug)]
pub struct RelationBlock {
    pub(crate) relations: Arc<Vec<Relation>>,
    pub(crate) cached_tag_ids: Arc<Vec<u32>>,
    pub(crate) table: Arc<Vec<Cow<'static, [u8]>>>,
}
impl RelationBlock {
    /// Creates an iterator over [`RelationRef`]
    pub fn iter(&self) -> impl Iterator<Item = RelationRef<'_>> {
        self.relations.iter().map(move |relation| RelationRef {
            relation,
            cached_tag_ids: &self.cached_tag_ids,
            table: &self.table,
        })
    }
    /// Get the number of [`RelationRef`]
    pub fn len(&self) -> usize {
        self.relations.len()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_string_table(&self) -> Vec<String> {
        self.table
            .iter()
            .map(|cow| unsafe { std::str::from_utf8_unchecked(cow.as_ref()) }.to_owned())
            .collect()
    }
    /// Helper method for node bindings.
    #[cfg(feature = "node_bindings")]
    pub fn get_raw_data(
        &self,
    ) -> (
        Vec<i64>, // relation ids
        Vec<u32>, // key ids
        Vec<u32>, // val ids
        Vec<u32>, // kv offsets
        Vec<i64>, // member ids
        Vec<u8>,  // member types
        Vec<i32>, // member roles
        Vec<u32>, // member offsets
    ) {
        let len = self.relations.len();
        let mut ids = Vec::with_capacity(len);

        let mut key_ids = Vec::new();
        let mut val_ids = Vec::new();
        let mut kv_offsets = Vec::with_capacity(len + 1);
        kv_offsets.push(0);

        let mut member_ids = Vec::new();
        let mut member_types = Vec::new();
        let mut member_roles = Vec::new();
        let mut member_offsets = Vec::with_capacity(len + 1);
        member_offsets.push(0);

        let use_cache = TAG_KEYS_FILTER.get().is_some();

        for rel in self.relations.iter() {
            ids.push(rel.id);

            // append all tags for this node
            for (k, v) in rel.keys.iter().zip(rel.vals.iter()) {
                if !use_cache || self.cached_tag_ids.contains(k) {
                    key_ids.push(*k);
                    val_ids.push(*v);
                }
            }
            kv_offsets.push(key_ids.len() as u32);

            // member_ids are delta encoded
            let mut last_member_id = 0i64;
            for d in rel.memids.iter() {
                last_member_id += *d;
                member_ids.push(last_member_id);
            }
            member_types.extend(rel.types.iter().map(|t| *t as u8));
            member_roles.extend_from_slice(&rel.roles_sid);
            member_offsets.push(member_ids.len() as u32);
        }

        (
            ids,
            key_ids,
            val_ids,
            kv_offsets,
            member_ids,
            member_types,
            member_roles,
            member_offsets,
        )
    }
}

/// A Reference to a [`Relation`]
#[derive(Debug)]
pub struct RelationRef<'a> {
    relation: &'a Relation,
    cached_tag_ids: &'a [u32],
    table: &'a [Cow<'static, [u8]>],
}
impl<'a> RelationRef<'a> {
    /// Get ID
    #[inline]
    pub fn id(&self) -> i64 {
        self.relation.id
    }
    /// Get Iterator over [`RelationMember`]
    #[inline]
    pub fn members(&self) -> impl Iterator<Item = RelationMember<'_>> {
        RelationMemberIter {
            memids: &self.relation.memids,
            roles: &self.relation.roles_sid,
            types: &self.relation.types,
            table: &self.table,
            index: 0,
            prev_memid: 0,
        }
    }
    /// Get Iterator over (key, value) pairs
    #[inline]
    pub fn tags(&self) -> TagIter<'_> {
        TagIter {
            keys: &self.relation.keys,
            vals: &self.relation.vals,
            table: self.table,
            pos: 0,
            cached_tag_ids: self.cached_tag_ids,
            use_cache: TAG_KEYS_FILTER.get().is_some(),
        }
    }
}

// --------------------------- RELATION_MEMBER ---------------------------
// --------------------------- RELATION_MEMBER ---------------------------
// --------------------------- RELATION_MEMBER ---------------------------

/// An member of a [`Relation`]
#[derive(Debug)]
pub struct RelationMember<'a> {
    memid: i64,
    role: &'a str,
    member_type: MemberType,
}
impl<'a> RelationMember<'a> {
    /// Get ID
    #[inline]
    pub fn id(&self) -> i64 {
        self.memid
    }
    /// Get element type ([`Node`], [`Way`] or [`Relation`])
    #[inline]
    pub fn member_type(&self) -> MemberType {
        self.member_type
    }
    /// Get role
    #[inline]
    pub fn role(&self) -> &'a str {
        self.role
    }
}

struct RelationMemberIter<'a> {
    memids: &'a [i64],
    roles: &'a [i32],
    types: &'a [MemberType],
    table: &'a [Cow<'static, [u8]>],
    index: usize,
    prev_memid: i64,
}
impl<'a> Iterator for RelationMemberIter<'a> {
    type Item = RelationMember<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let delta = *self.memids.get(self.index)?;
        let role_sid = *self.roles.get(self.index)?;
        let member_type = *self.types.get(self.index)?;

        self.index += 1;
        self.prev_memid += delta;

        // scary scary i32 as usize conversion in unsafe block - but role_sid are uint32 in reality,
        // just a mistake when proto format was defined
        let role = unsafe { std::str::from_utf8_unchecked(&self.table[role_sid as usize]) };

        Some(RelationMember {
            memid: self.prev_memid,
            role,
            member_type,
        })
    }
}

// --------------------------- TAGS_ITER ---------------------------
// --------------------------- TAGS_ITER ---------------------------
// --------------------------- TAGS_ITER ---------------------------

/// An iterator that yields (key, value) tag pair
pub struct DenseNodeTagIter<'a> {
    slice: &'a [i32],
    table: &'a [Cow<'static, [u8]>],
    pos: usize,
    cached_tag_ids: &'a [u32],
    use_cache: bool,
}
impl<'a> Iterator for DenseNodeTagIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.slice.len() && self.slice[self.pos] != 0 {
            let k = self.slice[self.pos] as usize;
            let v = self.slice[self.pos + 1] as usize;
            self.pos += 2;
            if self.use_cache && !self.cached_tag_ids.contains(&(k as u32)) {
                continue;
            }
            return Some((
                unsafe { std::str::from_utf8_unchecked(&self.table[k]) },
                unsafe { std::str::from_utf8_unchecked(&self.table[v]) },
            ));
        }
        None
    }
}
impl<'a> DenseNodeTagIter<'a> {
    /// Get the number of tag pairs
    #[inline]
    pub fn len(mut self) -> usize {
        let mut count = 0;
        while self.next().is_some() {
            count += 1;
        }
        count
    }
    /// Check if all applied filter keys are present in the iterator.
    /// We assume the same key cannot appear more than once.
    /// Essentially the same as calling .len() == <FILTER_COUNT_APPLIED>
    #[inline]
    pub fn has_all_filter_keys(self) -> bool {
        Some(&self.len()) == TAG_KEYS_FILTER_COUNT.get()
    }
}

/// An iterator that yields (key, value) tag pair
pub struct TagIter<'a> {
    keys: &'a [u32],
    vals: &'a [u32],
    table: &'a [Cow<'static, [u8]>],
    pos: usize,
    cached_tag_ids: &'a [u32],
    use_cache: bool,
}
impl<'a> Iterator for TagIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.keys.len() {
            let k = self.keys[self.pos] as usize;
            let v = self.vals[self.pos] as usize;
            self.pos += 1;
            if self.use_cache && !self.cached_tag_ids.contains(&(k as u32)) {
                continue;
            }
            return Some((
                unsafe { std::str::from_utf8_unchecked(&self.table[k]) },
                unsafe { std::str::from_utf8_unchecked(&self.table[v]) },
            ));
        }
        None
    }
}
impl<'a> TagIter<'a> {
    /// Get the number of tag pairs
    #[inline]
    pub fn len(mut self) -> usize {
        let mut count = 0;
        while self.next().is_some() {
            count += 1;
        }
        count
    }
    /// Check if all applied filter keys are present in the iterator.
    /// We assume the same key cannot appear more than once.
    /// Essentially the same as calling .len() == <FILTER_COUNT_APPLIED>
    #[inline]
    pub fn has_all_filter_keys(self) -> bool {
        Some(&self.len()) == TAG_KEYS_FILTER_COUNT.get()
    }
}

/// An Iterator that yields [`ElementBlock`]
pub struct ElementBlockIter {
    pub(crate) rx: Receiver<ElementBlock>,
}

impl Iterator for ElementBlockIter {
    type Item = ElementBlock;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.recv().ok()
    }
}
