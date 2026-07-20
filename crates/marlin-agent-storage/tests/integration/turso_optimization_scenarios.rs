#![cfg(feature = "turso")]

use marlin_agent_storage::{
    TursoAgentStorage, TursoAgentStorageConfig, TursoAsyncIoMode, TursoMvccCheckpointMode,
    TursoMvccMode, TursoOptimizationProfile, TursoOptimizationReceipt,
};
use tempfile::tempdir;

#[derive(Clone, Copy, Debug)]
struct TursoOptimizationScenario {
    name: &'static str,
    optimization_profile: TursoOptimizationProfile,
    batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode,
}

impl TursoOptimizationScenario {
    fn receipt(self) -> TursoOptimizationReceipt {
        self.optimization_profile.receipt()
    }
}

#[tokio::test]
async fn turso_0_7_optimization_scenarios_are_explicit_and_receiptable() {
    let root = tempdir().expect("create Turso scenario directory");
    let scenarios = [
        TursoOptimizationScenario {
            name: "compatibility-baseline",
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Immediate,
        },
        TursoOptimizationScenario {
            name: "mvcc",
            optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
            batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
        },
        TursoOptimizationScenario {
            name: "mvcc-passive-checkpoint",
            optimization_profile:
                TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
            batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
        },
    ];

    for scenario in scenarios {
        let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: root.path().join(format!("{}.db", scenario.name)),
            optimization_profile: scenario.optimization_profile,
            batch_transaction_mode: scenario.batch_transaction_mode,
        })
        .await
        .unwrap_or_else(|error| panic!("scenario {} failed to open: {error}", scenario.name));

        assert_eq!(
            storage.optimization_receipt(),
            scenario.receipt(),
            "scenario {} must report the exact applied optimization profile",
            scenario.name
        );
    }
}

#[test]
fn turso_0_7_maximized_profile_enables_every_exposed_optimization() {
    assert_eq!(
        TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental.receipt(),
        TursoOptimizationReceipt {
            async_io: TursoAsyncIoMode::Enabled,
            mvcc: TursoMvccMode::Required,
            mvcc_checkpoint: TursoMvccCheckpointMode::PassiveExperimental,
            connection_lanes: 4,
            statement_cache:
                marlin_agent_storage::TursoStatementCacheMode::PreparedCachedPerConnection,
        }
    );
}
