// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
#![cfg(feature = "org-workspace")]

use marlin_agent_context::org::workspace::{OrgDocument, OrgDocumentLoader};
use marlin_workspace_protocol::WorkspaceError;

#[test]
fn workspace_requires_host_startup_without_implicitly_initializing() {
    // This separate test process deliberately never admits the parser runtime.
    let document = OrgDocument::new("doc:startup", "* TODO Host startup\n");
    for result in [
        OrgDocumentLoader::load(&document).map(|_| ()),
        OrgDocumentLoader::discover_contract_documents(&[document]).map(|_| ()),
    ] {
        match result {
            Err(WorkspaceError::Backend(message)) => {
                assert!(message.contains("not initialized"), "{message}");
            }
            other => panic!("expected explicit startup error, got {other:?}"),
        }
    }
}
