use std::time::Duration;

use marlin_agent_runtime::{RuntimeEdgeLayer, RuntimeEdgePolicy, RuntimeEdgePolicyError};
use tower::{BoxError, Service, ServiceExt, service_fn};

#[test]
fn runtime_edge_policy_records_tower_layers() {
    let receipt = RuntimeEdgePolicy::new()
        .with_concurrency_limit(2)
        .with_load_shed(true)
        .with_timeout_ms(250)
        .receipt()
        .expect("runtime edge policy should be valid");

    assert_eq!(receipt.concurrency_limit(), Some(2));
    assert!(receipt.load_shed());
    assert_eq!(receipt.timeout_ms(), Some(250));
    assert_eq!(
        receipt.layers(),
        &[
            RuntimeEdgeLayer::ConcurrencyLimit,
            RuntimeEdgeLayer::LoadShed,
            RuntimeEdgeLayer::Timeout,
        ]
    );
    assert_eq!(RuntimeEdgeLayer::LoadShed.as_str(), "load-shed");
}

#[test]
fn runtime_edge_policy_rejects_zero_limits() {
    assert!(matches!(
        RuntimeEdgePolicy::new().with_timeout_ms(0).receipt(),
        Err(RuntimeEdgePolicyError::ZeroTimeout)
    ));
    assert!(matches!(
        RuntimeEdgePolicy::new().with_concurrency_limit(0).receipt(),
        Err(RuntimeEdgePolicyError::ZeroConcurrencyLimit)
    ));
}

#[tokio::test]
async fn runtime_edge_policy_wraps_successful_service() {
    let policy = RuntimeEdgePolicy::new()
        .with_concurrency_limit(1)
        .with_load_shed(true)
        .with_timeout_ms(100);
    let (mut service, receipt) = policy
        .apply(service_fn(|request: &'static str| async move {
            Ok::<_, BoxError>(format!("handled:{request}"))
        }))
        .expect("runtime edge policy should wrap service");

    let response = service
        .ready()
        .await
        .expect("service should become ready")
        .call("provider")
        .await
        .expect("service should complete");

    assert_eq!(response, "handled:provider");
    assert_eq!(receipt.layers().len(), 3);
}

#[tokio::test]
async fn runtime_edge_policy_times_out_slow_service() {
    let (mut service, receipt) = RuntimeEdgePolicy::new()
        .with_timeout_ms(1)
        .apply(service_fn(|()| async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok::<_, BoxError>("late")
        }))
        .expect("timeout policy should wrap service");

    let result = service
        .ready()
        .await
        .expect("service should become ready")
        .call(())
        .await;

    assert!(result.is_err(), "slow service should time out");
    assert_eq!(receipt.layers(), &[RuntimeEdgeLayer::Timeout]);
}
