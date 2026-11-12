// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod agent_graph_projection;
mod content_graph;
mod contracts;
mod contracts_accumulation;
mod document;
mod project_graph;
mod project_recall;
mod release;
mod session_graph;
mod tool_graph;
mod topology_graph;

pub(super) fn cases() -> impl Iterator<Item = (&'static str, fn())> {
    agent_graph_projection::CASES
        .iter()
        .copied()
        .chain(content_graph::CASES.iter().copied())
        .chain(contracts::CASES.iter().copied())
        .chain(contracts_accumulation::CASES.iter().copied())
        .chain(document::CASES.iter().copied())
        .chain(project_graph::CASES.iter().copied())
        .chain(project_recall::CASES.iter().copied())
        .chain(release::CASES.iter().copied())
        .chain(session_graph::CASES.iter().copied())
        .chain(tool_graph::CASES.iter().copied())
        .chain(topology_graph::CASES.iter().copied())
}
