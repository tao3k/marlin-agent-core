//! Rust static-link planning for generated Gerbil runtime link units.

use super::{
    config::GerbilLinkLibrary,
    receipt::{GerbilAotBuildReceipt, GerbilAotSymbol},
    status::{GerbilAotBuildStatus, GerbilAotStaticLinkStatus},
};
use std::path::PathBuf;

/// Cargo directive kind needed to consume a generated native link unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilAotCargoDirectiveKind {
    RustcLinkArg,
    RustcLinkLib,
    RustcLinkSearch,
}

impl GerbilAotCargoDirectiveKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::RustcLinkArg => "rustc-link-arg",
            Self::RustcLinkLib => "rustc-link-lib",
            Self::RustcLinkSearch => "rustc-link-search",
        }
    }
}

/// One `cargo:` directive emitted by a future build-script integration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotCargoDirective {
    pub kind: GerbilAotCargoDirectiveKind,
    pub value: String,
}

impl GerbilAotCargoDirective {
    pub fn new(kind: GerbilAotCargoDirectiveKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }

    pub fn line(&self) -> String {
        format!("cargo:{}={}", self.kind.key(), self.value)
    }
}

/// Static-link inputs for a generated native Gerbil runtime link unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotStaticLinkPlan {
    pub status: GerbilAotStaticLinkStatus,
    pub module_object: PathBuf,
    pub module_objects: Vec<PathBuf>,
    pub link_object: PathBuf,
    pub header: PathBuf,
    pub required_symbols: Vec<GerbilAotSymbol>,
    pub link_libraries: Vec<GerbilLinkLibrary>,
    pub link_search_dirs: Vec<PathBuf>,
    pub cargo_directives: Vec<GerbilAotCargoDirective>,
    pub detail: Option<String>,
}

impl GerbilAotBuildReceipt {
    /// Projects an object build receipt into Rust static-link inputs.
    pub fn static_link_plan(&self) -> GerbilAotStaticLinkPlan {
        if self.status != GerbilAotBuildStatus::LinkUnitReady {
            return GerbilAotStaticLinkPlan {
                status: GerbilAotStaticLinkStatus::LinkUnitNotReady,
                module_object: self.plan.object.clone(),
                module_objects: self.plan.module_objects.clone(),
                link_object: self.plan.link_object.clone(),
                header: self.plan.header.clone(),
                required_symbols: self.plan.exported_symbols.clone(),
                link_libraries: Vec::new(),
                link_search_dirs: Vec::new(),
                cargo_directives: Vec::new(),
                detail: Some(format!(
                    "native {} link unit is not ready: {:?}",
                    self.plan.profile.label(),
                    self.status,
                )),
            };
        }

        let mut link_search_dirs = Vec::new();
        let mut cargo_directives = self
            .plan
            .module_objects
            .iter()
            .map(|object| {
                GerbilAotCargoDirective::new(
                    GerbilAotCargoDirectiveKind::RustcLinkArg,
                    object.to_string_lossy().into_owned(),
                )
            })
            .collect::<Vec<_>>();
        cargo_directives.push(GerbilAotCargoDirective::new(
            GerbilAotCargoDirectiveKind::RustcLinkArg,
            self.plan.link_object.to_string_lossy().into_owned(),
        ));
        if let Some(search_dir) = &self.plan.gambit_link_search_dir {
            link_search_dirs.push(search_dir.clone());
            cargo_directives.push(GerbilAotCargoDirective::new(
                GerbilAotCargoDirectiveKind::RustcLinkSearch,
                format!("native={}", search_dir.display()),
            ));
        }
        cargo_directives.push(GerbilAotCargoDirective::new(
            GerbilAotCargoDirectiveKind::RustcLinkLib,
            self.plan.gambit_link_library.as_str(),
        ));

        GerbilAotStaticLinkPlan {
            status: GerbilAotStaticLinkStatus::Ready,
            module_object: self.plan.object.clone(),
            module_objects: self.plan.module_objects.clone(),
            link_object: self.plan.link_object.clone(),
            header: self.plan.header.clone(),
            required_symbols: self.plan.exported_symbols.clone(),
            link_libraries: vec![self.plan.gambit_link_library.clone()],
            link_search_dirs,
            cargo_directives,
            detail: None,
        }
    }
}
