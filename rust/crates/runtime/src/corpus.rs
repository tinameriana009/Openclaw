use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::json::{JsonError, JsonValue};

const DEFAULT_MAX_FILE_BYTES: u64 = 512 * 1024;
const DEFAULT_CHUNK_BYTES: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CorpusKind {
    Repo,
    Docs,
    Notes,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CorpusBackend {
    Lexical,
    Hybrid,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusSkipSummary {
    pub skipped_directories: u32,
    pub unsupported_files: u32,
    pub oversized_files: u32,
    pub binary_files: u32,
    pub unreadable_files: u32,
    pub empty_files: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusRootSummary {
    pub root: String,
    pub document_count: u32,
    pub chunk_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusManifest {
    pub corpus_id: String,
    pub roots: Vec<String>,
    pub kind: CorpusKind,
    pub backend: CorpusBackend,
    pub document_count: u32,
    pub chunk_count: u32,
    pub estimated_bytes: u64,
    pub root_summaries: Vec<CorpusRootSummary>,
    pub skip_summary: CorpusSkipSummary,
    pub documents: Vec<CorpusDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub ordinal: u32,
    pub start_offset: u32,
    pub end_offset: u32,
    pub text_preview: String,
    pub metadata: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusInspectResult {
    pub corpus_id: String,
    pub kind: CorpusKind,
    pub backend: CorpusBackend,
    pub roots: Vec<String>,
    pub document_count: u32,
    pub chunk_count: u32,
    pub estimated_bytes: u64,
    pub root_summaries: Vec<CorpusRootSummary>,
    pub skip_summary: CorpusSkipSummary,
    pub documents: Vec<CorpusDocumentSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusDocumentSummary {
    pub document_id: String,
    pub path: String,
    pub language: Option<String>,
    pub headings: Vec<String>,
    pub chunk_count: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusSlice {
    pub corpus_id: String,
    pub chunk_id: String,
    pub document_id: String,
    pub path: String,
    pub ordinal: u32,
    pub start_offset: u32,
    pub end_offset: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RetrievalResult {
    pub corpus_id: String,
    pub query: String,
    pub backend: CorpusBackend,
    pub elapsed_ms: u64,
    pub path_filter: Option<String>,
    pub total_candidate_chunks: u32,
    pub total_matching_chunks: u32,
    pub hits: Vec<RetrievalHit>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RetrievalHit {
    pub chunk_id: String,
    pub document_id: String,
    pub path: String,
    pub score: f64,
    pub reason: String,
    pub matched_terms: Vec<String>,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusAttachOptions {
    pub corpus_id: Option<String>,
    pub chunk_bytes: usize,
    pub max_file_bytes: u64,
}

impl Default for CorpusAttachOptions {
    fn default() -> Self {
        Self {
            corpus_id: None,
            chunk_bytes: DEFAULT_CHUNK_BYTES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
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

impl CorpusSkipSummary {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            skipped_directories: 0,
            unsupported_files: 0,
            oversized_files: 0,
            binary_files: 0,
            unreadable_files: 0,
            empty_files: 0,
        }
    }

    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "skippedDirectories".to_string(),
                JsonValue::Number(i64::from(self.skipped_directories)),
            ),
            (
                "unsupportedFiles".to_string(),
                JsonValue::Number(i64::from(self.unsupported_files)),
            ),
            (
                "oversizedFiles".to_string(),
                JsonValue::Number(i64::from(self.oversized_files)),
            ),
            (
                "binaryFiles".to_string(),
                JsonValue::Number(i64::from(self.binary_files)),
            ),
            (
                "unreadableFiles".to_string(),
                JsonValue::Number(i64::from(self.unreadable_files)),
            ),
            (
                "emptyFiles".to_string(),
                JsonValue::Number(i64::from(self.empty_files)),
            ),
        ]))
    }

    fn from_json_value(value: Option<&JsonValue>) -> Result<Self, CorpusError> {
        let Some(value) = value else {
            return Ok(Self::empty());
        };
        let object = value.as_object().ok_or_else(|| {
            CorpusError::Format("skipSummary must be an object when present".to_string())
        })?;
        Ok(Self {
            skipped_directories: optional_u32(object, "skippedDirectories")?.unwrap_or(0),
            unsupported_files: optional_u32(object, "unsupportedFiles")?.unwrap_or(0),
            oversized_files: optional_u32(object, "oversizedFiles")?.unwrap_or(0),
            binary_files: optional_u32(object, "binaryFiles")?.unwrap_or(0),
            unreadable_files: optional_u32(object, "unreadableFiles")?.unwrap_or(0),
            empty_files: optional_u32(object, "emptyFiles")?.unwrap_or(0),
        })
    }
}

impl CorpusRootSummary {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            ("root".to_string(), JsonValue::String(self.root.clone())),
            (
                "documentCount".to_string(),
                JsonValue::Number(i64::from(self.document_count)),
            ),
            (
                "chunkCount".to_string(),
                JsonValue::Number(i64::from(self.chunk_count)),
            ),
        ]))
    }

    fn from_json_value(value: &JsonValue) -> Result<Self, CorpusError> {
        let object = value.as_object().ok_or_else(|| {
            CorpusError::Format("root summary entries must be objects".to_string())
        })?;
        Ok(Self {
            root: expect_string(object, "root")?.to_string(),
            document_count: u32::try_from(expect_u64(object, "documentCount")?).map_err(|_| {
                CorpusError::Format("documentCount is out of range for u32".to_string())
            })?,
            chunk_count: u32::try_from(expect_u64(object, "chunkCount")?).map_err(|_| {
                CorpusError::Format("chunkCount is out of range for u32".to_string())
            })?,
        })
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
                JsonValue::Array(self.roots.iter().cloned().map(JsonValue::String).collect()),
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
                "rootSummaries".to_string(),
                JsonValue::Array(
                    self.root_summaries
                        .iter()
                        .map(CorpusRootSummary::to_json_value)
                        .collect(),
                ),
            ),
            ("skipSummary".to_string(), self.skip_summary.to_json_value()),
            (
                "documents".to_string(),
                JsonValue::Array(
                    self.documents
                        .iter()
                        .map(CorpusDocument::to_json_value)
                        .collect(),
                ),
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
            root_summaries: optional_root_summaries(object, "rootSummaries")?,
            skip_summary: CorpusSkipSummary::from_json_value(object.get("skipSummary"))?,
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
            other => Err(CorpusError::Format(format!(
                "unsupported corpus kind {other}"
            ))),
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
            ordinal: u32::try_from(expect_u64(object, "ordinal")?)
                .map_err(|_| CorpusError::Format("ordinal is out of range for u32".to_string()))?,
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

pub fn default_corpus_store_dir(cwd: &Path) -> PathBuf {
    cwd.join(".claw").join("corpora")
}

pub fn attach_corpus(
    cwd: &Path,
    roots: &[PathBuf],
    options: CorpusAttachOptions,
) -> Result<CorpusManifest, CorpusError> {
    if roots.is_empty() {
        return Err(CorpusError::Format(
            "at least one corpus root is required".to_string(),
        ));
    }
    let mut documents = Vec::new();
    let mut estimated_bytes = 0_u64;
    let mut root_summaries = Vec::new();
    let mut skip_summary = CorpusSkipSummary::empty();

    for root in roots {
        let canonical_root = fs::canonicalize(root).map_err(|error| {
            CorpusError::Format(format!(
                "failed to resolve corpus root {}: {error}",
                root.display()
            ))
        })?;
        let before_docs = documents.len();
        let before_chunks: usize = documents
            .iter()
            .map(|doc: &CorpusDocument| doc.chunks.len())
            .sum();
        collect_documents(
            &canonical_root,
            &canonical_root,
            options.chunk_bytes.max(256),
            options.max_file_bytes,
            &mut documents,
            &mut estimated_bytes,
            &mut skip_summary,
        )?;
        let after_chunks: usize = documents.iter().map(|doc| doc.chunks.len()).sum();
        root_summaries.push(CorpusRootSummary {
            root: root.display().to_string(),
            document_count: u32::try_from(documents.len().saturating_sub(before_docs))
                .unwrap_or(u32::MAX),
            chunk_count: u32::try_from(after_chunks.saturating_sub(before_chunks))
                .unwrap_or(u32::MAX),
        });
    }

    let corpus_id = options
        .corpus_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| stable_corpus_id(roots));
    let manifest = CorpusManifest {
        corpus_id: corpus_id.clone(),
        roots: roots
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        kind: infer_corpus_kind(&documents),
        backend: CorpusBackend::Lexical,
        document_count: u32::try_from(documents.len()).unwrap_or(u32::MAX),
        chunk_count: u32::try_from(documents.iter().map(|doc| doc.chunks.len()).sum::<usize>())
            .unwrap_or(u32::MAX),
        estimated_bytes,
        root_summaries,
        skip_summary,
        documents,
    };

    let store_dir = default_corpus_store_dir(cwd);
    fs::create_dir_all(&store_dir)?;
    manifest.write_to_path(&store_dir.join(format!("{}.json", manifest.corpus_id)))?;
    Ok(manifest)
}

pub fn list_corpora(cwd: &Path) -> Result<Vec<CorpusManifest>, CorpusError> {
    let store_dir = default_corpus_store_dir(cwd);
    let mut corpora = Vec::new();
    let entries = match fs::read_dir(&store_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(CorpusError::Io(error)),
    };
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        corpora.push(CorpusManifest::read_from_path(&path)?);
    }
    corpora.sort_by(|left, right| left.corpus_id.cmp(&right.corpus_id));
    Ok(corpora)
}

pub fn load_corpus(cwd: &Path, corpus_id: &str) -> Result<CorpusManifest, CorpusError> {
    CorpusManifest::read_from_path(&default_corpus_store_dir(cwd).join(format!("{corpus_id}.json")))
}

pub fn inspect_corpus(cwd: &Path, corpus_id: &str) -> Result<CorpusInspectResult, CorpusError> {
    let manifest = load_corpus(cwd, corpus_id)?;
    Ok(CorpusInspectResult {
        corpus_id: manifest.corpus_id,
        kind: manifest.kind,
        backend: manifest.backend,
        roots: manifest.roots,
        document_count: manifest.document_count,
        chunk_count: manifest.chunk_count,
        estimated_bytes: manifest.estimated_bytes,
        root_summaries: manifest.root_summaries,
        skip_summary: manifest.skip_summary,
        documents: manifest
            .documents
            .into_iter()
            .map(|doc| CorpusDocumentSummary {
                chunk_count: u32::try_from(doc.chunks.len()).unwrap_or(u32::MAX),
                document_id: doc.document_id,
                path: doc.path,
                language: doc.language,
                headings: doc.headings,
                bytes: doc.bytes,
            })
            .collect(),
    })
}

pub fn slice_corpus(
    cwd: &Path,
    corpus_id: &str,
    chunk_id: Option<&str>,
    path: Option<&str>,
    ordinal: Option<u32>,
) -> Result<CorpusSlice, CorpusError> {
    let manifest = load_corpus(cwd, corpus_id)?;
    for doc in manifest.documents {
        let path_match = path.map_or(true, |candidate| candidate == doc.path);
        if !path_match {
            continue;
        }
        for chunk in &doc.chunks {
            let chunk_match = chunk_id.map_or(true, |candidate| candidate == chunk.chunk_id);
            let ordinal_match = ordinal.map_or(true, |candidate| candidate == chunk.ordinal);
            if chunk_match && ordinal_match {
                return Ok(CorpusSlice {
                    corpus_id: manifest.corpus_id,
                    chunk_id: chunk.chunk_id.clone(),
                    document_id: doc.document_id.clone(),
                    path: doc.path.clone(),
                    ordinal: chunk.ordinal,
                    start_offset: chunk.start_offset,
                    end_offset: chunk.end_offset,
                    text: chunk
                        .metadata
                        .get("text")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
    }
    Err(CorpusError::Format(
        "matching corpus slice not found".to_string(),
    ))
}

pub fn search_corpus(
    cwd: &Path,
    corpus_id: &str,
    query: &str,
    top_k: usize,
    path_filter: Option<&str>,
) -> Result<RetrievalResult, CorpusError> {
    let started = std::time::Instant::now();
    let manifest = load_corpus(cwd, corpus_id)?;
    let mut result = search_corpus_manifest(&manifest, query, top_k, path_filter);
    result.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(result)
}

#[must_use]
pub fn search_corpus_manifest(
    manifest: &CorpusManifest,
    query: &str,
    top_k: usize,
    path_filter: Option<&str>,
) -> RetrievalResult {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return RetrievalResult {
            corpus_id: manifest.corpus_id.clone(),
            query: query.to_string(),
            backend: manifest.backend,
            elapsed_ms: 0,
            path_filter: path_filter.map(ToOwned::to_owned),
            total_candidate_chunks: 0,
            total_matching_chunks: 0,
            hits: Vec::new(),
        };
    }

    let normalized_query = normalize_for_match(query);
    let filename_query = filename_like_query(&tokens);
    let mut hits = Vec::new();
    let mut total_candidate_chunks = 0_u32;
    let mut total_matching_chunks = 0_u32;
    for doc in &manifest.documents {
        if let Some(filter) = path_filter {
            if !doc.path.contains(filter) {
                continue;
            }
        }
        let path_lower = doc.path.to_ascii_lowercase();
        let heading_text = doc.headings.join(" ").to_ascii_lowercase();
        let normalized_path = normalize_for_match(&doc.path);
        let normalized_headings = normalize_for_match(&doc.headings.join(" "));
        let basename_lower = Path::new(&doc.path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let normalized_basename = normalize_for_match(&basename_lower).replace(' ', "");
        for chunk in &doc.chunks {
            total_candidate_chunks = total_candidate_chunks.saturating_add(1);
            let body = chunk
                .metadata
                .get("text")
                .and_then(JsonValue::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            let preview_lower = chunk.text_preview.to_ascii_lowercase();
            let normalized_body = normalize_for_match(&body);
            let mut score = 0.0_f64;
            let mut reasons = Vec::new();
            let mut matched_terms = Vec::new();
            let mut matched_tokens = 0usize;
            let mut in_order = true;
            let mut cursor = 0usize;
            for token in &tokens {
                let body_hits = count_occurrences(&body, token);
                let preview_hits = count_occurrences(&preview_lower, token);
                let path_hit = path_lower.contains(token);
                let heading_hit = heading_text.contains(token);
                if body_hits > 0 || preview_hits > 0 || path_hit || heading_hit {
                    matched_tokens += 1;
                    matched_terms.push(token.clone());
                }
                if body_hits > 0 {
                    score += body_hits as f64 * 2.0;
                    reasons.push(format!("content:{token}x{body_hits}"));
                }
                if preview_hits > body_hits {
                    let preview_only_hits = preview_hits - body_hits;
                    score += preview_only_hits as f64 * 0.75;
                    reasons.push(format!("preview:{token}x{preview_only_hits}"));
                }
                if path_hit {
                    score += 3.0;
                    reasons.push(format!("path:{token}"));
                }
                if heading_hit {
                    score += 2.5;
                    reasons.push(format!("heading:{token}"));
                }
                if let Some(found) =
                    normalized_body[cursor.min(normalized_body.len())..].find(token)
                {
                    cursor = cursor.saturating_add(found + token.len());
                } else {
                    in_order = false;
                }
            }
            if matched_tokens == 0 {
                continue;
            }
            if matched_tokens == tokens.len() {
                score += 4.0;
                reasons.push(format!("coverage:{matched_tokens}/{}", tokens.len()));
            } else {
                score += matched_tokens as f64 / tokens.len() as f64;
                reasons.push(format!("coverage:{matched_tokens}/{}", tokens.len()));
            }
            if tokens.len() > 1 && in_order {
                score += 2.0;
                reasons.push("ordered-terms".to_string());
            }
            if !normalized_query.is_empty() {
                if normalized_body.contains(&normalized_query) {
                    score += 6.0;
                    reasons.push("phrase:body".to_string());
                }
                if normalized_headings.contains(&normalized_query) {
                    score += 4.0;
                    reasons.push("phrase:heading".to_string());
                }
                if normalized_path.contains(&normalized_query) {
                    score += 5.0;
                    reasons.push("phrase:path".to_string());
                }
            }
            if let Some(filename_query) = filename_query.as_deref() {
                if basename_lower.contains(filename_query)
                    || normalized_basename.contains(filename_query)
                {
                    score += 3.5;
                    reasons.push("filename-match".to_string());
                }
            }
            if let Some(span) = minimum_term_span(&normalized_body, &tokens) {
                if span <= 48 {
                    score += 3.0;
                    reasons.push(format!("tight-span:{span}"));
                } else if span <= 96 {
                    score += 1.5;
                    reasons.push(format!("span:{span}"));
                }
            }
            total_matching_chunks = total_matching_chunks.saturating_add(1);
            hits.push(RetrievalHit {
                chunk_id: chunk.chunk_id.clone(),
                document_id: doc.document_id.clone(),
                path: doc.path.clone(),
                score,
                reason: reasons.join(", "),
                matched_terms,
                preview: chunk.text_preview.clone(),
            });
        }
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.chunk_id.cmp(&right.chunk_id))
    });
    hits.truncate(top_k.max(1));

    RetrievalResult {
        corpus_id: manifest.corpus_id.clone(),
        query: query.to_string(),
        backend: manifest.backend,
        elapsed_ms: 0,
        path_filter: path_filter.map(ToOwned::to_owned),
        total_candidate_chunks,
        total_matching_chunks,
        hits,
    }
}

fn stable_corpus_id(roots: &[PathBuf]) -> String {
    let joined = roots
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("-");
    format!("corpus-{}", sanitize_id_component(&joined))
}

fn collect_documents(
    root: &Path,
    current: &Path,
    chunk_bytes: usize,
    max_file_bytes: u64,
    documents: &mut Vec<CorpusDocument>,
    estimated_bytes: &mut u64,
    skip_summary: &mut CorpusSkipSummary,
) -> Result<(), CorpusError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            if should_skip_dir(&path) {
                skip_summary.skipped_directories =
                    skip_summary.skipped_directories.saturating_add(1);
                continue;
            }
            collect_documents(
                root,
                &path,
                chunk_bytes,
                max_file_bytes,
                documents,
                estimated_bytes,
                skip_summary,
            )?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > max_file_bytes {
            skip_summary.oversized_files = skip_summary.oversized_files.saturating_add(1);
            continue;
        }
        if !is_supported_text_path(&path) {
            skip_summary.unsupported_files = skip_summary.unsupported_files.saturating_add(1);
            continue;
        }
        let raw = match fs::read(&path) {
            Ok(raw) => raw,
            Err(_) => {
                skip_summary.unreadable_files = skip_summary.unreadable_files.saturating_add(1);
                continue;
            }
        };
        if raw.contains(&0) {
            skip_summary.binary_files = skip_summary.binary_files.saturating_add(1);
            continue;
        }
        let text = match String::from_utf8(raw) {
            Ok(text) => text,
            Err(_) => {
                skip_summary.binary_files = skip_summary.binary_files.saturating_add(1);
                continue;
            }
        };
        if text.trim().is_empty() {
            skip_summary.empty_files = skip_summary.empty_files.saturating_add(1);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let document_id = CorpusManifest::stable_document_id(&relative);
        let headings = collect_headings(&text);
        let chunks = chunk_document(&document_id, &text, chunk_bytes, headings.first().cloned());
        if chunks.is_empty() {
            continue;
        }
        *estimated_bytes = estimated_bytes.saturating_add(metadata.len());
        documents.push(CorpusDocument {
            document_id,
            path: relative,
            media_type: media_type_for_path(&path),
            language: language_for_path(&path),
            headings,
            bytes: metadata.len(),
            modified_at_ms: metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .and_then(|value| u64::try_from(value.as_millis()).ok()),
            chunks,
        });
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target" | "node_modules" | ".claw")
    )
}

fn is_supported_text_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|v| v.to_ascii_lowercase()),
        Some(ext)
            if matches!(
                ext.as_str(),
                "md" | "markdown" | "txt" | "rs" | "toml" | "json" | "yaml" | "yml" | "js" | "ts" | "tsx" | "py" | "java" | "c" | "cc" | "cpp" | "h" | "hpp" | "go" | "sh"
            )
    )
}

fn media_type_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|v| v.to_ascii_lowercase())
    {
        Some(ext) if matches!(ext.as_str(), "md" | "markdown") => "text/markdown".to_string(),
        Some(ext) if ext == "json" => "application/json".to_string(),
        _ => "text/plain".to_string(),
    }
}

fn language_for_path(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(
        match ext.as_str() {
            "rs" => "rust",
            "md" | "markdown" => "markdown",
            "txt" => "text",
            "toml" => "toml",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "js" => "javascript",
            "ts" | "tsx" => "typescript",
            "py" => "python",
            "java" => "java",
            "c" | "cc" | "cpp" | "h" | "hpp" => "cpp",
            "go" => "go",
            "sh" => "shell",
            _ => return None,
        }
        .to_string(),
    )
}

fn collect_headings(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix('#').map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .take(32)
        .collect()
}

fn chunk_document(
    document_id: &str,
    text: &str,
    chunk_bytes: usize,
    default_heading: Option<String>,
) -> Vec<CorpusChunk> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut ordinal = 0u32;
    let bytes = text.as_bytes();
    let mut active_heading = default_heading;

    while start < bytes.len() {
        let mut end = (start + chunk_bytes).min(bytes.len());
        while end < bytes.len() && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end <= start {
            end = bytes.len();
        }
        if let Some(split) = text[start..end].rfind("\n\n") {
            let candidate = start + split + 2;
            if candidate > start + chunk_bytes / 2 {
                end = candidate;
            }
        } else if let Some(split) = text[start..end].rfind('\n') {
            let candidate = start + split + 1;
            if candidate > start + chunk_bytes / 2 {
                end = candidate;
            }
        }

        let slice = text[start..end].trim();
        if !slice.is_empty() {
            if let Some(latest) = slice
                .lines()
                .filter_map(|line| line.trim().strip_prefix('#').map(str::trim))
                .find(|heading| !heading.is_empty())
            {
                active_heading = Some(latest.to_string());
            }
            let mut metadata = BTreeMap::new();
            metadata.insert("text".to_string(), JsonValue::String(slice.to_string()));
            metadata.insert(
                "heading".to_string(),
                active_heading
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            );
            metadata.insert(
                "preview".to_string(),
                JsonValue::String(make_preview(slice, 220)),
            );
            chunks.push(CorpusChunk {
                chunk_id: CorpusManifest::stable_chunk_id(document_id, ordinal),
                document_id: document_id.to_string(),
                ordinal,
                start_offset: u32::try_from(start).unwrap_or(u32::MAX),
                end_offset: u32::try_from(end).unwrap_or(u32::MAX),
                text_preview: make_preview(slice, 220),
                metadata,
            });
            ordinal = ordinal.saturating_add(1);
        }
        start = end;
    }

    chunks
}

fn make_preview(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        format!(
            "{}…",
            compact
                .chars()
                .take(max_chars)
                .collect::<String>()
                .trim_end()
        )
    }
}

fn infer_corpus_kind(documents: &[CorpusDocument]) -> CorpusKind {
    let mut saw_markdown = false;
    let mut saw_code = false;
    for doc in documents {
        match doc.language.as_deref() {
            Some("markdown" | "text") => saw_markdown = true,
            Some(_) => saw_code = true,
            None => {}
        }
    }
    match (saw_markdown, saw_code) {
        (true, true) => CorpusKind::Mixed,
        (true, false) => CorpusKind::Docs,
        (false, true) => CorpusKind::Repo,
        (false, false) => CorpusKind::Mixed,
    }
}

fn query_tokens(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '.')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn minimum_term_span(haystack: &str, tokens: &[String]) -> Option<usize> {
    let mut positions = Vec::new();
    for token in tokens {
        let position = haystack.find(token)?;
        positions.push((position, position + token.len()));
    }
    let start = positions.iter().map(|(start, _)| *start).min()?;
    let end = positions.iter().map(|(_, end)| *end).max()?;
    Some(end.saturating_sub(start))
}

fn normalize_for_match(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn filename_like_query(tokens: &[String]) -> Option<String> {
    (tokens.len() >= 2).then(|| tokens.join(" ").replace(' ', ""))
}

fn sanitize_id_component(value: &str) -> String {
    value
        .chars()
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
        Some(_) => Err(CorpusError::Format(format!(
            "field {key} must be a string or null"
        ))),
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

fn optional_u64(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<u64>, CorpusError> {
    match object.get(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::Number(value)) => u64::try_from(*value)
            .map(Some)
            .map_err(|_| CorpusError::Format(format!("numeric field {key} is out of range"))),
        Some(_) => Err(CorpusError::Format(format!(
            "field {key} must be a number or null"
        ))),
    }
}

fn optional_u32(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<u32>, CorpusError> {
    optional_u64(object, key)?.map_or(Ok(None), |value| {
        u32::try_from(value)
            .map(Some)
            .map_err(|_| CorpusError::Format(format!("numeric field {key} is out of range")))
    })
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

fn optional_root_summaries(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<CorpusRootSummary>, CorpusError> {
    match object.get(key) {
        Some(JsonValue::Null) | None => Ok(Vec::new()),
        Some(JsonValue::Array(values)) => values
            .iter()
            .map(CorpusRootSummary::from_json_value)
            .collect(),
        Some(_) => Err(CorpusError::Format(format!(
            "field {key} must be an array or null"
        ))),
    }
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
    use super::*;
    use crate::json::JsonValue;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("corpus-{name}-{nanos}"))
    }

    fn temp_manifest_path() -> std::path::PathBuf {
        temp_dir("manifest").with_extension("json")
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
            root_summaries: vec![CorpusRootSummary {
                root: "docs".to_string(),
                document_count: 1,
                chunk_count: 1,
            }],
            skip_summary: CorpusSkipSummary::empty(),
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
                    text_preview: "# Intro Hello world".to_string(),
                    metadata: BTreeMap::from([
                        (
                            "heading".to_string(),
                            JsonValue::String("Intro".to_string()),
                        ),
                        (
                            "text".to_string(),
                            JsonValue::String("# Intro\nHello world".to_string()),
                        ),
                    ]),
                }],
            }],
        }
    }

    #[test]
    fn stable_document_ids_are_path_based_and_repeatable() {
        let first = CorpusManifest::stable_document_id("src/main.rs");
        let second = CorpusManifest::stable_document_id("src/main.rs");
        assert_eq!(first, second);
        assert!(first.contains("src_main_rs"));
    }

    #[test]
    fn manifest_round_trips_through_json_and_disk() {
        let manifest = sample_manifest();
        let value = manifest.to_json_value();
        let reparsed = CorpusManifest::from_json_value(&value).expect("manifest should parse");
        assert_eq!(manifest, reparsed);

        let path = temp_manifest_path();
        manifest.write_to_path(&path).expect("write manifest");
        let loaded = CorpusManifest::read_from_path(&path).expect("read manifest");
        assert_eq!(manifest, loaded);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn attach_chunk_search_inspect_and_slice_round_trip() {
        let cwd = temp_dir("workspace");
        let root = cwd.join("docs");
        fs::create_dir_all(&root).expect("mkdir");
        fs::write(
            root.join("guide.md"),
            "# Intro\nRust retrieval harness\n\n## Search\nLexical search finds chunk previews.\n",
        )
        .expect("write markdown");
        fs::write(root.join("lib.rs"), "fn lexical_search() {}\n// harness\n").expect("write rust");

        let manifest = attach_corpus(&cwd, &[root.clone()], CorpusAttachOptions::default())
            .expect("attach corpus");
        assert_eq!(manifest.backend, CorpusBackend::Lexical);
        assert!(manifest.document_count >= 2);
        assert!(default_corpus_store_dir(&cwd)
            .join(format!("{}.json", manifest.corpus_id))
            .exists());

        let inspect = inspect_corpus(&cwd, &manifest.corpus_id).expect("inspect");
        assert_eq!(inspect.corpus_id, manifest.corpus_id);
        assert!(inspect.documents.iter().any(|doc| doc.path == "guide.md"));
        assert_eq!(inspect.root_summaries.len(), 1);
        assert_eq!(inspect.root_summaries[0].document_count, 2);

        let result = search_corpus(&cwd, &manifest.corpus_id, "lexical search guide", 5, None)
            .expect("search");
        assert!(!result.hits.is_empty());
        assert!(result.hits[0]
            .preview
            .to_ascii_lowercase()
            .contains("lexical"));

        let hit = &result.hits[0];
        let slice = slice_corpus(&cwd, &manifest.corpus_id, Some(&hit.chunk_id), None, None)
            .expect("slice");
        assert_eq!(slice.chunk_id, hit.chunk_id);
        assert!(slice.text.to_ascii_lowercase().contains("lexical"));

        let listed = list_corpora(&cwd).expect("list corpora");
        assert_eq!(listed.len(), 1);

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn chunker_captures_headings_and_is_deterministic() {
        let text = "# Title\nalpha\nbeta\n\n## Details\ngamma\ndelta\n";
        let doc_id = CorpusManifest::stable_document_id("notes.md");
        let chunks_a = chunk_document(&doc_id, text, 20, None);
        let chunks_b = chunk_document(&doc_id, text, 20, None);
        assert_eq!(chunks_a, chunks_b);
        assert!(chunks_a.len() >= 2);
        assert_eq!(
            chunks_a[0].metadata.get("heading"),
            Some(&JsonValue::String("Title".to_string()))
        );
    }

    #[test]
    fn attach_skips_unsupported_binary_and_oversized_files() {
        let cwd = temp_dir("workspace-skip-files");
        let root = cwd.join("docs");
        fs::create_dir_all(&root).expect("mkdir");
        fs::write(root.join("guide.md"), "useful lexical content\n").expect("write markdown");
        fs::write(root.join("binary.md"), b"abc\0def").expect("write binary markdown");
        fs::write(root.join("image.png"), b"png").expect("write unsupported file");
        fs::write(root.join("large.txt"), "x".repeat(64)).expect("write oversized text");

        let manifest = attach_corpus(
            &cwd,
            &[root.clone()],
            CorpusAttachOptions {
                max_file_bytes: 32,
                ..CorpusAttachOptions::default()
            },
        )
        .expect("attach corpus");

        let paths = manifest
            .documents
            .iter()
            .map(|doc| doc.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["guide.md"]);
        assert_eq!(manifest.document_count, 1);
        assert_eq!(manifest.chunk_count, 1);
        assert_eq!(manifest.skip_summary.oversized_files, 1);
        assert_eq!(manifest.skip_summary.unsupported_files, 1);
        assert_eq!(manifest.skip_summary.binary_files, 1);

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn search_prefers_exact_phrase_match_over_partial_keyword_overlap() {
        let cwd = temp_dir("workspace-ranking");
        let root = cwd.join("docs");
        fs::create_dir_all(&root).expect("mkdir");
        fs::write(
            root.join("exact.md"),
            "# Guide\nlexical search guide\nmore filler text\n",
        )
        .expect("write exact");
        fs::write(
            root.join("partial.md"),
            "# Guide\nlexical topics\nsearch notes\nguide appendix\n",
        )
        .expect("write partial");

        let manifest = attach_corpus(&cwd, &[root.clone()], CorpusAttachOptions::default())
            .expect("attach corpus");
        let result = search_corpus(&cwd, &manifest.corpus_id, "lexical search guide", 5, None)
            .expect("search");

        assert_eq!(result.hits[0].path, "exact.md");
        assert!(result.hits[0].reason.contains("phrase:body"));
        assert!(result.hits[0].reason.contains("tight-span:"));
        assert_eq!(result.total_matching_chunks, 2);
        assert!(result.hits[0]
            .matched_terms
            .iter()
            .any(|term| term == "lexical"));
        assert!(result.hits.iter().any(|hit| hit.path == "partial.md"));

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn inspect_reports_per_root_counts_for_multi_corpus_attach() {
        let cwd = temp_dir("workspace-multi-root");
        let docs_root = cwd.join("docs");
        let code_root = cwd.join("src");
        fs::create_dir_all(&docs_root).expect("mkdir docs");
        fs::create_dir_all(&code_root).expect("mkdir src");
        fs::write(docs_root.join("guide.md"), "# Guide\nretrieval notes\n").expect("write docs");
        fs::write(code_root.join("main.rs"), "fn retrieval_notes() {}\n").expect("write code");

        let manifest = attach_corpus(
            &cwd,
            &[docs_root.clone(), code_root.clone()],
            CorpusAttachOptions::default(),
        )
        .expect("attach corpus");
        let inspect = inspect_corpus(&cwd, &manifest.corpus_id).expect("inspect");

        assert_eq!(inspect.root_summaries.len(), 2);
        assert!(inspect
            .root_summaries
            .iter()
            .any(|summary| summary.root.ends_with("docs") && summary.document_count == 1));
        assert!(inspect
            .root_summaries
            .iter()
            .any(|summary| summary.root.ends_with("src") && summary.document_count == 1));

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn search_rewards_filename_matches_for_symbol_split_queries() {
        let cwd = temp_dir("workspace-filename-ranking");
        let root = cwd.join("docs");
        fs::create_dir_all(&root).expect("mkdir");
        fs::write(
            root.join("auth_callback.md"),
            "# Notes\nThis file explains the redirect flow.\n",
        )
        .expect("write filename match");
        fs::write(
            root.join("redirect.md"),
            "# Notes\nauth details and callback steps are described separately here\n",
        )
        .expect("write content-only match");

        let manifest = attach_corpus(&cwd, &[root.clone()], CorpusAttachOptions::default())
            .expect("attach corpus");
        let result =
            search_corpus(&cwd, &manifest.corpus_id, "auth callback", 5, None).expect("search");

        assert_eq!(result.hits[0].path, "auth_callback.md");
        assert!(result.hits[0].reason.contains("filename-match"));

        let _ = fs::remove_dir_all(cwd);
    }
}
