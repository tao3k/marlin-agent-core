// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{collections::BTreeMap, path::Path};

mod activation;
mod import;
mod refresh;
mod resolver;
mod state_storage;
mod workspace_project;

fn isolated_direnv_environment(root: &Path) -> BTreeMap<String, String> {
    let mut environment: BTreeMap<_, _> = std::env::vars().collect();
    for (name, directory) in [
        ("XDG_DATA_HOME", "direnv-data"),
        ("DIRENV_CONFIG", "direnv-config"),
    ] {
        environment.insert(name.into(), root.join(directory).display().to_string());
    }
    environment
}
