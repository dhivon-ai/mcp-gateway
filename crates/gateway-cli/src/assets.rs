//! Immutable compiled local website assets; no runtime filesystem reads.
use crate::management::{EmbeddedAsset, ManagementAssets};
use std::sync::Arc;
include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
pub fn bundled() -> Option<ManagementAssets> {
    INDEX.map(|html| ManagementAssets {
        index_html: Arc::from(html),
        assets: ASSETS
            .iter()
            .filter_map(|(path, mime, bytes)| {
                path.strip_prefix("assets/").map(|name| {
                    (
                        name.to_owned(),
                        EmbeddedAsset {
                            content_type: mime,
                            bytes: Arc::from(*bytes),
                        },
                    )
                })
            })
            .collect(),
    })
}
