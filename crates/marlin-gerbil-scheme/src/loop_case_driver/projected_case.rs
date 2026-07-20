//! Public facade for loading Scheme-owned loop cases as Rust LoopProgram values.

use std::{
    error::Error,
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    process::Command,
};

use marlin_agent_protocol::LoopProgram;

use super::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverProfileRef, GerbilLoopCaseDriverVerticalTraceReceipt,
    project_gerbil_loop_case_driver_loop_program, verify_gerbil_loop_case_driver_vertical_trace,
};
use crate::default_gerbil_gxi_program;

/// Request for a Scheme config-interface loop case projected into Rust types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilLoopCaseDriverProjectedLoopProgramRequest {
    gerbil_root: PathBuf,
    expected_vertical_trace_count: usize,
    case_id: GerbilLoopCaseDriverCaseId,
    loop_program_id: GerbilLoopCaseDriverLoopProgramId,
    profile_ref: Option<GerbilLoopCaseDriverProfileRef>,
    live_llm_required: Option<bool>,
    required_capabilities: Vec<GerbilLoopCaseDriverCapability>,
}

impl GerbilLoopCaseDriverProjectedLoopProgramRequest {
    #[must_use]
    pub fn new(
        case_id: impl Into<GerbilLoopCaseDriverCaseId>,
        loop_program_id: impl Into<GerbilLoopCaseDriverLoopProgramId>,
    ) -> Self {
        Self {
            gerbil_root: default_gerbil_config_interface_root(),
            expected_vertical_trace_count: 7,
            case_id: case_id.into(),
            loop_program_id: loop_program_id.into(),
            profile_ref: None,
            live_llm_required: None,
            required_capabilities: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_gerbil_root(mut self, gerbil_root: impl Into<PathBuf>) -> Self {
        self.gerbil_root = gerbil_root.into();
        self
    }

    #[must_use]
    pub fn with_expected_vertical_trace_count(mut self, count: usize) -> Self {
        self.expected_vertical_trace_count = count;
        self
    }

    #[must_use]
    pub fn with_profile_ref(
        mut self,
        profile_ref: impl Into<GerbilLoopCaseDriverProfileRef>,
    ) -> Self {
        self.profile_ref = Some(profile_ref.into());
        self
    }

    #[must_use]
    pub fn with_live_llm_required(mut self, required: bool) -> Self {
        self.live_llm_required = Some(required);
        self
    }

    #[must_use]
    pub fn with_required_capability(
        mut self,
        capability: impl Into<GerbilLoopCaseDriverCapability>,
    ) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }

    #[must_use]
    pub fn case_id(&self) -> &GerbilLoopCaseDriverCaseId {
        &self.case_id
    }

    #[must_use]
    pub fn loop_program_id(&self) -> &GerbilLoopCaseDriverLoopProgramId {
        &self.loop_program_id
    }
}

/// A verified Scheme loop case and its Rust LoopProgram projection.
#[derive(Clone, Debug)]
pub struct GerbilLoopCaseDriverProjectedLoopProgram {
    receipt: GerbilLoopCaseDriverVerticalTraceReceipt,
    loop_program: LoopProgram,
}

impl GerbilLoopCaseDriverProjectedLoopProgram {
    #[must_use]
    pub fn receipt(&self) -> &GerbilLoopCaseDriverVerticalTraceReceipt {
        &self.receipt
    }

    #[must_use]
    pub fn loop_program(&self) -> &LoopProgram {
        &self.loop_program
    }

    #[must_use]
    pub fn into_parts(self) -> (GerbilLoopCaseDriverVerticalTraceReceipt, LoopProgram) {
        (self.receipt, self.loop_program)
    }
}

/// Error returned by the Scheme loop case projection facade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilLoopCaseDriverProjectedLoopProgramError {
    message: String,
}

impl GerbilLoopCaseDriverProjectedLoopProgramError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GerbilLoopCaseDriverProjectedLoopProgramError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GerbilLoopCaseDriverProjectedLoopProgramError {}

/// Gerbil package root for the config-interface cases shipped by this crate.
#[must_use]
pub fn default_gerbil_config_interface_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("gerbil")
}

/// Run the Scheme config-interface case driver and return its receipt stdout.
pub fn run_gerbil_config_interface_case_driver_smoke()
-> Result<String, GerbilLoopCaseDriverProjectedLoopProgramError> {
    run_gerbil_config_interface_case_driver_smoke_in(default_gerbil_config_interface_root())
}

/// Run the Scheme config-interface case driver under a specific Gerbil root.
pub fn run_gerbil_config_interface_case_driver_smoke_in(
    gerbil_root: impl AsRef<Path>,
) -> Result<String, GerbilLoopCaseDriverProjectedLoopProgramError> {
    let gerbil_root = gerbil_root.as_ref();
    let gxi = default_gerbil_gxi_program();
    let loadpath = gerbil_config_interface_loadpath_with_src(gerbil_root);

    let output = Command::new(&gxi)
        .current_dir(gerbil_root)
        .env("GERBIL_LOADPATH", loadpath)
        .arg("t/config-interface-case-driver-test.ss")
        .output()
        .map_err(|error| {
            GerbilLoopCaseDriverProjectedLoopProgramError::new(format!("run {:?}: {error}", gxi))
        })?;

    if !output.status.success() {
        return Err(GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "gxi case-driver smoke failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    String::from_utf8(output.stdout).map_err(|error| {
        GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "gxi case-driver stdout should be utf8: {error}"
        ))
    })
}

/// Load one Scheme-owned loop case as a verified receipt plus LoopProgram.
pub fn load_gerbil_loop_case_driver_projected_loop_program(
    request: &GerbilLoopCaseDriverProjectedLoopProgramRequest,
) -> Result<GerbilLoopCaseDriverProjectedLoopProgram, GerbilLoopCaseDriverProjectedLoopProgramError>
{
    let stdout = run_gerbil_config_interface_case_driver_smoke_in(&request.gerbil_root)?;
    let receipt = verify_gerbil_loop_case_driver_vertical_trace(
        &stdout,
        request.expected_vertical_trace_count,
    )
    .map_err(|error| {
        GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "config-interface vertical trace should verify: {error}"
        ))
    })?
    .into_iter()
    .find(|receipt| {
        receipt.case_id() == &request.case_id
            && receipt.loop_program_id() == &request.loop_program_id
    })
    .ok_or_else(|| {
        GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "Scheme vertical trace should include case={} loop_program={}",
            request.case_id, request.loop_program_id
        ))
    })?;

    if let Some(expected_profile_ref) = request.profile_ref.as_ref()
        && receipt.profile_ref() != expected_profile_ref
    {
        return Err(GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "Scheme vertical trace case={} expected profile_ref={} but got {}",
            request.case_id,
            expected_profile_ref,
            receipt.profile_ref()
        )));
    }

    if let Some(expected_live_llm_required) = request.live_llm_required
        && receipt.live_llm_required() != expected_live_llm_required
    {
        return Err(GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "Scheme vertical trace case={} expected live_llm_required={} but got {}",
            request.case_id,
            expected_live_llm_required,
            receipt.live_llm_required()
        )));
    }

    for capability in &request.required_capabilities {
        if !receipt.has_capability(capability) {
            return Err(GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
                "Scheme vertical trace case={} is missing required capability {}",
                request.case_id, capability
            )));
        }
    }

    let loop_program = project_gerbil_loop_case_driver_loop_program(&receipt).map_err(|error| {
        GerbilLoopCaseDriverProjectedLoopProgramError::new(format!(
            "Scheme vertical trace should project into a LoopProgram: {error}"
        ))
    })?;

    Ok(GerbilLoopCaseDriverProjectedLoopProgram {
        receipt,
        loop_program,
    })
}

/// `GERBIL_LOADPATH` with package sources and local `gxpkg` dependencies first.
#[must_use]
pub fn gerbil_config_interface_loadpath_with_src(gerbil_root: &Path) -> OsString {
    let mut paths = vec![
        gerbil_root.join("src"),
        gerbil_root.join(".gerbil").join("lib"),
    ];
    if let Some(existing) = std::env::var_os("GERBIL_LOADPATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("join Gerbil loadpath")
}
