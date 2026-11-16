// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{string, symbol, vector};
use gerbil_scheme::{GerbilRuntime, RootedSchemeObject};
use marlin_agent_protocol::HumanDecision;
use marlin_agent_ui::{UiToolReviewDecision, decode_review_menu, decode_tool_review_menu};

pub(super) fn assert_tool_review_projection_rejects_missing_or_duplicate_choices(
    runtime: &GerbilRuntime,
) {
    let menu = |decisions: &[&str]| {
        let entries = decisions
            .iter()
            .map(|decision| {
                vector(
                    runtime,
                    &[
                        symbol(runtime, decision),
                        string(runtime, "choice"),
                        string(runtime, "description"),
                    ],
                )
            })
            .collect::<Vec<_>>();
        vector(
            runtime,
            &[
                string(runtime, "Tool request: "),
                vector(runtime, &entries),
                string(runtime, "Inspect tool arguments"),
            ],
        )
    };
    let admitted =
        decode_tool_review_menu(menu(&["reject", "approve-once"])).expect("typed tool review menu");
    assert_eq!(admitted.choices[0].decision, UiToolReviewDecision::Reject);
    assert_eq!(
        admitted.choices[1].decision,
        UiToolReviewDecision::ApproveOnce
    );
    assert_eq!(admitted.details_heading, "Inspect tool arguments");
    assert!(decode_tool_review_menu(menu(&["approve-once"])).is_err());
    assert!(decode_tool_review_menu(menu(&["reject", "reject"])).is_err());
    assert!(decode_tool_review_menu(menu(&["approve-once", "approved"])).is_err());
}

pub(super) fn assert_review_menu_projection_requires_each_typed_decision_once(
    runtime: &GerbilRuntime,
) {
    fn menu<'a>(runtime: &'a GerbilRuntime, decisions: &[(&str, &str)]) -> RootedSchemeObject<'a> {
        let entries = decisions
            .iter()
            .map(|(decision, label)| {
                vector(
                    runtime,
                    &[
                        symbol(runtime, decision),
                        string(runtime, label),
                        string(runtime, "description"),
                    ],
                )
            })
            .collect::<Vec<_>>();
        vector(
            runtime,
            &[string(runtime, "Review: "), vector(runtime, &entries)],
        )
    }
    let valid = menu(
        runtime,
        &[
            ("patch-requested", "patch"),
            ("rejected", "deny"),
            ("approved", "approve"),
            ("needs-question", "question"),
        ],
    );
    let projected = decode_review_menu(valid).expect("typed menu");
    assert_eq!(projected.choices[0].decision, HumanDecision::PatchRequested);
    assert_eq!(projected.choices[2].decision, HumanDecision::Approved);
    assert!(
        decode_review_menu(menu(
            runtime,
            &[
                ("approved", "a"),
                ("approved", "b"),
                ("rejected", "r"),
                ("needs-question", "q"),
            ]
        ))
        .is_err()
    );
    assert!(decode_review_menu(menu(runtime, &[("approved", "a")])).is_err());
    assert!(
        decode_review_menu(menu(
            runtime,
            &[
                ("approved", "a"),
                ("rejected", "r"),
                ("needs-question", "q"),
                ("unknown", "u"),
            ]
        ))
        .is_err()
    );
}
