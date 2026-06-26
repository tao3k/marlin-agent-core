//! Strategy lane admission for resident `Gerbil Scheme` runtime requests.

use super::{
    GerbilResidentStrategyLaneStatus, GerbilResidentStrategyRequest,
    GerbilResidentStrategyRequestReceipt, GerbilResidentStrategyRequestStatus,
    GerbilResidentStrategyServicePlan,
};

impl GerbilResidentStrategyServicePlan {
    pub fn request_receipt(
        &self,
        request: GerbilResidentStrategyRequest,
    ) -> GerbilResidentStrategyRequestReceipt {
        let lane = self
            .lanes
            .iter()
            .find(|lane| lane.lane_id == request.lane_id);
        let status = match lane {
            None => GerbilResidentStrategyRequestStatus::LaneUnavailable,
            Some(lane) if lane.event_kind != request.event_kind => {
                GerbilResidentStrategyRequestStatus::EventLaneMismatch
            }
            Some(lane) if lane.status == GerbilResidentStrategyLaneStatus::Disabled => {
                GerbilResidentStrategyRequestStatus::RuntimeDisabled
            }
            Some(_) => GerbilResidentStrategyRequestStatus::Accepted,
        };

        GerbilResidentStrategyRequestReceipt {
            request_id: request.request_id,
            lane_id: request.lane_id,
            event_kind: request.event_kind,
            status,
            session_mode: self.session_mode.clone(),
            session_id: request.session_id.or_else(|| self.session_id.clone()),
            process_reuse_required: self.process_reuse_required,
            state_isolated: self.state_isolated,
            policy_epoch: request.policy_epoch,
            child_id: None,
            process_health: None,
        }
    }
}
