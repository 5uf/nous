/// Sidecar metadata stored alongside every object.
///
/// Serialised to TOML by `nous-store`; the fields are deliberately plain
/// strings / primitives so the format stays human-readable and stable.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    /// `ObjectId` Display string, e.g. `"b3:<64hex>"`.
    pub id: String,

    /// Short algorithm tag, e.g. `"b3"`.
    pub algo: String,

    /// Raw byte length of the stored object.
    pub size: u64,

    /// Creation timestamp as Unix seconds (UTC).
    pub created: i64,

    /// Optional MIME type hint, e.g. `"application/octet-stream"`.
    pub content_type: Option<String>,
}
