//! Compile glob patterns once at registration. Dispatch only matches typed paths.

use std::path::{Component, Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookTargetPath(PathBuf);

impl HookTargetPath {
    /// Require a nonempty path within the invocation's bound workspace.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, HookPathFilterError> {
        let path = path.into();
        let mut components = path.components();
        if !matches!(components.next(), Some(Component::Normal(_)))
            || !components.all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(HookPathFilterError::InvalidPath(path));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookPathPattern(String);

impl HookPathPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HookPathFilterError {
    #[error("hook target must be a workspace-relative path: {0}")]
    InvalidPath(PathBuf),
    #[error("invalid hook path patterns: {0}")]
    Glob(#[from] globset::Error),
}

#[derive(Debug)]
pub struct HookPathFilter {
    include: GlobSet,
    exclude: GlobSet,
}

impl HookPathFilter {
    /// Match any included path not excluded. Empty includes permit any path;
    /// an invocation with no target paths does not match a filter.
    pub fn compile(
        include: &[HookPathPattern],
        exclude: &[HookPathPattern],
    ) -> Result<Self, HookPathFilterError> {
        fn compile_set(patterns: &[HookPathPattern]) -> Result<GlobSet, globset::Error> {
            let mut set = GlobSetBuilder::new();
            for pattern in patterns {
                set.add(
                    GlobBuilder::new(&pattern.0)
                        .literal_separator(true)
                        .build()?,
                );
            }
            set.build()
        }
        Ok(Self {
            include: compile_set(include)?,
            exclude: compile_set(exclude)?,
        })
    }

    pub fn matches(&self, paths: &[HookTargetPath]) -> bool {
        paths.iter().any(|path| {
            let candidate = globset::Candidate::new(path.as_path());
            (self.include.is_empty() || self.include.is_match_candidate(&candidate))
                && !self.exclude.is_match_candidate(&candidate)
        })
    }
}
