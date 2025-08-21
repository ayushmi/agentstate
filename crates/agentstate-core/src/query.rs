use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagFilter(pub BTreeMap<String, String>);

// MVP: materialized JSONPath equality filters on indexed paths
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonPathFilter {
    // path -> exact value match
    pub equals: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorQuery {
    pub field: String,
    pub top_k: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryRequest {
    pub tag_filter: Option<TagFilter>,
    pub jsonpath: Option<JsonPathFilter>,
    pub vector: Option<VectorQuery>,
    pub limit: Option<usize>,
    // Projection for body fields: e.g., ["text","status"]
    pub fields: Option<Vec<String>>,
}
