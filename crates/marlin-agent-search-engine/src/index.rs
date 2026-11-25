// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Sub-millisecond resident trigram + SIMD workspace search index for interactive Helm.
//!
//! Scans the workspace once in parallel (`rayon` + `ignore`), retains pre-lowercased
//! byte buffers, line-offset tables, and a 3-byte ASCII trigram inverted index in RAM,
//! and answers admitted Helm queries in tens to hundreds of microseconds with zero
//! disk I/O and zero full-file lowercase reallocations per keystroke.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use agent_semantic_search::{ConvergedLineMatch, SearchGenerationDigest};
use ignore::WalkBuilder;
use memchr::memmem::Finder;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

const DEFAULT_MAX_INDEXED_FILE_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_MAX_LINE_MATCHES_PER_HIT: usize = 6;
const DEFAULT_MAX_LINE_PREVIEW_CHARS: usize = 220;

/// Typed indexing policy supplied by the caller (normally projected from Scheme `scheme/ui-helm`).
///
/// The Rust engine owns traversal, trigram indexing, and SIMD matching, while directory
/// exclusions and byte limits remain declarative configuration owned by the caller/Scheme layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIndexConfig {
    pub skip_dirs: Vec<String>,
    pub max_file_bytes: u64,
    pub max_line_matches_per_hit: usize,
    pub max_line_preview_chars: usize,
}

impl Default for WorkspaceIndexConfig {
    fn default() -> Self {
        Self {
            skip_dirs: vec![".git".to_owned()],
            max_file_bytes: DEFAULT_MAX_INDEXED_FILE_BYTES,
            max_line_matches_per_hit: DEFAULT_MAX_LINE_MATCHES_PER_HIT,
            max_line_preview_chars: DEFAULT_MAX_LINE_PREVIEW_CHARS,
        }
    }
}

/// One line of surrounding context rendered in Helm's follow-preview pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceContextLine {
    pub line_number: usize,
    pub text: String,
    pub is_match: bool,
}

/// One ranked workspace hit produced by [`WorkspaceSearchIndex::query`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchHit {
    pub path: String,
    pub language: Option<String>,
    pub score: u32,
    pub matched_axes: Vec<String>,
    pub first_line: Option<usize>,
    pub first_line_text: String,
    pub line_matches: Vec<ConvergedLineMatch>,
    pub context_lines: Vec<WorkspaceContextLine>,
}

/// Query parameters admitted for sub-millisecond resident index execution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceSearchQuery<'a> {
    pub pattern: &'a str,
    pub languages: &'a [String],
    pub documents: &'a [String],
    pub path_prefix: Option<&'a str>,
    pub extension: Option<&'a str>,
    pub max_hits: usize,
    pub context_radius: usize,
}

/// Sub-millisecond query report with exact microsecond timing and generation digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchReport {
    pub generation_digest: SearchGenerationDigest,
    pub indexed_files: usize,
    pub candidate_files: usize,
    pub matched_files: usize,
    pub elapsed_micros: u64,
    pub hits: Vec<WorkspaceSearchHit>,
}

struct IndexedFile {
    relative_path: String,
    path_lower: Vec<u8>,
    ext: String,
    language: Option<&'static str>,
    body: String,
    body_lower: Vec<u8>,
    line_offsets: Vec<u32>,
}

struct BuiltFileRecord {
    file: IndexedFile,
    hash: [u8; 32],
    trigrams: Vec<u32>,
}

/// Resident, generation-stamped trigram index over a workspace root.
pub struct WorkspaceSearchIndex {
    root: PathBuf,
    config: WorkspaceIndexConfig,
    files: Vec<IndexedFile>,
    trigrams: HashMap<u32, Vec<u32>>,
    generation_digest: SearchGenerationDigest,
    build_micros: u64,
}

impl WorkspaceSearchIndex {
    /// Build a fresh resident trigram index for `root` using default minimal config.
    #[must_use]
    pub fn build(root: &Path) -> Self {
        Self::build_with_config(root, WorkspaceIndexConfig::default())
    }

    /// Build a fresh resident trigram index for `root` governed by caller/Scheme `config`.
    #[must_use]
    pub fn build_with_config(root: &Path, config: WorkspaceIndexConfig) -> Self {
        let started = Instant::now();
        let entries = collect_workspace_entries(root, &config);
        let mut records: Vec<BuiltFileRecord> = entries
            .into_par_iter()
            .filter_map(|(abs_path, rel_path)| build_file_record(&abs_path, rel_path))
            .collect();
        records.sort_unstable_by(|a, b| a.file.relative_path.cmp(&b.file.relative_path));

        let generation_digest = compute_generation_digest(&records);
        let (files, trigrams) = finalize_inverted_index(records);
        let build_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);

        Self {
            root: root.to_path_buf(),
            config,
            files,
            trigrams,
            generation_digest,
            build_micros,
        }
    }

    /// Workspace root owned by this resident index.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of files currently indexed in RAM.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Time spent building this resident snapshot, in microseconds.
    #[must_use]
    pub fn build_micros(&self) -> u64 {
        self.build_micros
    }

    /// Deterministic BLAKE3 generation digest of the indexed workspace snapshot.
    #[must_use]
    pub fn generation_digest(&self) -> &SearchGenerationDigest {
        &self.generation_digest
    }

    /// Execute a sub-millisecond query against the resident RAM trigram index.
    #[must_use]
    pub fn query(&self, query: &WorkspaceSearchQuery<'_>) -> WorkspaceSearchReport {
        execute_resident_query(self, query)
    }
}

fn collect_workspace_entries(root: &Path, config: &WorkspaceIndexConfig) -> Vec<(PathBuf, String)> {
    let mut entries = Vec::new();
    let skip_dirs = config.skip_dirs.clone();
    let max_file_bytes = config.max_file_bytes.max(1);
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .filter_entry(move |entry| {
            if entry.depth() == 0 {
                return true;
            }
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                let name = entry.file_name().to_str().unwrap_or("");
                return !skip_dirs.iter().any(|skip| skip == name);
            }
            true
        })
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.len() == 0 || meta.len() > max_file_bytes {
            continue;
        }
        let path = entry.into_path();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_path = rel.to_string_lossy().replace('\\', "/");
        entries.push((path, rel_path));
    }
    entries
}

fn build_file_record(abs_path: &Path, relative_path: String) -> Option<BuiltFileRecord> {
    let bytes = std::fs::read(abs_path).ok()?;
    if bytes.is_empty() || memchr::memchr(0, &bytes[..bytes.len().min(4096)]).is_some() {
        return None;
    }
    let body = String::from_utf8(bytes).ok()?;
    let hash = *blake3::hash(body.as_bytes()).as_bytes();
    let ext = Path::new(&relative_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let language = infer_language(&ext);
    let path_lower = ascii_lower_bytes(relative_path.as_bytes());
    let body_lower = ascii_lower_bytes(body.as_bytes());
    let line_offsets = build_line_offsets(body.as_bytes());
    let trigrams = extract_sorted_trigrams(&path_lower, &body_lower);

    Some(BuiltFileRecord {
        file: IndexedFile {
            relative_path,
            path_lower,
            ext,
            language,
            body,
            body_lower,
            line_offsets,
        },
        hash,
        trigrams,
    })
}

fn compute_generation_digest(records: &[BuiltFileRecord]) -> SearchGenerationDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"marlin.helm.resident-index.v1\n");
    for rec in records {
        hasher.update(rec.file.relative_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(&rec.hash);
        hasher.update(b"\n");
    }
    let digest = format!("blake3-256:{}", hasher.finalize().to_hex());
    SearchGenerationDigest::new(digest).expect("valid blake3-256 generation digest")
}

fn finalize_inverted_index(
    records: Vec<BuiltFileRecord>,
) -> (Vec<IndexedFile>, HashMap<u32, Vec<u32>>) {
    let mut files = Vec::with_capacity(records.len());
    let mut trigrams: HashMap<u32, Vec<u32>> = HashMap::with_capacity(16_384);
    for (idx, rec) in records.into_iter().enumerate() {
        let file_id = u32::try_from(idx).unwrap_or(u32::MAX);
        for tri in rec.trigrams {
            trigrams.entry(tri).or_default().push(file_id);
        }
        files.push(rec.file);
    }
    (files, trigrams)
}

fn execute_resident_query(
    index: &WorkspaceSearchIndex,
    query: &WorkspaceSearchQuery<'_>,
) -> WorkspaceSearchReport {
    let started = Instant::now();
    let max_hits = query.max_hits.max(1);
    let allowed_exts = allowed_extensions(query.languages, query.documents);
    let prefix_lower = query.path_prefix.map(|p| p.to_ascii_lowercase());
    let ext_filter = query
        .extension
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase());
    let pattern = query.pattern.trim();
    let needle_lower = ascii_lower_bytes(pattern.as_bytes());
    let has_topology = prefix_lower.is_some() || ext_filter.is_some();

    if needle_lower.is_empty() {
        let hits = list_files_matching_filters(
            index,
            &allowed_exts,
            prefix_lower.as_deref(),
            ext_filter.as_deref(),
            max_hits,
            query.context_radius,
            has_topology,
        );
        let matched = hits.len();
        return WorkspaceSearchReport {
            generation_digest: index.generation_digest.clone(),
            indexed_files: index.files.len(),
            candidate_files: index.files.len(),
            matched_files: matched,
            elapsed_micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
            hits,
        };
    }

    let candidates = select_candidate_file_ids(index, &needle_lower);
    let candidate_count = candidates.len();
    let finder = Finder::new(&needle_lower);
    let exact_finder =
        (pattern.as_bytes() != needle_lower.as_slice()).then(|| Finder::new(pattern));

    let mut hits = Vec::new();
    for file_id in candidates {
        let Some(file) = index.files.get(file_id as usize) else {
            continue;
        };
        if !matches_file_filters(
            file,
            &allowed_exts,
            prefix_lower.as_deref(),
            ext_filter.as_deref(),
        ) {
            continue;
        }
        if let Some(hit) = evaluate_indexed_file(
            file,
            &index.config,
            &finder,
            exact_finder.as_ref(),
            has_topology,
            query.context_radius,
        ) {
            hits.push(hit);
            // When typing 1–2 chars, avoid scanning all files once plenty of high-signal hits exist.
            if needle_lower.len() < 3 && hits.len() >= max_hits * 4 {
                break;
            }
        }
    }

    hits.sort_unstable_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    let matched_files = hits.len();
    hits.truncate(max_hits);

    WorkspaceSearchReport {
        generation_digest: index.generation_digest.clone(),
        indexed_files: index.files.len(),
        candidate_files: candidate_count,
        matched_files,
        elapsed_micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        hits,
    }
}

fn select_candidate_file_ids(index: &WorkspaceSearchIndex, needle_lower: &[u8]) -> Vec<u32> {
    if needle_lower.len() < 3 {
        return (0..index.files.len())
            .filter_map(|i| u32::try_from(i).ok())
            .collect();
    }
    let mut query_trigrams = Vec::with_capacity(needle_lower.len().saturating_sub(2));
    for win in needle_lower.windows(3) {
        query_trigrams.push(pack_trigram(win[0], win[1], win[2]));
    }
    query_trigrams.sort_unstable();
    query_trigrams.dedup();

    let mut posting_lists: Vec<&[u32]> = Vec::with_capacity(query_trigrams.len());
    for tri in query_trigrams {
        let Some(list) = index.trigrams.get(&tri) else {
            return Vec::new();
        };
        posting_lists.push(list.as_slice());
    }
    posting_lists.sort_unstable_by_key(|list| list.len());

    let mut acc = posting_lists[0].to_vec();
    for list in &posting_lists[1..] {
        acc = intersect_sorted(&acc, list);
        if acc.is_empty() {
            break;
        }
    }
    acc
}

fn intersect_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len().min(b.len()));
    if a.len().saturating_mul(8) < b.len() {
        for &val in a {
            if b.binary_search(&val).is_ok() {
                out.push(val);
            }
        }
        return out;
    }
    let (mut i, mut j) = (0usize, 0usize);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    out
}

fn list_files_matching_filters(
    index: &WorkspaceSearchIndex,
    allowed_exts: &Option<BTreeSet<&'static str>>,
    prefix_lower: Option<&str>,
    ext_filter: Option<&str>,
    max_hits: usize,
    context_radius: usize,
    has_topology: bool,
) -> Vec<WorkspaceSearchHit> {
    let mut hits = Vec::new();
    for file in &index.files {
        if !matches_file_filters(file, allowed_exts, prefix_lower, ext_filter) {
            continue;
        }
        let context_lines =
            extract_context_lines(file, 1, context_radius, index.config.max_line_preview_chars);
        let first_line_text = context_lines
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();
        let mut axes = Vec::new();
        if has_topology {
            axes.push("topology".to_owned());
        } else {
            axes.push("index".to_owned());
        }
        hits.push(WorkspaceSearchHit {
            path: file.relative_path.clone(),
            language: file.language.map(str::to_owned),
            score: 50,
            matched_axes: axes,
            first_line: Some(1),
            first_line_text,
            line_matches: Vec::new(),
            context_lines,
        });
        if hits.len() >= max_hits {
            break;
        }
    }
    hits
}

fn evaluate_indexed_file(
    file: &IndexedFile,
    config: &WorkspaceIndexConfig,
    finder: &Finder<'_>,
    exact_finder: Option<&Finder<'_>>,
    has_topology: bool,
    context_radius: usize,
) -> Option<WorkspaceSearchHit> {
    let path_match = finder.find(&file.path_lower).is_some();
    let mut line_matches = Vec::new();
    let mut last_line_num = 0usize;
    let mut exact_case = false;
    let max_matches = config.max_line_matches_per_hit.max(1);
    let max_preview_chars = config.max_line_preview_chars.max(16);

    for byte_offset in finder.find_iter(&file.body_lower) {
        let line_num = byte_offset_to_line(&file.line_offsets, byte_offset);
        if line_num == last_line_num {
            continue;
        }
        last_line_num = line_num;
        let line_text = extract_line_slice(file, line_num);
        if !exact_case
            && let Some(exact) = exact_finder
            && exact.find(line_text.as_bytes()).is_some()
        {
            exact_case = true;
        }
        line_matches.push(ConvergedLineMatch {
            line_number: line_num,
            line_text: truncate_line(line_text, max_preview_chars),
        });
        if line_matches.len() >= max_matches {
            break;
        }
    }

    if !path_match && line_matches.is_empty() {
        return None;
    }

    let mut score = 40u32;
    let mut matched_axes = Vec::new();
    if has_topology {
        matched_axes.push("topology".to_owned());
        score = score.saturating_add(10);
    }
    if path_match {
        matched_axes.push("path".to_owned());
        score = score.saturating_add(90);
        if let Some(basename) = file.relative_path.rsplit('/').next()
            && finder
                .find(&ascii_lower_bytes(basename.as_bytes()))
                .is_some()
        {
            score = score.saturating_add(50);
        }
    }
    if !line_matches.is_empty() {
        matched_axes.push("trigram+rg".to_owned());
        score = score.saturating_add(60 + (line_matches.len() as u32) * 8);
    }
    if exact_case {
        score = score.saturating_add(25);
    }
    let path_penalty = u32::try_from(file.relative_path.len() / 8)
        .unwrap_or(20)
        .min(20);
    score = score.saturating_sub(path_penalty);

    let first_line = line_matches.first().map(|m| m.line_number).or(Some(1));
    let first_line_text = line_matches
        .first()
        .map(|m| m.line_text.clone())
        .unwrap_or_else(|| truncate_line(extract_line_slice(file, 1), max_preview_chars));
    let focus_line = first_line.unwrap_or(1);
    let context_lines = extract_context_lines(file, focus_line, context_radius, max_preview_chars);

    Some(WorkspaceSearchHit {
        path: file.relative_path.clone(),
        language: file.language.map(str::to_owned),
        score,
        matched_axes,
        first_line,
        first_line_text,
        line_matches,
        context_lines,
    })
}

fn extract_context_lines(
    file: &IndexedFile,
    focus_line: usize,
    radius: usize,
    max_preview_chars: usize,
) -> Vec<WorkspaceContextLine> {
    let total_lines = file.line_offsets.len();
    if total_lines == 0 {
        return Vec::new();
    }
    let focus = focus_line.clamp(1, total_lines);
    let start = focus.saturating_sub(radius).max(1);
    let end = focus.saturating_add(radius).min(total_lines);
    let mut out = Vec::with_capacity(end - start + 1);
    for line_num in start..=end {
        out.push(WorkspaceContextLine {
            line_number: line_num,
            text: truncate_line(extract_line_slice(file, line_num), max_preview_chars),
            is_match: line_num == focus,
        });
    }
    out
}

fn extract_line_slice(file: &IndexedFile, line_num: usize) -> &str {
    let Some(&start_u32) = file.line_offsets.get(line_num.saturating_sub(1)) else {
        return "";
    };
    let start = start_u32 as usize;
    let end = file
        .line_offsets
        .get(line_num)
        .map_or(file.body.len(), |&next| next as usize);
    file.body
        .get(start..end)
        .unwrap_or("")
        .trim_end_matches(['\r', '\n'])
}

fn byte_offset_to_line(line_offsets: &[u32], byte_offset: usize) -> usize {
    let target = u32::try_from(byte_offset).unwrap_or(u32::MAX);
    line_offsets
        .partition_point(|&start| start <= target)
        .max(1)
}

fn build_line_offsets(bytes: &[u8]) -> Vec<u32> {
    let mut offsets = Vec::with_capacity(64);
    offsets.push(0);
    for pos in memchr::memchr_iter(b'\n', bytes) {
        if let Some(next) = pos.checked_add(1)
            && next < bytes.len()
            && let Ok(off) = u32::try_from(next)
        {
            offsets.push(off);
        }
    }
    offsets
}

fn ascii_lower_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        out.push(b.to_ascii_lowercase());
    }
    out
}

fn extract_sorted_trigrams(path_lower: &[u8], body_lower: &[u8]) -> Vec<u32> {
    let mut tris = Vec::with_capacity(
        path_lower
            .len()
            .saturating_add(body_lower.len().min(65_536)),
    );
    for win in path_lower.windows(3) {
        tris.push(pack_trigram(win[0], win[1], win[2]));
    }
    for win in body_lower.windows(3) {
        tris.push(pack_trigram(win[0], win[1], win[2]));
    }
    tris.sort_unstable();
    tris.dedup();
    tris
}

#[inline]
fn pack_trigram(b0: u8, b1: u8, b2: u8) -> u32 {
    (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2)
}

fn truncate_line(line: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (i, ch) in line.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            break;
        }
        if ch == '\t' {
            out.push_str("  ");
        } else if !ch.is_control() {
            out.push(ch);
        }
    }
    out
}

fn matches_file_filters(
    file: &IndexedFile,
    allowed_exts: &Option<BTreeSet<&'static str>>,
    prefix_lower: Option<&str>,
    ext_filter: Option<&str>,
) -> bool {
    if let Some(prefix) = prefix_lower
        && !file.path_lower.starts_with(prefix.as_bytes())
    {
        return false;
    }
    if let Some(ext) = ext_filter
        && file.ext != ext
    {
        return false;
    }
    if let Some(allowed) = allowed_exts
        && !allowed.contains(file.ext.as_str())
    {
        return false;
    }
    true
}

fn allowed_extensions(
    languages: &[String],
    documents: &[String],
) -> Option<BTreeSet<&'static str>> {
    let mut exts = BTreeSet::new();
    for lang in languages {
        for ext in extensions_for_language(lang.as_str()) {
            exts.insert(*ext);
        }
    }
    for doc in documents {
        for ext in extensions_for_document(doc.as_str()) {
            exts.insert(*ext);
        }
    }
    if exts.is_empty() { None } else { Some(exts) }
}

fn extensions_for_language(language: &str) -> &'static [&'static str] {
    match language {
        "rust" | "rs" => &["rs"],
        "typescript" | "ts" => &["ts", "tsx", "mts", "cts"],
        "javascript" | "js" => &["js", "jsx", "mjs", "cjs"],
        "python" | "py" => &["py", "pyi"],
        "julia" | "jl" => &["jl"],
        "gerbil-scheme" | "scheme" | "ss" => &["ss", "scm"],
        "sysml" => &["sysml"],
        "quint" | "qnt" => &["qnt"],
        _ => &[],
    }
}

fn extensions_for_document(document: &str) -> &'static [&'static str] {
    match document {
        "org" => &["org"],
        "markdown" | "md" => &["md", "markdown"],
        "toml" => &["toml"],
        "json" => &["json"],
        _ => &[],
    }
}

fn infer_language(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("rust"),
        "ts" | "tsx" | "mts" | "cts" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" | "pyi" => Some("python"),
        "jl" => Some("julia"),
        "ss" | "scm" => Some("scheme"),
        "org" => Some("org"),
        "md" | "markdown" => Some("markdown"),
        "toml" => Some("toml"),
        "sysml" => Some("sysml"),
        "qnt" => Some("quint"),
        _ => None,
    }
}
