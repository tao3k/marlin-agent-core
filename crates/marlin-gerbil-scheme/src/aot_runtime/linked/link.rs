//! Rust static-link planning for generated Gerbil runtime link units.

use super::{
    config::GerbilLinkLibrary,
    receipt::{GerbilDeckRouteAbiSymbol, GerbilDeckRuntimeAotBuildReceipt},
    status::{GerbilDeckRouteAbiStaticLinkStatus, GerbilDeckRuntimeAotBuildStatus},
};
use std::path::PathBuf;

/// Cargo directive kind needed to consume a generated native link unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilDeckRouteAbiCargoDirectiveKind {
    RustcLinkArg,
    RustcLinkLib,
    RustcLinkSearch,
}

impl GerbilDeckRouteAbiCargoDirectiveKind {
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
pub struct GerbilDeckRouteAbiCargoDirective {
    pub kind: GerbilDeckRouteAbiCargoDirectiveKind,
    pub value: String,
}

impl GerbilDeckRouteAbiCargoDirective {
    pub fn new(kind: GerbilDeckRouteAbiCargoDirectiveKind, value: impl Into<String>) -> Self {
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
pub struct GerbilDeckRouteAbiStaticLinkPlan {
    pub status: GerbilDeckRouteAbiStaticLinkStatus,
    pub module_object: PathBuf,
    pub module_objects: Vec<PathBuf>,
    pub link_object: PathBuf,
    pub header: PathBuf,
    pub required_symbols: Vec<GerbilDeckRouteAbiSymbol>,
    pub link_libraries: Vec<GerbilLinkLibrary>,
    pub link_search_dirs: Vec<PathBuf>,
    pub cargo_directives: Vec<GerbilDeckRouteAbiCargoDirective>,
    pub detail: Option<String>,
}

impl GerbilDeckRuntimeAotBuildReceipt {
    /// Projects an object build receipt into Rust static-link inputs.
    pub fn static_link_plan(&self) -> GerbilDeckRouteAbiStaticLinkPlan {
        if self.status != GerbilDeckRuntimeAotBuildStatus::LinkUnitReady {
            return GerbilDeckRouteAbiStaticLinkPlan {
                status: GerbilDeckRouteAbiStaticLinkStatus::LinkUnitNotReady,
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
                GerbilDeckRouteAbiCargoDirective::new(
                    GerbilDeckRouteAbiCargoDirectiveKind::RustcLinkArg,
                    object.to_string_lossy().into_owned(),
                )
            })
            .collect::<Vec<_>>();
        cargo_directives.push(GerbilDeckRouteAbiCargoDirective::new(
            GerbilDeckRouteAbiCargoDirectiveKind::RustcLinkArg,
            self.plan.link_object.to_string_lossy().into_owned(),
        ));
        if let Some(search_dir) = &self.plan.gambit_link_search_dir {
            link_search_dirs.push(search_dir.clone());
            cargo_directives.push(GerbilDeckRouteAbiCargoDirective::new(
                GerbilDeckRouteAbiCargoDirectiveKind::RustcLinkSearch,
                format!("native={}", search_dir.display()),
            ));
        }
        cargo_directives.push(GerbilDeckRouteAbiCargoDirective::new(
            GerbilDeckRouteAbiCargoDirectiveKind::RustcLinkLib,
            self.plan.gambit_link_library.as_str(),
        ));

        GerbilDeckRouteAbiStaticLinkPlan {
            status: GerbilDeckRouteAbiStaticLinkStatus::Ready,
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
