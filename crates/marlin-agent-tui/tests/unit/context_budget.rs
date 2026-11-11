use std::mem::size_of;

use marlin_agent_protocol::{ModelGatewayMessage, assistant_gateway_message, user_gateway_message};

use super::context_budget::ContextBudget;

#[test]
fn budget_accounts_full_history_prompt_and_name_without_truncating() {
    let history = vec![
        user_gateway_message("中"),
        assistant_gateway_message("ab").with_name("n"),
    ];
    let exact =
        size_of::<ModelGatewayMessage>() * 3 + "中".len() + "ab".len() + "n".len() + "x".len();
    let usage = ContextBudget {
        bytes: exact,
        turns: 2,
    }
    .measure(&history, "x")
    .unwrap();
    assert_eq!(usage.request_bytes, exact);
    assert_eq!(usage.completed_turns, 1);
    assert!(usage.allowed);
    assert!(
        !ContextBudget {
            bytes: exact - 1,
            turns: 2
        }
        .measure(&history, "x")
        .unwrap()
        .allowed
    );
    assert!(
        ContextBudget {
            bytes: exact,
            turns: 2
        }
        .allows(&history, "x")
    );
    assert!(
        !ContextBudget {
            bytes: exact - 1,
            turns: 2
        }
        .allows(&history, "x")
    );
    assert!(
        !ContextBudget {
            bytes: exact,
            turns: 1
        }
        .allows(&history, "x")
    );
    assert!(
        !ContextBudget {
            bytes: usize::MAX,
            turns: 0
        }
        .allows(&[], "x")
    );
}
