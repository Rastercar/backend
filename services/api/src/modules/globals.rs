use tokio::sync::RwLock;

use super::tracking::cache::TrackerIdCache;
use std::sync::{Arc, OnceLock};

pub static TRACKER_ID_CACHE: OnceLock<Arc<RwLock<TrackerIdCache>>> = OnceLock::new();
