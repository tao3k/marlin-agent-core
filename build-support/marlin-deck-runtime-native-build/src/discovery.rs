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
    candidates
}

fn push_gsc_prefix_lib_candidate(candidates: &mut Vec<PathBuf>, gsc: &Path) {
    let Some(prefix) = gsc.parent().and_then(Path::parent) else {
        return;
    };
    let candidate = prefix.join("lib");
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
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
