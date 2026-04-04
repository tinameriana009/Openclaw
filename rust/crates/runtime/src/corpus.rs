use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use crate::json::{JsonError, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusKind {
    Repo,
    Docs,
    Notes,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusBackend {
    Lexical,
    Hybrid,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusManifest {
    pub corpus_id: String,
    pub roots: Vec<String>,
    pub kind: CorpusKind,
    pub backend: CorpusBackend,
    pub document_count: u32,
    pub chunk_count: u32,
    pub estimated_bytes: u64,
    pub documents: Vec<CorpusDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusDocument {
    pub document_id: String,
    pub path: String,
    pub media_type: String,
    pub language: Option<String>,
    pub headings: Vec<String>,
    pub bytes: u64,
    pub modified_at_ms: Option<u64>,
    pub chunks: Vec<CorpusChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub ordinal: u32,
    pub start_offset: u32,
    pub end_offset: u32,
    pub text_preview: String,
    pub metadata: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalResult {
    pub query: String,
    pub backend: CorpusBackend,
    pub elapsed_ms: u64,
    pub hits: Vec<RetrievalHit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalHit {
    pub chunk_id: String,
    pub document_id: String,
    pub path: String,
    pub score: f64,
    pub reason: String,
    pub preview: String,
}

#[derive(Debug)]
pub enum CorpusError {
    Io(std::io::Error),
    Json(JsonError),
    Format(String),
}

impl Display for CorpusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Format(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CorpusError {}

impl From<std::io::Error> for CorpusError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<JsonError> for CorpusError {
    fn from(value: JsonError) -> Self {
        Self::Json(value)
    }
}

impl CorpusManifest {
    #[must_use]
    pub fn stable_document_id(path: &str) -> String {
        format!("doc:{}", sanitize_id_component(path))
    }

    #[must_use]
    pub fn stable_chunk_id(document_id: &str, ordinal: u32) -> String {
        format!("chunk:{}:{ordinal}", sanitize_id_component(document_id))
    }

    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "corpusId".to_string(),
                JsonValue::String(self.corpus_id.clone()),
            ),
            (
                "roots".to_string(),
                JsonValue::Array(
                    self.roots
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "kind".to_string(),
                JsonValue::String(self.kind.as_str().to_string()),
            ),
            (
                "backend".to_string(),
                JsonValue::String(self.backend.as_str().to_string()),
            ),
            (
                "documentCount".to_string(),
                JsonValue::Number(i64::from(self.document_count)),
            ),
            (
                "chunkCount".to_string(),
                JsonValue::Number(i64::from(self.chunk_count)),
            ),
            (
                "estimatedBytes".to_string(),
                JsonValue::Number(i64::try_from(self.estimated_bytes).unwrap_or(i64::MAX)),
            ),
            (
                "documents".to_string(),
                JsonValue::Array(self.documents.iter().map(CorpusDocument::to_json_value).collect()),
            ),
        ]))
    }

    pub fn from_json_value(value: &JsonValue) -> Result<Self, CorpusError> {
        let object = value
            .as_object()
            .ok_or_else(|| CorpusError::Format("corpus manifest must be an object".to_string()))?;
        Ok(Self {
            corpus_id: expect_string(object, "corpusId")?.to_string(),
            roots: expect_string_array(object, "roots")?,
            kind: CorpusKind::from_str(expect_string(object, "kind")?)?,
            backend: CorpusBackend::from_str(expect_string(object, "backend")?)?,
            document_count: u32::try_from(expect_u64(object, "documentCount")?).map_err(|_| {
                CorpusError::Format("documentCount is out of range for u32".to_string())
            })?,
            chunk_count: u32::try_from(expect_u64(object, "chunkCount")?).map_err(|_| {
                CorpusError::Format("chunkCount is out of range for u32".to_string())
            })?,
            estimated_bytes: expect_u64(object, "estimatedBytes")?,
            documents: expect_documents(object, "documents")?,
        })
    }

    #[must_use]
    pub fn render_json(&self) -> String {
        self.to_json_value().render()
    }

    pub fn write_to_path(&self, path: &Path) -> Result<(), CorpusError> {
        fs::write(path, self.render_json()).map_err(CorpusError::Io)
    }

    pub fn read_from_path(path: &Path) -> Result<Self, CorpusError> {
        let raw = fs::read_to_string(path)?;
        let value = JsonValue::parse(&raw)?;
        Self::from_json_value(&value)
    }
}

impl CorpusKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Repo => "repo",
            Self::Docs => "docs",
            Self::Notes => "notes",
            Self::Mixed => "mixed",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, CorpusError> {
        match value {
            "repo" => Ok(Self::Repo),
            "docs" => Ok(Self::Docs),
            "notes" => Ok(Self::Notes),
            "mixed" => Ok(Self::Mixed),
            other => Err(CorpusError::Format(format!("unsupported corpus kind {other}"))),
        }
    }
}

impl CorpusBackend {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lexical => "lexical",
            Self::Hybrid => "hybrid",
            Self::Semantic => "semantic",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, CorpusError> {
        match value {
            "lexical" => Ok(Self::Lexical),
            "hybrid" => Ok(Self::Hybrid),
            "semantic" => Ok(Self::Semantic),
            other => Err(CorpusError::Format(format!(
                "unsupported corpus backend {other}"
            ))),
        }
    }
}

impl CorpusDocument {
    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "documentId".to_string(),
                JsonValue::String(self.document_id.clone()),
            ),
            ("path".to_string(), JsonValue::String(self.path.clone())),
            (
                "mediaType".to_string(),
                JsonValue::String(self.media_type.clone()),
            ),
            (
                "language".to_string(),
                self.language
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "headings".to_string(),
                JsonValue::Array(
                    self.headings
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "bytes".to_string(),
                JsonValue::Number(i64::try_from(self.bytes).unwrap_or(i64::MAX)),
            ),
            (
                "modifiedAtMs".to_string(),
                self.modified_at_ms
                    .map(|value| JsonValue::Number(i64::try_from(value).unwrap_or(i64::MAX)))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "chunks".to_string(),
                JsonValue::Array(self.chunks.iter().map(CorpusChunk::to_json_value).collect()),
            ),
        ]))
    }

    pub fn from_json_value(value: &JsonValue) -> Result<Self, CorpusError> {
        let object = value
            .as_object()
            .ok_or_else(|| CorpusError::Format("corpus document must be an object".to_string()))?;
        Ok(Self {
            document_id: expect_string(object, "documentId")?.to_string(),
            path: expect_string(object, "path")?.to_string(),
            media_type: expect_string(object, "mediaType")?.to_string(),
            language: optional_string(object, "language")?.map(ToOwned::to_owned),
            headings: expect_string_array(object, "headings")?,
            bytes: expect_u64(object, "bytes")?,
            modified_at_ms: optional_u64(object, "modifiedAtMs")?,
            chunks: expect_chunks(object, "chunks")?,
        })
    }
}

impl CorpusChunk {
    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "chunkId".to_string(),
                JsonValue::String(self.chunk_id.clone()),
            ),
            (
                "documentId".to_string(),
                JsonValue::String(self.document_id.clone()),
            ),
            (
                "ordinal".to_string(),
                JsonValue::Number(i64::from(self.ordinal)),
            ),
            (
                "startOffset".to_string(),
                JsonValue::Number(i64::from(self.start_offset)),
            ),
            (
                "endOffset".to_string(),
                JsonValue::Number(i64::from(self.end_offset)),
            ),
            (
                "textPreview".to_string(),
                JsonValue::String(self.text_preview.clone()),
            ),
            (
                "metadata".to_string(),
                JsonValue::Object(self.metadata.clone()),
            ),
        ]))
    }

    pub fn from_json_value(value: &JsonValue) -> Result<Self, CorpusError> {
        let object = value
            .as_object()
            .ok_or_else(|| CorpusError::Format("corpus chunk must be an object".to_string()))?;
        Ok(Self {
            chunk_id: expect_string(object, "chunkId")?.to_string(),
            document_id: expect_string(object, "documentId")?.to_string(),
            ordinal: u32::try_from(expect_u64(object, "ordinal")?).map_err(|_| {
                CorpusError::Format("ordinal is out of range for u32".to_string())
            })?,
            start_offset: u32::try_from(expect_u64(object, "startOffset")?).map_err(|_| {
                CorpusError::Format("startOffset is out of range for u32".to_string())
            })?,
            end_offset: u32::try_from(expect_u64(object, "endOffset")?).map_err(|_| {
                CorpusError::Format("endOffset is out of range for u32".to_string())
            })?,
            text_preview: expect_string(object, "textPreview")?.to_string(),
            metadata: expect_object(object, "metadata")?.clone(),
        })
    }
}

fn sanitize_id_component(value: &str) -> String {
    value.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}

fn expect_object<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, CorpusError> {
    object
        .get(key)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| CorpusError::Format(format!("missing object field {key}")))
}

fn expect_string<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, CorpusError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CorpusError::Format(format!("missing string field {key}")))
}

fn optional_string<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<&'a str>, CorpusError> {
    match object.get(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(CorpusError::Format(format!("field {key} must be a string or null"))),
    }
}

fn expect_u64(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u64, CorpusError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| CorpusError::Format(format!("missing numeric field {key}")))?;
    u64::try_from(value)
        .map_err(|_| CorpusError::Format(format!("numeric field {key} is out of range")))
}

fn optional_u64(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<Option<u64>, CorpusError> {
    match object.get(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::Number(value)) => u64::try_from(*value)
            .map(Some)
            .map_err(|_| CorpusError::Format(format!("numeric field {key} is out of range"))),
        Some(_) => Err(CorpusError::Format(format!("field {key} must be a number or null"))),
    }
}

fn expect_string_array(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<String>, CorpusError> {
    let values = object
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| CorpusError::Format(format!("missing array field {key}")))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| CorpusError::Format(format!("field {key} must contain strings")))
        })
        .collect()
}

fn expect_documents(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<CorpusDocument>, CorpusError> {
    let values = object
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| CorpusError::Format(format!("missing array field {key}")))?;
    values.iter().map(CorpusDocument::from_json_value).collect()
}

fn expect_chunks(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<CorpusChunk>, CorpusError> {
    let values = object
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| CorpusError::Format(format!("missing array field {key}")))?;
    values.iter().map(CorpusChunk::from_json_value).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        CorpusBackend, CorpusChunk, CorpusDocument, CorpusKind, CorpusManifest, RetrievalHit,
        RetrievalResult,
    };
    use crate::json::JsonValue;
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_manifest_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("corpus-manifest-{nanos}.json"))
    }

    fn sample_manifest() -> CorpusManifest {
        let document_id = CorpusManifest::stable_document_id("docs/guide.md");
        let chunk_id = CorpusManifest::stable_chunk_id(&document_id, 0);
        CorpusManifest {
            corpus_id: "corpus-main".to_string(),
            roots: vec![".".to_string(), "docs".to_string()],
            kind: CorpusKind::Mixed,
            backend: CorpusBackend::Lexical,
            document_count: 1,
            chunk_count: 1,
            estimated_bytes: 1234,
            documents: vec![CorpusDocument {
                document_id: document_id.clone(),
                path: "docs/guide.md".to_string(),
                media_type: "text/markdown".to_string(),
                language: Some("markdown".to_string()),
                headings: vec!["Intro".to_string()],
                bytes: 1234,
                modified_at_ms: Some(1_700_000_000_000),
                chunks: vec![CorpusChunk {
                    chunk_id,
                    document_id,
                    ordinal: 0,
                    start_offset: 0,
                    end_offset: 200,
                    text_preview: "# Intro\nHello world".to_string(),
                    metadata: BTreeMap::from([(
                        "heading".to_string(),
                        JsonValue::String("Intro".to_string()),
                    )]),
                }],
            }],
        }
    }

    #[test]
    fn stable_document_ids_are_path_based_and_repeatable() {
        let first = CorpusManifest::stable_document_id("src/main.rs");
        let second = CorpusManifest::stable_document_id("src/main.rs");
        let third = CorpusManifest::stable_document_id("src/lib.rs");

        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn stable_chunk_ids_are_document_and_ordinal_based() {
        let document_id = CorpusManifest::stable_document_id("docs/spec.md");
        let first = CorpusManifest::stable_chunk_id(&document_id, 0);
        let second = CorpusManifest::stable_chunk_id(&document_id, 0);
        let third = CorpusManifest::stable_chunk_id(&document_id, 1);

        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn manifest_round_trips_via_json_value_and_file() {
        let manifest = sample_manifest();
        let parsed = CorpusManifest::from_json_value(&manifest.to_json_value())
            .expect("manifest should parse after round trip");
        assert_eq!(parsed, manifest);

        let path = temp_manifest_path();
        manifest.write_to_path(&path).expect("manifest should write");
        let restored = CorpusManifest::read_from_path(&path).expect("manifest should read");
        assert_eq!(restored, manifest);
        fs::remove_file(path).expect("temp manifest should be removable");
    }

    #[test]
    fn retrieval_types_hold_scored_results() {
        let hit = RetrievalHit {
            chunk_id: "chunk:doc_docs_guide_md:0".to_string(),
            document_id: "doc:docs_guide_md".to_string(),
            path: "docs/guide.md".to_string(),
            score: 0.92,
            reason: "heading match".to_string(),
            preview: "# Intro\nHello world".to_string(),
        };
        let result = RetrievalResult {
            query: "intro".to_string(),
            backend: CorpusBackend::Lexical,
            elapsed_ms: 12,
            hits: vec![hit.clone()],
        };

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0], hit);
        assert_eq!(result.backend, CorpusBackend::Lexical);
    }
}
