//! Public method implementations for native Gerbil runtime AOT artifact planning.

use super::{
    config::{
        GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeAotProfile, GerbilNativeCCompiler,
        GerbilNativeLinkLibrary, GerbilNativeSymbolAuditor,
    },
    paths::{
        compiled_runtime_link_c_source, compiled_runtime_link_object, compiled_runtime_object,
        default_compiled_runtime_scm, native_output_dir,
    },
    receipt::{
        GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandPlan,
        GerbilDeckRuntimeNativeAotPlan, GerbilDeckRuntimeNativeSymbol,
    },
    run::build_gerbil_deck_runtime_native_link_unit,
    status::GerbilDeckRuntimeNativeAotStatus,
};
use crate::runtime::{default_gerbil_gsc_program, resolve_gerbil_executable};
use std::path::{Path, PathBuf};

impl GerbilDeckRuntimeNativeAotConfig {
    /// Builds a native AOT plan rooted at a writable runtime asset directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::new_for_profile(root, GerbilDeckRuntimeNativeAotProfile::DeckRuntime)
    }

    /// Builds a native AOT plan for an explicit Gerbil native ABI profile.
    pub fn new_for_profile(
        root: impl Into<PathBuf>,
        profile: GerbilDeckRuntimeNativeAotProfile,
    ) -> Self {
        let root = root.into();
        let output_dir = native_output_dir(&root);
        let compiled_runtime_dependency_scms = profile
            .dependency_artifact_stems()
            .iter()
            .map(|stem| default_compiled_runtime_scm(&output_dir, stem))
            .collect();
        Self {
            profile,
            root,
            compiled_runtime_scm: default_compiled_runtime_scm(
                &output_dir,
                profile.artifact_stem(),
            ),
            compiled_runtime_dependency_scms,
            output_dir,
            gsc: default_gerbil_gsc_program(),
            header: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(profile.header_path()),
            c_compiler: None,
            symbol_auditor: GerbilNativeSymbolAuditor::new("nm"),
            gambit_link_library: GerbilNativeLinkLibrary::new("gambit"),
            gambit_link_search_dir: None,
        }
    }

    /// Builds a native AOT plan for the AgentGraph policy-routing C ABI profile.
    pub fn agent_policy_routing(root: impl Into<PathBuf>) -> Self {
        Self::new_for_profile(root, GerbilDeckRuntimeNativeAotProfile::AgentPolicyRouting)
    }

    /// Overrides the native AOT output directory.
    pub fn with_output_dir(mut self, output_dir: impl Into<PathBuf>) -> Self {
        let output_dir = output_dir.into();
        self.compiled_runtime_scm =
            default_compiled_runtime_scm(&output_dir, self.profile.artifact_stem());
        self.compiled_runtime_dependency_scms = self
            .profile
            .dependency_artifact_stems()
            .iter()
            .map(|stem| default_compiled_runtime_scm(&output_dir, stem))
            .collect();
        self.output_dir = output_dir;
        self
    }

    /// Overrides the compiled Gambit Scheme input consumed by the native ABI builder.
    pub fn with_compiled_runtime_scm(mut self, compiled_runtime_scm: impl Into<PathBuf>) -> Self {
        self.compiled_runtime_scm = compiled_runtime_scm.into();
        self
    }

    /// Overrides additional compiled Gambit Scheme modules required by this native ABI profile.
    pub fn with_compiled_runtime_dependency_scms(
        mut self,
        compiled_runtime_dependency_scms: impl IntoIterator<Item = impl Into<PathBuf>>,
    ) -> Self {
        self.compiled_runtime_dependency_scms = compiled_runtime_dependency_scms
            .into_iter()
            .map(Into::into)
            .collect();
        self
    }

    /// Overrides the `gsc` executable used to compile the generated Scheme object.
    pub fn with_gsc(mut self, gsc: impl Into<PathBuf>) -> Self {
        self.gsc = gsc.into();
        self
    }

    /// Overrides the header path exposed to Rust build/link consumers.
    pub fn with_header(mut self, header: impl Into<PathBuf>) -> Self {
        self.header = header.into();
        self
    }

    /// Selects a C compiler for the Gambit object phase, for example `clang`.
    pub fn with_c_compiler(mut self, c_compiler: impl Into<String>) -> Self {
        self.c_compiler = Some(GerbilNativeCCompiler::new(c_compiler));
        self
    }

    /// Overrides the symbol table auditor used to verify native ABI exports.
    pub fn with_symbol_auditor(mut self, symbol_auditor: impl Into<PathBuf>) -> Self {
        self.symbol_auditor = GerbilNativeSymbolAuditor::new(symbol_auditor);
        self
    }

    /// Overrides the Gambit runtime library name used by static link plans.
    pub fn with_gambit_link_library(mut self, link_library: impl Into<String>) -> Self {
        self.gambit_link_library = GerbilNativeLinkLibrary::new(link_library);
        self
    }

    /// Adds the native library search directory used by static link plans.
    pub fn with_gambit_link_search_dir(mut self, link_search_dir: impl Into<PathBuf>) -> Self {
        self.gambit_link_search_dir = Some(link_search_dir.into());
        self
    }

    /// Produces a typed, auditable native AOT link-unit build plan.
    pub fn plan(&self) -> GerbilDeckRuntimeNativeAotPlan {
        let object = compiled_runtime_object(&self.compiled_runtime_scm);
        let dependency_objects = self
            .compiled_runtime_dependency_scms
            .iter()
            .map(|source| compiled_runtime_object(source))
            .collect::<Vec<_>>();
        let mut module_objects = dependency_objects.clone();
        module_objects.push(object.clone());
        let link_c_source = compiled_runtime_link_c_source(&self.compiled_runtime_scm);
        let link_object = compiled_runtime_link_object(&self.compiled_runtime_scm);
        let status = native_plan_status(self);
        let detail = native_plan_detail(status, self);

        GerbilDeckRuntimeNativeAotPlan {
            profile: self.profile,
            status,
            root: self.root.clone(),
            output_dir: self.output_dir.clone(),
            compiled_runtime_scm: self.compiled_runtime_scm.clone(),
            compiled_runtime_dependency_scms: self.compiled_runtime_dependency_scms.clone(),
            header: self.header.clone(),
            object: object.clone(),
            dependency_objects: dependency_objects.clone(),
            module_objects: module_objects.clone(),
            link_c_source: link_c_source.clone(),
            link_object: link_object.clone(),
            exported_symbols: vec![
                GerbilDeckRuntimeNativeSymbol::new(self.profile.initialize_symbol()),
                GerbilDeckRuntimeNativeSymbol::new(self.profile.select_symbol()),
            ],
            c_compiler: self.c_compiler.clone(),
            symbol_auditor: self.symbol_auditor.clone(),
            gambit_link_library: self.gambit_link_library.clone(),
            gambit_link_search_dir: self.gambit_link_search_dir.clone(),
            gsc_compile_object: gsc_compile_object_plan(
                &self.gsc,
                self.c_compiler.as_ref(),
                &self.compiled_runtime_scm,
            ),
            gsc_compile_dependency_objects: self
                .compiled_runtime_dependency_scms
                .iter()
                .map(|source| gsc_compile_object_plan(&self.gsc, self.c_compiler.as_ref(), source))
                .collect(),
            gsc_generate_link_source: gsc_generate_link_source_plan(
                &self.gsc,
                self.c_compiler.as_ref(),
                &self.compiled_runtime_dependency_scms,
                &self.compiled_runtime_scm,
            ),
            gsc_compile_link_object: gsc_compile_link_object_plan(
                &self.gsc,
                self.c_compiler.as_ref(),
                &link_c_source,
            ),
            audit_symbols: audit_symbols_plan(
                self.symbol_auditor.as_path(),
                &module_objects,
                &link_object,
            ),
            detail,
        }
    }

    /// Executes the native AOT link-unit build and returns a typed receipt.
    pub fn build_link_unit(&self) -> GerbilDeckRuntimeNativeAotBuildReceipt {
        build_gerbil_deck_runtime_native_link_unit(self)
    }
}

fn native_plan_status(
    config: &GerbilDeckRuntimeNativeAotConfig,
) -> GerbilDeckRuntimeNativeAotStatus {
    if resolve_gerbil_executable(&config.gsc).is_none() {
        return GerbilDeckRuntimeNativeAotStatus::MissingGsc;
    }
    if !config.compiled_runtime_scm.is_file() {
        return GerbilDeckRuntimeNativeAotStatus::MissingCompiledRuntime;
    }
    if config
        .compiled_runtime_dependency_scms
        .iter()
        .any(|source| !source.is_file())
    {
        return GerbilDeckRuntimeNativeAotStatus::MissingCompiledRuntime;
    }
    if !config.header.is_file() {
        return GerbilDeckRuntimeNativeAotStatus::MissingHeader;
    }
    GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit
}

fn native_plan_detail(
    status: GerbilDeckRuntimeNativeAotStatus,
    config: &GerbilDeckRuntimeNativeAotConfig,
) -> Option<String> {
    match status {
        GerbilDeckRuntimeNativeAotStatus::MissingGsc => Some(format!(
            "missing Gerbil Gambit compiler at {}",
            config.gsc.display()
        )),
        GerbilDeckRuntimeNativeAotStatus::MissingCompiledRuntime => {
            let missing = std::iter::once(&config.compiled_runtime_scm)
                .chain(config.compiled_runtime_dependency_scms.iter())
                .filter(|source| !source.is_file())
                .map(|source| source.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!(
                "missing compiled native {} Scheme artifact at {}",
                config.profile.label(),
                missing
            ))
        }
        GerbilDeckRuntimeNativeAotStatus::MissingHeader => Some(format!(
            "missing native {} C ABI header at {}",
            config.profile.label(),
            config.header.display()
        )),
        GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit => None,
    }
}

fn gsc_compile_object_plan(
    gsc: &Path,
    c_compiler: Option<&GerbilNativeCCompiler>,
    source: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    let mut args = vec!["-target".to_string(), "C".to_string()];
    if let Some(c_compiler) = c_compiler {
        args.push("-cc".to_string());
        args.push(c_compiler.as_str().to_string());
    }
    args.push("-obj".to_string());
    args.push(source.to_string_lossy().into_owned());

    GerbilDeckRuntimeNativeAotCommandPlan {
        program: gsc.to_path_buf(),
        args,
    }
}

fn gsc_compile_link_object_plan(
    gsc: &Path,
    c_compiler: Option<&GerbilNativeCCompiler>,
    source: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    let mut args = vec!["-target".to_string(), "C".to_string()];
    if let Some(c_compiler) = c_compiler {
        args.push("-cc".to_string());
        args.push(c_compiler.as_str().to_string());
    }
    args.push("-cc-options".to_string());
    args.push("-D___LIBRARY".to_string());
    args.push("-obj".to_string());
    args.push(source.to_string_lossy().into_owned());

    GerbilDeckRuntimeNativeAotCommandPlan {
        program: gsc.to_path_buf(),
        args,
    }
}

fn gsc_generate_link_source_plan(
    gsc: &Path,
    c_compiler: Option<&GerbilNativeCCompiler>,
    dependency_sources: &[PathBuf],
    generated_runtime_scm: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    let mut args = vec!["-target".to_string(), "C".to_string()];
    if let Some(c_compiler) = c_compiler {
        args.push("-cc".to_string());
        args.push(c_compiler.as_str().to_string());
    }
    args.push("-link".to_string());
    args.extend(
        dependency_sources
            .iter()
            .map(|source| source.to_string_lossy().into_owned()),
    );
    args.push(generated_runtime_scm.to_string_lossy().into_owned());

    GerbilDeckRuntimeNativeAotCommandPlan {
        program: gsc.to_path_buf(),
        args,
    }
}

fn audit_symbols_plan(
    symbol_auditor: &Path,
    module_objects: &[PathBuf],
    link_object: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    GerbilDeckRuntimeNativeAotCommandPlan {
        program: symbol_auditor.to_path_buf(),
        args: module_objects
            .iter()
            .map(PathBuf::as_path)
            .chain(std::iter::once(link_object))
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    }
}
