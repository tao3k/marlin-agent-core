//! Native library discovery for generated Gambit link units.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GambitLinkSearchDiscovery {
    pub(crate) search_dir: PathBuf,
    pub(crate) library_path: PathBuf,
}

pub(crate) fn find_gambit_link_search_dir_from_gsc(
    gsc: &Path,
) -> Option<GambitLinkSearchDiscovery> {
    for search_dir in gambit_link_search_dir_candidates(gsc) {
        if let Some(library_path) = find_gambit_library(&search_dir) {
            return Some(GambitLinkSearchDiscovery {
                search_dir,
                library_path,
            });
        }
    }
    None
}

fn gambit_link_search_dir_candidates(gsc: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_gsc_prefix_lib_candidate(&mut candidates, gsc);
    if let Ok(canonical_gsc) = fs::canonicalize(gsc) {
        push_gsc_prefix_lib_candidate(&mut candidates, &canonical_gsc);
    }
    push_gsc_wrapper_lib_candidates(&mut candidates, gsc);
    candidates
}

fn push_gsc_prefix_lib_candidate(candidates: &mut Vec<PathBuf>, gsc: &Path) {
    let Some(prefix) = gsc.parent().and_then(Path::parent) else {
        return;
    };
    push_unique_candidate(candidates, prefix.join("lib"));
}

fn push_gsc_wrapper_lib_candidates(candidates: &mut Vec<PathBuf>, gsc: &Path) {
    let Ok(wrapper) = fs::read_to_string(gsc) else {
        return;
    };

    for prefix in gsc_wrapper_prefixes(&wrapper) {
        push_unique_candidate(candidates, prefix.join("lib"));
    }
}

fn push_unique_candidate(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn gsc_wrapper_prefixes(wrapper: &str) -> Vec<PathBuf> {
    wrapper
        .lines()
        .map(str::trim)
        .flat_map(|line| [gerbil_home_prefix(line), exec_gsc_prefix(line)])
        .flatten()
        .collect()
}

fn gerbil_home_prefix(line: &str) -> Option<PathBuf> {
    let line = line.strip_prefix("export ").unwrap_or(line);
    let value = line.strip_prefix("GERBIL_HOME=")?;
    shell_word(value).map(PathBuf::from)
}

fn exec_gsc_prefix(line: &str) -> Option<PathBuf> {
    let value = line.strip_prefix("exec ")?;
    let program = PathBuf::from(shell_word(value)?);
    if program.file_name()? != "gsc" {
        return None;
    }
    program
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

fn shell_word(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        return rest.split('"').next().map(str::to_owned);
    }
    if let Some(rest) = value.strip_prefix('\'') {
        return rest.split('\'').next().map(str::to_owned);
    }
    value.split_whitespace().next().map(str::to_owned)
}

fn find_gambit_library(search_dir: &Path) -> Option<PathBuf> {
    for file_name in ["libgambit.a", "libgambit.dylib", "libgambit.so"] {
        let library = search_dir.join(file_name);
        if library.is_file() {
            return Some(library);
        }
    }

    let entries = fs::read_dir(search_dir).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with("libgambit.so.") && entry.path().is_file() {
            return Some(entry.path());
        }
    }

    None
}
