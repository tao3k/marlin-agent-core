use marlin_org_model::OrgNodeId;
use marlin_workspace_patch::WorkspacePatchOp;

pub(crate) fn workspace_patch_op_node(op: &WorkspacePatchOp) -> &OrgNodeId {
    match op {
        WorkspacePatchOp::SetTodo { node, .. }
        | WorkspacePatchOp::SetProperty { node, .. }
        | WorkspacePatchOp::AddCheckbox { node, .. }
        | WorkspacePatchOp::MarkCheckbox { node, .. }
        | WorkspacePatchOp::AppendSection { node, .. }
        | WorkspacePatchOp::AddLink { node, .. }
        | WorkspacePatchOp::AddEvidenceRef { node, .. }
        | WorkspacePatchOp::AddMetricPoint { node, .. }
        | WorkspacePatchOp::AddDecision { node, .. }
        | WorkspacePatchOp::AddTraceEvent { node, .. }
        | WorkspacePatchOp::MarkMemoryCandidate { node, .. } => node,
    }
}
