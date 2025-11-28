use fast_osmpbf::ElementBlock;
use napi::bindgen_prelude::*;
use napi_derive::napi;

pub fn construct_js_block(block: ElementBlock) -> JsElementBlock {
    let js_block = match block {
        ElementBlock::DenseNodeBlock(block) => {
            let (ids, latitudes, longitudes, tag_key_ids, tag_val_ids, tag_kv_offsets) =
                block.get_raw_data();
            JsElementBlock {
                element_type: String::from("Node"),
                ids: ids.into(),
                dense_tags: Some((
                    tag_key_ids.into(),
                    tag_val_ids.into(),
                    tag_kv_offsets.into(),
                )),
                tags: None,
                node_ids: None,
                latitudes: Some(latitudes.into()),
                longitudes: Some(longitudes.into()),
                relation_members: None,
                string_table: block.get_string_table(),
            }
        }
        ElementBlock::NodeBlock(block) => {
            let (ids, latitudes, longitudes, tag_key_ids, tag_val_ids, tag_kv_offsets) =
                block.get_raw_data();
            JsElementBlock {
                element_type: String::from("Node"),
                ids: ids.into(),
                dense_tags: None,
                tags: Some((
                    tag_key_ids.into(),
                    tag_val_ids.into(),
                    tag_kv_offsets.into(),
                )),
                node_ids: None,
                latitudes: Some(latitudes.into()),
                longitudes: Some(longitudes.into()),
                relation_members: None,
                string_table: block.get_string_table(),
            }
        }
        ElementBlock::WayBlock(block) => {
            let (ids, tag_key_ids, tag_val_ids, tag_kv_offsets, node_ids, node_id_offsets) =
                block.get_raw_data();
            JsElementBlock {
                element_type: String::from("Way"),
                ids: ids.into(),
                dense_tags: None,
                tags: Some((
                    tag_key_ids.into(),
                    tag_val_ids.into(),
                    tag_kv_offsets.into(),
                )),
                node_ids: Some((node_ids.into(), node_id_offsets.into())),
                latitudes: None,
                longitudes: None,
                relation_members: None,
                string_table: block.get_string_table(),
            }
        }
        ElementBlock::RelationBlock(block) => {
            let (
                ids,
                tag_key_ids,
                tag_val_ids,
                tag_kv_offsets,
                member_ids,
                member_types,
                member_roles,
                member_offsets,
            ) = block.get_raw_data();
            JsElementBlock {
                element_type: String::from("Relation"),
                ids: ids.into(),
                dense_tags: None,
                tags: Some((
                    tag_key_ids.into(),
                    tag_val_ids.into(),
                    tag_kv_offsets.into(),
                )),
                node_ids: None,
                latitudes: None,
                longitudes: None,
                relation_members: Some((
                    member_ids.into(),
                    member_types.into(),
                    member_roles.into(),
                    member_offsets.into(),
                )),
                string_table: block.get_string_table(),
            }
        }
    };

    js_block
}

#[napi(object)]
pub struct JsElementBlock {
    pub ids: BigInt64Array,
    pub element_type: String,
    pub node_ids: Option<(BigInt64Array, Uint32Array)>,
    pub latitudes: Option<Float64Array>,
    pub longitudes: Option<Float64Array>,
    pub relation_members: Option<(BigInt64Array, Uint8Array, Int32Array, Uint32Array)>,
    pub dense_tags: Option<(Uint32Array, Uint32Array, Uint32Array)>,
    pub tags: Option<(Uint32Array, Uint32Array, Uint32Array)>,
    pub string_table: Vec<String>,
}
