use crate::element::{JsElementBlock, construct_js_block};
use fast_osmpbf::{ElementFilter, prelude::*};
use napi_derive::napi;
use std::sync::Arc;
use std::thread;
use tokio::sync::Mutex;
pub mod element;

const MAX_BLOCK_THROUGHPUT: usize = 64;

#[napi]
pub struct OsmReader {
    path: String,
}

#[napi(object)]
pub struct JsElementFilter {
    pub nodes: bool,
    pub ways: bool,
    pub relations: bool,
}

#[napi]
impl OsmReader {
    #[napi(constructor)]
    pub fn new(path: String) -> Self {
        Self { path }
    }
    #[napi]
    pub fn stream_blocks(
        &self,
        element_filter: Option<JsElementFilter>,
        tag_filter: Option<Vec<String>>,
    ) -> AsyncBlockIterator {
        let (tx, rx) = tokio::sync::mpsc::channel::<JsElementBlock>(MAX_BLOCK_THROUGHPUT);

        let path = self.path.clone();
        thread::spawn(move || {
            let reader = fast_osmpbf::OsmReader::from_path(&path).expect("Failed to open file");
            if let Some(filter) = tag_filter {
                reader
                    .apply_tag_filter(
                        &(filter.iter().map(|tag| tag.as_str()).collect::<Vec<&str>>()),
                    )
                    .expect("Invalid tag filter");
            }
            if let Some(filter) = element_filter {
                reader
                    .apply_element_filter(ElementFilter {
                        nodes: filter.nodes,
                        ways: filter.ways,
                        relations: filter.relations,
                    })
                    .expect("Invalid tag filter");
            }

            reader.par_blocks().for_each(|block| {
                let block = construct_js_block(block);
                if tx.blocking_send(block).is_err() {
                    return;
                }
            })
        });

        AsyncBlockIterator {
            rx: Arc::new(Mutex::new(rx)),
        }
    }
}

#[napi]
pub struct AsyncBlockIterator {
    rx: Arc<Mutex<tokio::sync::mpsc::Receiver<JsElementBlock>>>,
}

#[napi]
impl AsyncBlockIterator {
    #[napi]
    pub async fn next(&self) -> Option<JsElementBlock> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }
}
