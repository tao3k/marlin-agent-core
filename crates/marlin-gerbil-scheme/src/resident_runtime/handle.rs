//! Prepared handle receipts and process plans for resident `Gerbil Scheme` runtime sessions.

use std::io;

use crate::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilCommandProfile, gerbil_runtime_loadpath,
};

use super::{
    GerbilResidentRuntimePlan, GerbilResidentRuntimePrepareReceipt, GerbilResidentRuntimeProcess,
    GerbilResidentRuntimeProcessPlan, GerbilResidentRuntimeProcessReceipt,
    GerbilResidentRuntimeProcessStatus, GerbilResidentStrategyEventKind,
    GerbilResidentStrategyLaneStatus, GerbilResidentStrategyServicePlan,
    GerbilResidentStrategyServiceReceipt,
};

/// Prepared resident runtime handle. Process ownership is layered on top of this.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilResidentRuntimeHandle {
    pub(crate) plan: GerbilResidentRuntimePlan,
    pub(crate) written_assets: Vec<std::path::PathBuf>,
}

impl GerbilResidentRuntimeHandle {
    pub fn plan(&self) -> &GerbilResidentRuntimePlan {
        &self.plan
    }

    pub fn written_assets(&self) -> &[std::path::PathBuf] {
        self.written_assets.as_slice()
    }

    pub fn receipt(&self) -> GerbilResidentRuntimePrepareReceipt {
        GerbilResidentRuntimePrepareReceipt {
            session_mode: self.plan.session_mode.clone(),
            session_id: self.plan.session_id.clone(),
            process_reuse_required: self.plan.requires_process_reuse(),
            state_isolated: self.plan.isolates_state(),
            command_profile: self.plan.command_profile.clone(),
            loadpath_root: self.plan.loadpath_root.clone(),
            written_asset_count: self.written_assets.len(),
        }
    }

    pub fn process_plan(&self) -> GerbilResidentRuntimeProcessPlan {
        let process_reuse_required = self.plan.requires_process_reuse();
        let status = if process_reuse_required {
            GerbilResidentRuntimeProcessStatus::ReadyToSpawn
        } else {
            GerbilResidentRuntimeProcessStatus::Disabled
        };
        let command_profile =
            process_reuse_required.then(|| resident_process_command_profile(&self.plan));

        GerbilResidentRuntimeProcessPlan {
            session_mode: self.plan.session_mode.clone(),
            session_id: self.plan.session_id.clone(),
            process_reuse_required,
            state_isolated: self.plan.isolates_state(),
            status,
            command_profile,
            loadpath_root: self.plan.loadpath_root.clone(),
        }
    }

    pub fn process_receipt(&self) -> GerbilResidentRuntimeProcessReceipt {
        let process_plan = self.process_plan();
        GerbilResidentRuntimeProcessReceipt {
            session_mode: process_plan.session_mode,
            session_id: process_plan.session_id,
            process_reuse_required: process_plan.process_reuse_required,
            state_isolated: process_plan.state_isolated,
            status: process_plan.status,
            command_profile: process_plan.command_profile,
            loadpath_root: process_plan.loadpath_root,
            written_asset_count: self.written_assets.len(),
        }
    }

    pub fn strategy_service_plan(&self) -> GerbilResidentStrategyServicePlan {
        self.process_plan().strategy_service_plan()
    }

    pub fn strategy_service_receipt(&self) -> GerbilResidentStrategyServiceReceipt {
        let service_plan = self.strategy_service_plan();
        let lane_count = service_plan.lanes.len();
        let ready_lane_count = service_plan
            .lanes
            .iter()
            .filter(|lane| lane.status == GerbilResidentStrategyLaneStatus::ReadyToServe)
            .count();
        let disabled_lane_count = service_plan
            .lanes
            .iter()
            .filter(|lane| lane.status == GerbilResidentStrategyLaneStatus::Disabled)
            .count();
        let dynamic_replan_lane_count = service_plan
            .lanes
            .iter()
            .filter(|lane| lane.event_kind == GerbilResidentStrategyEventKind::DynamicReplan)
            .count();
        let policy_change_lane_count = service_plan
            .lanes
            .iter()
            .filter(|lane| lane.event_kind == GerbilResidentStrategyEventKind::PolicyChange)
            .count();

        GerbilResidentStrategyServiceReceipt {
            session_mode: service_plan.session_mode,
            session_id: service_plan.session_id,
            process_reuse_required: service_plan.process_reuse_required,
            state_isolated: service_plan.state_isolated,
            loadpath_root: service_plan.loadpath_root,
            lane_count,
            ready_lane_count,
            disabled_lane_count,
            dynamic_replan_lane_count,
            policy_change_lane_count,
            lanes: service_plan.lanes,
        }
    }

    pub fn spawn_process(&self) -> io::Result<GerbilResidentRuntimeProcess> {
        self.process_plan().spawn()
    }
}

fn resident_process_command_profile(plan: &GerbilResidentRuntimePlan) -> GerbilCommandProfile {
    let loadpath = gerbil_runtime_loadpath(&plan.loadpath_root);
    let mut profile = plan.command_profile.clone();
    profile.env.insert(
        GERBIL_LOADPATH_ENV.to_owned(),
        loadpath.to_string_lossy().into_owned(),
    );
    profile.args = vec![GERBIL_ADAPTER_MODULE.to_owned()];
    profile
}
