// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_ui::grpc::v1::{
    FILE_DESCRIPTOR_SET, StartSessionTurnRequest, app_server_client::AppServerClient,
};

#[test]
fn exposes_the_generated_session_client_and_descriptor() {
    let _connect = AppServerClient::connect::<&str>;
    let request = StartSessionTurnRequest {
        session_id: "session".into(),
        user_text: "hello".into(),
        after_turn: 0,
        submission_id: "send-1".into(),
    };
    assert_eq!(request.submission_id, "send-1");
    assert!(
        FILE_DESCRIPTOR_SET
            .windows(b"marlin.app.v1".len())
            .any(|bytes| bytes == b"marlin.app.v1")
    );
}
