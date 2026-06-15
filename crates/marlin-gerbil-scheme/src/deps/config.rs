//! Platform-aware configuration resolution for Gerbil package bootstrap.

use super::{
    GERBIL_BIN_ENV, GERBIL_CELLAR_ENV, GERBIL_GCC_ENV, GERBIL_GSC_ENV, GERBIL_MACOS_SDK_ENV,
    GERBIL_PREFIX_ENV, GerbilDepsError, MARLIN_GERBIL_PKG_CACHE_ENV,
    cli::GerbilDepsRequest,
    process::{command_stdout, command_stdout_path, find_program},
};
use std::{
    env,
    path::{Path, PathBuf},
};

/// Resolved, platform-aware Gerbil dependency bootstrap configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDepsConfig {
    pub(super) platform: String,
    pub(super) home_dir: PathBuf,
    pub(super) cache_dir: PathBuf,
    pub(super) gerbil_bin: PathBuf,
    pub(super) gerbil_cellar: Option<PathBuf>,
    pub(super) gerbil_prefix: Option<PathBuf>,
    pub(super) gerbil_gsc: Option<PathBuf>,
    pub(super) gerbil_gcc: Option<PathBuf>,
    pub(super) macos_sdk: Option<PathBuf>,
}

impl GerbilDepsConfig {
    pub(super) fn from_request(request: &GerbilDepsRequest) -> Result<Self, GerbilDepsError> {
        let platform = request
            .platform
            .clone()
            .or_else(|| env::var("MARLIN_GERBIL_DEPS_PLATFORM").ok())
            .unwrap_or_else(|| env::consts::OS.to_string());
        let home_dir = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| GerbilDepsError::message("HOME is required for Gerbil package state"))?;
        let cache_dir = request
            .cache_dir
            .clone()
            .or_else(|| env::var_os(MARLIN_GERBIL_PKG_CACHE_ENV).map(PathBuf::from))
            .or_else(|| {
                env::var_os("XDG_CACHE_HOME")
                    .map(PathBuf::from)
                    .map(|path| path.join("marlin-agent-core").join("gerbil-pkgs"))
            })
            .unwrap_or_else(|| {
                home_dir
                    .join(".cache")
                    .join("marlin-agent-core")
                    .join("gerbil-pkgs")
            });
        let gerbil_bin = request
            .gerbil_bin
            .clone()
            .or_else(|| env::var_os(GERBIL_BIN_ENV).map(PathBuf::from))
            .or_else(|| find_program("gxi").and_then(|path| path.parent().map(Path::to_path_buf)))
            .ok_or_else(|| {
                GerbilDepsError::message(format!(
                    "Gerbil bin directory is required; pass --gerbil-bin, set {GERBIL_BIN_ENV}, or put gxi on PATH"
                ))
            })?;
        let gxi = gerbil_bin.join("gxi");
        let gerbil_cellar = request
            .gerbil_cellar
            .clone()
            .or_else(|| env::var_os(GERBIL_CELLAR_ENV).map(PathBuf::from))
            .or_else(|| {
                (platform == "macos")
                    .then(|| canonical_parent(&gerbil_bin))
                    .flatten()
            });
        let gerbil_prefix = request
            .gerbil_prefix
            .clone()
            .or_else(|| env::var_os(GERBIL_PREFIX_ENV).map(PathBuf::from))
            .or_else(|| query_gerbil_home(&gxi))
            .or_else(|| {
                (platform == "macos")
                    .then(|| gerbil_cellar.as_ref().map(|cellar| cellar.join("v0.18.2")))
                    .flatten()
            });
        let macos_sdk = request
            .macos_sdk
            .clone()
            .or_else(|| env::var_os(GERBIL_MACOS_SDK_ENV).map(PathBuf::from))
            .or_else(|| (platform == "macos").then(default_macos_sdk).flatten());
        let gerbil_gsc = request
            .gerbil_gsc
            .clone()
            .or_else(|| env::var_os(GERBIL_GSC_ENV).map(PathBuf::from))
            .or_else(|| {
                (platform == "macos")
                    .then(|| {
                        gerbil_prefix
                            .as_ref()
                            .map(|prefix| prefix.join("bin").join("gsc"))
                    })
                    .flatten()
            })
            .or_else(|| find_program("gsc"))
            .or_else(|| Some(gerbil_bin.join("gsc")));
        let gerbil_gcc = request
            .gerbil_gcc
            .clone()
            .or_else(|| env::var_os(GERBIL_GCC_ENV).map(PathBuf::from))
            .or_else(|| (platform == "macos").then(default_macos_clang).flatten());

        Ok(Self {
            platform,
            home_dir,
            cache_dir,
            gerbil_bin,
            gerbil_cellar,
            gerbil_prefix,
            gerbil_gsc,
            gerbil_gcc,
            macos_sdk,
        })
    }

    /// Creates a test configuration without reading host environment defaults.
    pub fn for_test(platform: impl Into<String>, home_dir: impl Into<PathBuf>) -> Self {
        let home_dir = home_dir.into();
        let gerbil_bin = home_dir.join("gerbil").join("bin");
        Self {
            platform: platform.into(),
            cache_dir: home_dir
                .join(".cache")
                .join("marlin-agent-core")
                .join("gerbil-pkgs"),
            home_dir,
            gerbil_bin,
            gerbil_cellar: None,
            gerbil_prefix: None,
            gerbil_gsc: None,
            gerbil_gcc: None,
            macos_sdk: None,
        }
    }

    /// Returns whether this platform requires the Homebrew/Gambit layout repair.
    pub fn requires_homebrew_repair(&self) -> bool {
        self.platform == "macos"
    }

    /// Describes the resolved bootstrap plan.
    pub fn describe(&self) -> String {
        let mut lines = vec![
            format!("platform={}", self.platform),
            format!("home_dir={}", self.home_dir.display()),
            format!("cache_dir={}", self.cache_dir.display()),
            format!("gerbil_bin={}", self.gerbil_bin.display()),
        ];
        push_optional_path(&mut lines, "gerbil_cellar", &self.gerbil_cellar);
        push_optional_path(&mut lines, "gerbil_prefix", &self.gerbil_prefix);
        push_optional_path(&mut lines, "gerbil_gsc", &self.gerbil_gsc);
        push_optional_path(&mut lines, "gerbil_gcc", &self.gerbil_gcc);
        push_optional_path(&mut lines, "macos_sdk", &self.macos_sdk);
        lines.push(format!(
            "homebrew_repair={}",
            self.requires_homebrew_repair()
        ));
        lines.join("\n")
    }
}

fn push_optional_path(lines: &mut Vec<String>, key: &str, path: &Option<PathBuf>) {
    if let Some(path) = path {
        lines.push(format!("{key}={}", path.display()));
    }
}

fn canonical_parent(path: &Path) -> Option<PathBuf> {
    path.parent().and_then(|parent| parent.canonicalize().ok())
}

fn default_macos_sdk() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = command_stdout("xcrun", ["--show-sdk-path"]) {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(PathBuf::from(
        "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk",
    ));
    candidates.push(PathBuf::from(
        "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
    ));
    candidates.into_iter().find(|candidate| {
        candidate
            .join("usr")
            .join("include")
            .join("_bounds.h")
            .is_file()
            && candidate.join("usr").join("lib").join("libz.tbd").is_file()
    })
}

fn default_macos_clang() -> Option<PathBuf> {
    let clt = PathBuf::from("/Library/Developer/CommandLineTools/usr/bin/clang");
    if clt.is_file() {
        return Some(clt);
    }
    command_stdout("xcrun", ["-find", "clang"]).map(PathBuf::from)
}

fn query_gerbil_home(gxi: &Path) -> Option<PathBuf> {
    command_stdout_path(gxi, ["-e", "(display (gerbil-home))", "-e", "(newline)"])
        .map(PathBuf::from)
}
