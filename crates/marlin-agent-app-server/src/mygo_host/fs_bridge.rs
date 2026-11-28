// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! High-performance Rust-owned filesystem, live grep, and project discovery
//! projection (`fs:ls`, `fs:project-files`, `fs:grep`, `fs:projects`, `fs:read`)
//! over native HTTP/2 gRPC (`ProjectionQueryService::QueryProjectGraph`).
//!
//! Serves both local sidecar and remote Marlin Authority Host workspaces uniformly
//! over typed protobuf streams, backed by VCS-native (`git ls-files -z`) project
//! indexing, TTL caches, pre-lowercased SIMD Orderless filtering, and native `rg`
//! live project grep (`consult-ripgrep`).

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant, UNIX_EPOCH},
};

use crate::wire::marlin::projection::v1::{GraphRecordView, GraphVisibleSurface};

const MAX_DIR_ENTRIES: usize = 512;
const MAX_PROJECT_FILES: usize = 8192;
const MAX_SERVER_FILTER_RESULTS: usize = 256;
const MAX_GREP_HITS: usize = 200;
const MAX_READ_BYTES: usize = 256 * 1024;
const PROJECT_INDEX_TTL: Duration = Duration::from_secs(5);
const KNOWN_PROJECTS_TTL: Duration = Duration::from_secs(15);

const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".jj",
    "target",
    "node_modules",
    ".gerbil",
    ".cache",
    ".devenv",
    ".direnv",
    "dist",
    "build",
    "__pycache__",
];

#[derive(Clone, Debug)]
struct IndexedProjectFile {
    rel_path: String,
    rel_lower: String,
    abs_path: String,
}

#[derive(Clone)]
struct CachedProjectIndex {
    built_at: Instant,
    files: Arc<Vec<IndexedProjectFile>>,
}

#[derive(Clone, Debug)]
struct DiscoveredProject {
    name: String,
    path: String,
    vcs: &'static str,
}

#[derive(Clone)]
struct CachedProjectList {
    built_at: Instant,
    projects: Arc<Vec<DiscoveredProject>>,
}

static PROJECT_INDEX_CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedProjectIndex>>> = OnceLock::new();
static KNOWN_PROJECTS_CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedProjectList>>> = OnceLock::new();

fn project_cache() -> &'static Mutex<HashMap<PathBuf, CachedProjectIndex>> {
    PROJECT_INDEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn known_projects_cache() -> &'static Mutex<HashMap<PathBuf, CachedProjectList>> {
    KNOWN_PROJECTS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Handles `fs:ls[@dir]`, `fs:project-files[!][@root][?q=<orderless>]`,
/// `fs:grep[@root]?q=<pattern>`, `fs:projects[@root]`, and `fs:read@<path>`
/// queries against the live host filesystem rooted at `default_root`.
pub fn query_fs_records(
    query: &str,
    default_root: &Path,
) -> Option<Result<Vec<GraphRecordView>, String>> {
    let q = query.trim();
    if let Some(rest) = q.strip_prefix("fs:ls") {
        let arg = rest.strip_prefix('@').map(str::trim).unwrap_or("");
        return Some(list_directory_records(arg, default_root));
    }
    if let Some(rest) = q.strip_prefix("fs:project-files") {
        let (force_refresh, after_bang) = match rest.strip_prefix('!') {
            Some(r) => (true, r),
            None => (false, rest),
        };
        let (root_part, filter_query) = match after_bang.split_once("?q=") {
            Some((r, fq)) => (r, Some(fq.trim())),
            None => (after_bang, None),
        };
        let arg = root_part.strip_prefix('@').map(str::trim).unwrap_or("");
        return Some(list_project_file_records(
            arg,
            default_root,
            force_refresh,
            filter_query.filter(|s| !s.is_empty()),
        ));
    }
    if let Some(rest) = q.strip_prefix("fs:grep") {
        let (root_part, pattern) = match rest.split_once("?q=") {
            Some((r, pat)) => (r, pat.trim()),
            None => (rest, ""),
        };
        let arg = root_part.strip_prefix('@').map(str::trim).unwrap_or("");
        return Some(grep_project_records(arg, default_root, pattern));
    }
    if let Some(rest) = q.strip_prefix("fs:projects") {
        let (force_refresh, after_bang) = match rest.strip_prefix('!') {
            Some(r) => (true, r),
            None => (false, rest),
        };
        let arg = after_bang.strip_prefix('@').map(str::trim).unwrap_or("");
        return Some(list_known_project_records(arg, default_root, force_refresh));
    }
    if let Some(path_arg) = q.strip_prefix("fs:read@") {
        return Some(read_file_record(path_arg.trim(), default_root));
    }
    None
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn resolve_path(raw: &str, default_root: &Path) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return canonical_or_self(default_root);
    }
    if trimmed == "~" {
        return canonical_or_self(&home_dir());
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return canonical_or_self(&home_dir().join(rest));
    }
    let p = PathBuf::from(trimmed);
    if p.is_absolute() {
        canonical_or_self(&p)
    } else {
        canonical_or_self(&default_root.join(p))
    }
}

fn canonical_or_self(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn discover_project_root(start: &Path) -> PathBuf {
    let base = canonical_or_self(start);
    let mut cur = base.as_path();
    loop {
        if cur.join(".git").exists() || cur.join(".jj").is_dir() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(p) if p != cur => cur = p,
            _ => return base,
        }
    }
}

fn detect_vcs_kind(dir: &Path) -> Option<&'static str> {
    let has_git = dir.join(".git").exists();
    let has_jj = dir.join(".jj").is_dir();
    match (has_jj, has_git) {
        (true, true) => Some("jj+git"),
        (true, false) => Some("jj"),
        (false, true) => Some("git"),
        (false, false) => None,
    }
}

/// Zero-allocation ASCII case-insensitive comparison for fast directory sorting.
fn cmp_ignore_ascii_case(a: &str, b: &str) -> Ordering {
    for (ba, bb) in a.bytes().zip(b.bytes()) {
        let ord = ba.to_ascii_lowercase().cmp(&bb.to_ascii_lowercase());
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a.len().cmp(&b.len())
}

fn list_directory_records(
    raw_dir: &str,
    default_root: &Path,
) -> Result<Vec<GraphRecordView>, String> {
    let dir = resolve_path(raw_dir, default_root);
    let read_dir =
        fs::read_dir(&dir).map_err(|err| format!("read directory {}: {err}", dir.display()))?;

    let parent = dir
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| dir.display().to_string());
    let home = home_dir().display().to_string();

    let mut meta_facets = HashMap::with_capacity(4);
    meta_facets.insert("dir".to_owned(), dir.display().to_string());
    meta_facets.insert("parent".to_owned(), parent);
    meta_facets.insert("home".to_owned(), home);
    meta_facets.insert(
        "workspace_root".to_owned(),
        default_root.display().to_string(),
    );

    let mut records = vec![GraphRecordView {
        record_id: "fs.meta".to_owned(),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "fs.meta".to_owned(),
        title: dir.display().to_string(),
        summary: "Live directory listing".to_owned(),
        source_path: Some(dir.display().to_string()),
        start_line: None,
        end_line: None,
        facets: meta_facets,
    }];

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let ft = entry.file_type().ok();
        let is_dir = match ft {
            Some(t) if t.is_dir() => true,
            Some(t) if t.is_symlink() => fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false),
            _ => false,
        };
        if is_dir {
            dirs.push((name, path));
        } else {
            let meta = entry.metadata().ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mod_unix = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push((name, path, size, mod_unix));
        }
    }
    dirs.sort_by(|a, b| cmp_ignore_ascii_case(&a.0, &b.0));
    files.sort_by(|a, b| cmp_ignore_ascii_case(&a.0, &b.0));

    for (name, path) in dirs.into_iter().take(MAX_DIR_ENTRIES / 2) {
        let path_str = path.display().to_string();
        let mut facets = HashMap::with_capacity(3);
        facets.insert("name".to_owned(), format!("{name}/"));
        facets.insert("path".to_owned(), path_str.clone());
        facets.insert("is_dir".to_owned(), "true".to_owned());
        records.push(GraphRecordView {
            record_id: format!("fs.dir:{name}"),
            surface: GraphVisibleSurface::Workspace as i32,
            kind: "fs.dir".to_owned(),
            title: format!("{name}/"),
            summary: "dir".to_owned(),
            source_path: Some(path_str),
            start_line: None,
            end_line: None,
            facets,
        });
    }

    let remaining = MAX_DIR_ENTRIES.saturating_sub(records.len());
    for (name, path, size, mod_unix) in files.into_iter().take(remaining) {
        let path_str = path.display().to_string();
        let mut facets = HashMap::with_capacity(5);
        facets.insert("name".to_owned(), name.clone());
        facets.insert("path".to_owned(), path_str.clone());
        facets.insert("is_dir".to_owned(), "false".to_owned());
        facets.insert("size".to_owned(), size.to_string());
        facets.insert("mod_unix".to_owned(), mod_unix.to_string());
        records.push(GraphRecordView {
            record_id: format!("fs.file:{name}"),
            surface: GraphVisibleSurface::Workspace as i32,
            kind: "fs.file".to_owned(),
            title: name,
            summary: format_bytes(size),
            source_path: Some(path_str),
            start_line: None,
            end_line: None,
            facets,
        });
    }

    Ok(records)
}

/// Fast VCS-native file enumeration (`git ls-files --cached --others --exclude-standard -z`)
/// which reads Git's index directly without per-file `stat()` storms and respects `.gitignore`.
fn scan_project_files_git(root: &Path) -> Option<Vec<IndexedProjectFile>> {
    if !root.join(".git").exists() {
        return None;
    }
    let output = Command::new("git")
        .args([
            "-C",
            root.to_str()?,
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root_prefix = {
        let s = root.to_string_lossy();
        let trimmed = s.trim_end_matches('/');
        format!("{trimmed}/")
    };
    let mut files = Vec::with_capacity(1024);
    for raw in output.stdout.split(|&b| b == 0) {
        if raw.is_empty() {
            continue;
        }
        let rel = String::from_utf8_lossy(raw).into_owned();
        let rel_lower = rel.to_lowercase();
        let abs = format!("{root_prefix}{rel}");
        files.push(IndexedProjectFile {
            rel_path: rel,
            rel_lower,
            abs_path: abs,
        });
        if files.len() >= MAX_PROJECT_FILES {
            break;
        }
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Some(files)
}

/// Fallback filesystem walker for non-Git workspaces (zero extra `stat()` syscalls).
fn scan_project_files_walk(root: &Path) -> Vec<IndexedProjectFile> {
    let mut stack = vec![root.to_path_buf()];
    let mut collected = Vec::with_capacity(512);
    while let Some(dir) = stack.pop() {
        if collected.len() >= MAX_PROJECT_FILES {
            break;
        }
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        let mut subdirs = Vec::new();
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                if IGNORED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                subdirs.push(path);
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let rel_lower = rel.to_lowercase();
                collected.push(IndexedProjectFile {
                    rel_path: rel,
                    rel_lower,
                    abs_path: path.display().to_string(),
                });
                if collected.len() >= MAX_PROJECT_FILES {
                    break;
                }
            }
        }
        subdirs.sort();
        for sd in subdirs.into_iter().rev() {
            stack.push(sd);
        }
    }
    collected.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    collected
}

fn load_or_refresh_project_index(root: &Path, force_refresh: bool) -> Arc<Vec<IndexedProjectFile>> {
    if !force_refresh
        && let Ok(guard) = project_cache().lock()
        && let Some(cached) = guard.get(root)
        && cached.built_at.elapsed() < PROJECT_INDEX_TTL
    {
        return Arc::clone(&cached.files);
    }

    let scanned = scan_project_files_git(root).unwrap_or_else(|| scan_project_files_walk(root));
    let arc = Arc::new(scanned);
    if let Ok(mut guard) = project_cache().lock() {
        guard.insert(
            root.to_path_buf(),
            CachedProjectIndex {
                built_at: Instant::now(),
                files: Arc::clone(&arc),
            },
        );
    }
    arc
}

/// Server-side Rust Orderless filter over the pre-lowercased project index using
/// Rust stdlib's SIMD/Two-Way `str::contains` (supports out-of-order tokens,
/// `!neg` exclusion, and prefix-first ranking with zero per-item allocations).
fn filter_project_files_orderless<'a>(
    files: &'a [IndexedProjectFile],
    query: &str,
    limit: usize,
) -> Vec<&'a IndexedProjectFile> {
    let q_lower = query.trim().to_lowercase();
    let tokens: Vec<(bool, &str)> = q_lower
        .split_whitespace()
        .map(|raw| {
            if let Some(neg) = raw.strip_prefix('!')
                && !neg.is_empty()
            {
                (true, neg)
            } else {
                (false, raw)
            }
        })
        .collect();
    if tokens.is_empty() {
        return files.iter().take(limit).collect();
    }
    let mut tier0 = Vec::new();
    let mut tier1 = Vec::new();
    for f in files {
        let mut matched = true;
        let mut has_prefix = false;
        for &(neg, tok) in &tokens {
            let hit = f.rel_lower.contains(tok);
            if neg {
                if hit {
                    matched = false;
                    break;
                }
            } else if !hit {
                matched = false;
                break;
            } else if f.rel_lower.starts_with(tok) {
                has_prefix = true;
            }
        }
        if !matched {
            continue;
        }
        if has_prefix {
            tier0.push(f);
        } else {
            tier1.push(f);
        }
    }
    tier0.truncate(limit);
    let rem = limit.saturating_sub(tier0.len());
    tier0.extend(tier1.into_iter().take(rem));
    tier0
}

fn list_project_file_records(
    raw_root: &str,
    default_root: &Path,
    force_refresh: bool,
    filter_query: Option<&str>,
) -> Result<Vec<GraphRecordView>, String> {
    let base = resolve_path(raw_root, default_root);
    let root = discover_project_root(&base);
    let indexed = load_or_refresh_project_index(&root, force_refresh);

    let mut meta_facets = HashMap::with_capacity(2);
    meta_facets.insert("root".to_owned(), root.display().to_string());
    meta_facets.insert("total".to_owned(), indexed.len().to_string());

    let selected: Vec<&IndexedProjectFile> = match filter_query {
        Some(q) => filter_project_files_orderless(&indexed, q, MAX_SERVER_FILTER_RESULTS),
        None => indexed.iter().collect(),
    };

    let mut records = Vec::with_capacity(selected.len() + 1);
    records.push(GraphRecordView {
        record_id: "fs.project_meta".to_owned(),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "fs.project_meta".to_owned(),
        title: root.display().to_string(),
        summary: format!("{} files", indexed.len()),
        source_path: Some(root.display().to_string()),
        start_line: None,
        end_line: None,
        facets: meta_facets,
    });

    for item in selected {
        let mut facets = HashMap::with_capacity(2);
        facets.insert("rel_path".to_owned(), item.rel_path.clone());
        facets.insert("path".to_owned(), item.abs_path.clone());
        records.push(GraphRecordView {
            record_id: format!("fs.pf:{}", item.rel_path),
            surface: GraphVisibleSurface::Workspace as i32,
            kind: "fs.project_file".to_owned(),
            title: item.rel_path.clone(),
            summary: String::new(),
            source_path: Some(item.abs_path.clone()),
            start_line: None,
            end_line: None,
            facets,
        });
    }

    Ok(records)
}

/// Live Doom Emacs `consult-ripgrep` (`fs:grep[@root]?q=<pattern>`) executed
/// natively in Rust via `rg --vimgrep` (respecting `.gitignore` + smart-case),
/// with an automatic pure-Rust fallback over the cached project index.
fn grep_project_records(
    raw_root: &str,
    default_root: &Path,
    pattern: &str,
) -> Result<Vec<GraphRecordView>, String> {
    let base = resolve_path(raw_root, default_root);
    let root = discover_project_root(&base);

    let mut meta_facets = HashMap::with_capacity(2);
    meta_facets.insert("root".to_owned(), root.display().to_string());
    meta_facets.insert("query".to_owned(), pattern.to_owned());

    let mut records = vec![GraphRecordView {
        record_id: "fs.grep_meta".to_owned(),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "fs.grep_meta".to_owned(),
        title: root.display().to_string(),
        summary: format!("consult-ripgrep: {pattern}"),
        source_path: Some(root.display().to_string()),
        start_line: None,
        end_line: None,
        facets: meta_facets,
    }];

    if pattern.is_empty() {
        return Ok(records);
    }

    let root_prefix = {
        let s = root.to_string_lossy();
        format!("{}/", s.trim_end_matches('/'))
    };

    if let Some(hits) = run_ripgrep(&root, pattern) {
        for (idx, (rel, line, col, snippet)) in hits.into_iter().take(MAX_GREP_HITS).enumerate() {
            let abs = format!("{root_prefix}{rel}");
            let mut facets = HashMap::with_capacity(5);
            facets.insert("rel_path".to_owned(), rel.clone());
            facets.insert("path".to_owned(), abs.clone());
            facets.insert("line".to_owned(), line.to_string());
            facets.insert("col".to_owned(), col.to_string());
            facets.insert("snippet".to_owned(), snippet.clone());
            records.push(GraphRecordView {
                record_id: format!("fs.grep:{idx}:{rel}:{line}"),
                surface: GraphVisibleSurface::Workspace as i32,
                kind: "fs.grep_hit".to_owned(),
                title: format!("{rel}:{line}"),
                summary: snippet,
                source_path: Some(abs),
                start_line: Some(line),
                end_line: Some(line),
                facets,
            });
        }
        return Ok(records);
    }

    // Pure-Rust fallback over cached VCS project index when `rg` is not on PATH.
    let needle = pattern.to_lowercase();
    let indexed = load_or_refresh_project_index(&root, false);
    let mut hit_idx = 0usize;
    'outer: for item in indexed.iter() {
        let Ok(bytes) = fs::read(&item.abs_path) else {
            continue;
        };
        if bytes.len() > MAX_READ_BYTES || bytes.iter().take(2048).any(|&b| b == 0) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (ln0, line_text) in text.lines().enumerate() {
            if line_text.to_lowercase().contains(&needle) {
                let line_num = (ln0 + 1) as u32;
                let snippet = trim_snippet(line_text, 200);
                let mut facets = HashMap::with_capacity(5);
                facets.insert("rel_path".to_owned(), item.rel_path.clone());
                facets.insert("path".to_owned(), item.abs_path.clone());
                facets.insert("line".to_owned(), line_num.to_string());
                facets.insert("col".to_owned(), "1".to_owned());
                facets.insert("snippet".to_owned(), snippet.clone());
                records.push(GraphRecordView {
                    record_id: format!("fs.grep:{hit_idx}:{}:{line_num}", item.rel_path),
                    surface: GraphVisibleSurface::Workspace as i32,
                    kind: "fs.grep_hit".to_owned(),
                    title: format!("{}:{line_num}", item.rel_path),
                    summary: snippet,
                    source_path: Some(item.abs_path.clone()),
                    start_line: Some(line_num),
                    end_line: Some(line_num),
                    facets,
                });
                hit_idx += 1;
                if hit_idx >= MAX_GREP_HITS {
                    break 'outer;
                }
            }
        }
    }
    Ok(records)
}

fn run_ripgrep(root: &Path, pattern: &str) -> Option<Vec<(String, u32, u32, String)>> {
    let output = Command::new("rg")
        .current_dir(root)
        .args([
            "--vimgrep",
            "--no-heading",
            "--color=never",
            "--smart-case",
            "--fixed-strings",
            "--max-columns=240",
            "--max-count=32",
            "--",
            pattern,
        ])
        .output()
        .ok()?;
    // rg exits 0 (matches), 1 (no matches), 2 (error).
    if output.status.code() == Some(2) {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut hits = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(4, ':');
        let Some(rel) = parts.next() else { continue };
        let Some(line_s) = parts.next() else { continue };
        let Some(col_s) = parts.next() else { continue };
        let Some(text) = parts.next() else { continue };
        let Ok(line_num) = line_s.parse::<u32>() else {
            continue;
        };
        let col_num = col_s.parse::<u32>().unwrap_or(1);
        hits.push((
            rel.replace('\\', "/"),
            line_num,
            col_num,
            trim_snippet(text, 200),
        ));
        if hits.len() >= MAX_GREP_HITS {
            break;
        }
    }
    Some(hits)
}

fn trim_snippet(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_owned()
    } else {
        let mut out: String = trimmed.chars().take(max_chars).collect();
        out.push('…');
        out
    }
}

/// Discovers real Git/jj repositories around `default_root` (active repo, nested
/// `.data/*` / `packages/*` repos, and sibling repos in `parent`) for Doom Emacs
/// `project-switch-project` (`SPC p p` / `C-c p p`).
fn discover_known_projects(anchor: &Path, force_refresh: bool) -> Arc<Vec<DiscoveredProject>> {
    if !force_refresh
        && let Ok(guard) = known_projects_cache().lock()
        && let Some(cached) = guard.get(anchor)
        && cached.built_at.elapsed() < KNOWN_PROJECTS_TTL
    {
        return Arc::clone(&cached.projects);
    }

    let mut found: BTreeMap<String, DiscoveredProject> = BTreeMap::new();
    let mut record_if_repo = |dir: &Path| {
        let canon = canonical_or_self(dir);
        if let Some(vcs) = detect_vcs_kind(&canon) {
            let path_str = canon.display().to_string();
            let name = canon
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.clone());
            found.entry(path_str.clone()).or_insert(DiscoveredProject {
                name,
                path: path_str,
                vcs,
            });
        }
    };

    let active_root = discover_project_root(anchor);
    record_if_repo(&active_root);

    // Inspect nested companion repos (e.g. `.data/marlin-ui`, `.data/marlin-mygo`, `packages/*`).
    for nested in [".data", "packages", "repos"] {
        let sub = active_root.join(nested);
        if let Ok(rd) = fs::read_dir(&sub) {
            for entry in rd.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    record_if_repo(&entry.path());
                }
            }
        }
    }

    // Inspect sibling repos in parent directory (e.g. `~/ghq/github.com/tao3k/*`).
    if let Some(parent) = active_root.parent()
        && let Ok(rd) = fs::read_dir(parent)
    {
        for entry in rd.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                record_if_repo(&entry.path());
            }
        }
    }

    let active_str = active_root.display().to_string();
    let mut list: Vec<DiscoveredProject> = found.into_values().collect();
    list.sort_by(|a, b| {
        let a_active = a.path == active_str;
        let b_active = b.path == active_str;
        b_active
            .cmp(&a_active)
            .then_with(|| cmp_ignore_ascii_case(&a.name, &b.name))
    });

    let arc = Arc::new(list);
    if let Ok(mut guard) = known_projects_cache().lock() {
        guard.insert(
            anchor.to_path_buf(),
            CachedProjectList {
                built_at: Instant::now(),
                projects: Arc::clone(&arc),
            },
        );
    }
    arc
}

fn list_known_project_records(
    raw_root: &str,
    default_root: &Path,
    force_refresh: bool,
) -> Result<Vec<GraphRecordView>, String> {
    let base = resolve_path(raw_root, default_root);
    let projects = discover_known_projects(&base, force_refresh);
    let mut records = Vec::with_capacity(projects.len());
    for proj in projects.iter() {
        let mut facets = HashMap::with_capacity(3);
        facets.insert("name".to_owned(), proj.name.clone());
        facets.insert("path".to_owned(), proj.path.clone());
        facets.insert("vcs".to_owned(), proj.vcs.to_owned());
        records.push(GraphRecordView {
            record_id: format!("fs.project:{}", proj.path),
            surface: GraphVisibleSurface::Workspace as i32,
            kind: "fs.project".to_owned(),
            title: proj.name.clone(),
            summary: proj.vcs.to_owned(),
            source_path: Some(proj.path.clone()),
            start_line: None,
            end_line: None,
            facets,
        });
    }
    Ok(records)
}

fn read_file_record(raw_path: &str, default_root: &Path) -> Result<Vec<GraphRecordView>, String> {
    if raw_path.is_empty() {
        return Err("fs:read requires a file path".to_owned());
    }
    let abs = resolve_path(raw_path, default_root);
    let bytes = fs::read(&abs).map_err(|err| format!("read file {}: {err}", abs.display()))?;
    let total_size = bytes.len();
    let truncated = total_size > MAX_READ_BYTES;
    let slice = if truncated {
        &bytes[..MAX_READ_BYTES]
    } else {
        &bytes[..]
    };
    let is_binary = slice.iter().take(4096).any(|&b| b == 0);
    let content = if is_binary {
        format!(
            "[Binary file: {} ({}) — cannot render raw bytes as text]",
            abs.display(),
            format_bytes(total_size as u64)
        )
    } else {
        String::from_utf8_lossy(slice).into_owned()
    };
    let line_count = content.lines().count();
    let rel = abs
        .strip_prefix(default_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| abs.display().to_string());

    let mut facets = HashMap::with_capacity(7);
    facets.insert("path".to_owned(), abs.display().to_string());
    facets.insert("rel_path".to_owned(), rel.clone());
    facets.insert("size".to_owned(), total_size.to_string());
    facets.insert("lines".to_owned(), line_count.to_string());
    facets.insert("truncated".to_owned(), truncated.to_string());
    facets.insert("binary".to_owned(), is_binary.to_string());
    facets.insert("content".to_owned(), content);

    Ok(vec![GraphRecordView {
        record_id: format!("fs.content:{rel}"),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "fs.content".to_owned(),
        title: abs
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.clone()),
        summary: format!("{} · {} lines", format_bytes(total_size as u64), line_count),
        source_path: Some(abs.display().to_string()),
        start_line: Some(1),
        end_line: Some(line_count as u32),
        facets,
    }])
}

fn format_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MiB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KiB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_bridge_workspace_project_index_grep_projects_and_orderless_perf() {
        let ws = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf();

        // 1. Cold VCS-native project file scan.
        let t0 = Instant::now();
        let cold = load_or_refresh_project_index(&ws, true);
        let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;
        assert!(
            !cold.is_empty(),
            "expected non-empty project file index for {}",
            ws.display()
        );

        // 2. Warm TTL cache hit.
        let t1 = Instant::now();
        let warm = load_or_refresh_project_index(&ws, false);
        let warm_us = t1.elapsed().as_secs_f64() * 1_000_000.0;
        assert_eq!(cold.len(), warm.len());

        // 3. Server-side SIMD Rust Orderless filter over the pre-lowercased index.
        let t2 = Instant::now();
        let mut last_hits = 0usize;
        for _ in 0..200 {
            let hits = filter_project_files_orderless(&warm, "ui-meow key !test", 64);
            last_hits = hits.len();
        }
        let orderless_us = (t2.elapsed().as_secs_f64() * 1_000_000.0) / 200.0;
        assert!(
            last_hits > 0,
            "expected Orderless match for `ui-meow key !test`"
        );

        // 4. Live consult-ripgrep (`fs:grep?q=...`).
        let t3 = Instant::now();
        let grep_recs = query_fs_records("fs:grep?q=query_fs_records", &ws)
            .expect("grep handled")
            .expect("grep ok");
        let grep_ms = t3.elapsed().as_secs_f64() * 1000.0;
        let grep_hits = grep_recs
            .iter()
            .filter(|r| r.kind == "fs.grep_hit")
            .count();
        assert!(grep_hits > 0, "expected live grep hits for query_fs_records");

        // 5. Project discovery (`fs:projects`).
        let t4 = Instant::now();
        let proj_recs = query_fs_records("fs:projects!", &ws)
            .expect("projects handled")
            .expect("projects ok");
        let projects_ms = t4.elapsed().as_secs_f64() * 1000.0;
        assert!(
            proj_recs.iter().any(|r| r.kind == "fs.project"),
            "expected at least one discovered Git/jj project"
        );

        eprintln!(
            "[fs_bridge bench] files={} cold_scan={:.2}ms warm_cache={:.1}µs rust_orderless={:.1}µs/op grep={}hits/{:.2}ms projects={}/{:.2}ms",
            cold.len(),
            cold_ms,
            warm_us,
            orderless_us,
            grep_hits,
            grep_ms,
            proj_recs.len(),
            projects_ms
        );
    }
}
