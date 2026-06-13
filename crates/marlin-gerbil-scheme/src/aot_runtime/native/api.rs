//! Public method implementations for native Deck runtime AOT artifact planning.

use super::{
    config::{
        GerbilDeckRuntimeNativeAotConfig, GerbilNativeCCompiler, GerbilNativeLinkLibrary,
        GerbilNativeSymbolAuditor,
    },
    paths::{
        GERBIL_DECK_RUNTIME_NATIVE_INITIALIZE_SYMBOL, GERBIL_DECK_RUNTIME_NATIVE_SELECT_SYMBOL,
        generated_link_c_source, generated_link_object, generated_loader_scm, generated_object,
        generated_runtime_scm, generated_ssi, generated_ssxi, native_output_dir,
        native_scheme_source, static_scm,
    },
    receipt::{
        GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandPlan,
        GerbilDeckRuntimeNativeAotPlan, GerbilDeckRuntimeNativeSymbol,
    },
    run::build_gerbil_deck_runtime_native_link_unit,
    status::GerbilDeckRuntimeNativeAotStatus,
};
use crate::{
    deck_runtime_native::GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH,
    runtime::{default_gerbil_gsc_program, default_gerbil_gxc_program},
};
use std::path::{Path, PathBuf};

impl GerbilDeckRuntimeNativeAotConfig {
    /// Builds a native AOT plan rooted at a writable runtime asset directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let output_dir = native_output_dir(&root);
        Self {
            root,
            output_dir,
            gxc: default_gerbil_gxc_program(),
            gsc: default_gerbil_gsc_program(),
            header: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH),
            c_compiler: None,
            symbol_auditor: GerbilNativeSymbolAuditor::new("nm"),
            gambit_link_library: GerbilNativeLinkLibrary::new("gambit"),
            gambit_link_search_dir: None,
        }
    }

    /// Overrides the native AOT output directory.
    pub fn with_output_dir(mut self, output_dir: impl Into<PathBuf>) -> Self {
        self.output_dir = output_dir.into();
        self
    }

    /// Overrides the `gxc` executable used to generate Gambit Scheme.
    pub fn with_gxc(mut self, gxc: impl Into<PathBuf>) -> Self {
        self.gxc = gxc.into();
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
        let scheme_source = native_scheme_source(&self.root);
        let generated_runtime_scm = generated_runtime_scm(&self.output_dir);
        let link_c_source = generated_link_c_source(&self.output_dir);
        let status = native_plan_status(self, &scheme_source);
        let detail = native_plan_detail(status, self, &scheme_source);

        GerbilDeckRuntimeNativeAotPlan {
            status,
            root: self.root.clone(),
            output_dir: self.output_dir.clone(),
            scheme_source: scheme_source.clone(),
            header: self.header.clone(),
            generated_loader_scm: generated_loader_scm(&self.output_dir),
            generated_runtime_scm: generated_runtime_scm.clone(),
            generated_ssi: generated_ssi(&self.output_dir),
            generated_ssxi: generated_ssxi(&self.output_dir),
            static_scm: static_scm(&self.output_dir),
            object: generated_object(&self.output_dir),
            link_c_source: link_c_source.clone(),
            link_object: generated_link_object(&self.output_dir),
            exported_symbols: vec![
                GerbilDeckRuntimeNativeSymbol::new(GERBIL_DECK_RUNTIME_NATIVE_INITIALIZE_SYMBOL),
                GerbilDeckRuntimeNativeSymbol::new(GERBIL_DECK_RUNTIME_NATIVE_SELECT_SYMBOL),
            ],
            c_compiler: self.c_compiler.clone(),
            symbol_auditor: self.symbol_auditor.clone(),
            gambit_link_library: self.gambit_link_library.clone(),
            gambit_link_search_dir: self.gambit_link_search_dir.clone(),
            gxc_generate_scheme: gxc_generate_scheme_plan(
                &self.gxc,
                &self.output_dir,
                &scheme_source,
            ),
            gsc_compile_object: gsc_compile_object_plan(
                &self.gsc,
                self.c_compiler.as_ref(),
                &generated_runtime_scm,
            ),
            gsc_generate_link_source: gsc_generate_link_source_plan(
                &self.gsc,
                self.c_compiler.as_ref(),
                &generated_runtime_scm,
            ),
            gsc_compile_link_object: gsc_compile_link_object_plan(
                &self.gsc,
                self.c_compiler.as_ref(),
                &link_c_source,
            ),
            audit_symbols: audit_symbols_plan(
                self.symbol_auditor.as_path(),
                &generated_object(&self.output_dir),
                &generated_link_object(&self.output_dir),
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
    scheme_source: &Path,
) -> GerbilDeckRuntimeNativeAotStatus {
    if !config.gxc.is_file() {
        return GerbilDeckRuntimeNativeAotStatus::MissingGxc;
    }
    if !config.gsc.is_file() {
        return GerbilDeckRuntimeNativeAotStatus::MissingGsc;
    }
    if !scheme_source.is_file() {
        return GerbilDeckRuntimeNativeAotStatus::MissingSchemeSource;
    }
    if !config.header.is_file() {
        return GerbilDeckRuntimeNativeAotStatus::MissingHeader;
    }
    GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit
}

fn native_plan_detail(
    status: GerbilDeckRuntimeNativeAotStatus,
    config: &GerbilDeckRuntimeNativeAotConfig,
    scheme_source: &Path,
) -> Option<String> {
    match status {
        GerbilDeckRuntimeNativeAotStatus::MissingGxc => Some(format!(
            "missing gxc executable at {}",
            config.gxc.display()
        )),
        GerbilDeckRuntimeNativeAotStatus::MissingGsc => Some(format!(
            "missing Gerbil Gambit compiler at {}",
            config.gsc.display()
        )),
        GerbilDeckRuntimeNativeAotStatus::MissingSchemeSource => Some(format!(
            "missing native Deck runtime Scheme source at {}",
            scheme_source.display()
        )),
        GerbilDeckRuntimeNativeAotStatus::MissingHeader => Some(format!(
            "missing native Deck runtime C ABI header at {}",
            config.header.display()
        )),
        GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit => None,
    }
}

fn gxc_generate_scheme_plan(
    gxc: &Path,
    output_dir: &Path,
    scheme_source: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    GerbilDeckRuntimeNativeAotCommandPlan {
        program: gxc.to_path_buf(),
        args: vec![
            "-d".to_string(),
            output_dir.to_string_lossy().into_owned(),
            "-target".to_string(),
            "C".to_string(),
            "-s".to_string(),
            "-S".to_string(),
            "-O".to_string(),
            scheme_source.to_string_lossy().into_owned(),
        ],
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
    generated_runtime_scm: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    let mut args = vec!["-target".to_string(), "C".to_string()];
    if let Some(c_compiler) = c_compiler {
        args.push("-cc".to_string());
        args.push(c_compiler.as_str().to_string());
    }
    args.push("-link".to_string());
    args.push(generated_runtime_scm.to_string_lossy().into_owned());

    GerbilDeckRuntimeNativeAotCommandPlan {
        program: gsc.to_path_buf(),
        args,
    }
}

fn audit_symbols_plan(
    symbol_auditor: &Path,
    object: &Path,
    link_object: &Path,
) -> GerbilDeckRuntimeNativeAotCommandPlan {
    GerbilDeckRuntimeNativeAotCommandPlan {
        program: symbol_auditor.to_path_buf(),
        args: vec![
            object.to_string_lossy().into_owned(),
            link_object.to_string_lossy().into_owned(),
        ],
    }
}
