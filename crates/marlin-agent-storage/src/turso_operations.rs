use super::{
    BatchAttemptError, Duration, MemoryKey, STORAGE_SCHEMA, SessionEventRecord, StorageError,
    StorageResult, StorageSchemaSnapshot, TursoAgentStorage, TursoAgentStorageConfig,
    TursoAsyncIoMode, TursoBatchWriteReceipt, TursoConnectionPool, TursoDatabaseAuthority,
    TursoMemoryEmbeddingRecord, TursoMemorySearchMatch, TursoMemorySearchRequest,
    TursoMvccCheckpointMode, TursoMvccMode, TursoOperationalReceipt, TursoOptimizationReceipt,
    TursoSdkTelemetryStatus, TursoSyncFeatureStatus, append_session_events_transaction, drain_rows,
    execute_retrying, integer_at, is_retryable_turso_error, list_schema_tables, local_open_lock,
    map_turso_error, text_at, text_value,
};

#[cfg(feature = "turso-sync")]
use super::{TursoOptimizationProfile, TursoSyncAgentStorageConfig, TursoSyncStatsReceipt};

impl TursoAgentStorage {
    pub async fn open_local(config: TursoAgentStorageConfig) -> StorageResult<Self> {
        let open_lock = local_open_lock(&config.path);
        let _open_guard = open_lock.lock().await;
        let optimization_receipt = config.optimization_profile.receipt();
        let path = config.path.to_string_lossy().into_owned();
        let builder = match optimization_receipt.async_io {
            TursoAsyncIoMode::Enabled => turso::Builder::new_local(&path),
        };
        let database = builder
            .experimental_mvcc_passive_checkpoint(
                optimization_receipt.mvcc_checkpoint
                    == TursoMvccCheckpointMode::PassiveExperimental,
            )
            .build()
            .await
            .map_err(map_turso_error)?;
        let database = TursoDatabaseAuthority::Local(database);
        let connections =
            TursoConnectionPool::new(&database, optimization_receipt.connection_lanes).await?;
        let storage = Self {
            _database: database,
            connections,
            config,
        };
        storage.bootstrap().await?;
        Ok(storage)
    }

    #[cfg(feature = "turso-sync")]
    pub async fn open_sync(config: TursoSyncAgentStorageConfig) -> StorageResult<Self> {
        let open_lock = local_open_lock(&config.path);
        let _open_guard = open_lock.lock().await;
        let path = config.path.to_string_lossy().into_owned();
        let database = turso::sync::Builder::new_remote(&path)
            .with_remote_url(config.remote_url.as_str())
            .with_auth_token(config.auth_token.expose())
            .bootstrap_if_empty(config.bootstrap_if_empty)
            .with_logical_mvcc_pull(true)
            .build()
            .await
            .map_err(map_turso_error)?;
        let database = TursoDatabaseAuthority::Sync(database);
        let storage_config = TursoAgentStorageConfig {
            path: config.path,
            optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
            batch_transaction_mode: super::TursoBatchTransactionMode::Concurrent,
        };
        let optimization_receipt = storage_config.optimization_profile.receipt();
        let connections =
            TursoConnectionPool::new(&database, optimization_receipt.connection_lanes).await?;
        let storage = Self {
            _database: database,
            connections,
            config: storage_config,
        };
        storage.bootstrap().await?;
        Ok(storage)
    }

    #[cfg(feature = "turso-sync")]
    pub async fn sync_push(&self) -> StorageResult<()> {
        match &self._database {
            TursoDatabaseAuthority::Sync(database) => {
                database.push().await.map_err(map_turso_error)
            }
            TursoDatabaseAuthority::Local(_) => Err(StorageError::SyncNotConfigured),
        }
    }

    #[cfg(feature = "turso-sync")]
    pub async fn sync_pull(&self) -> StorageResult<bool> {
        match &self._database {
            TursoDatabaseAuthority::Sync(database) => {
                database.pull().await.map_err(map_turso_error)
            }
            TursoDatabaseAuthority::Local(_) => Err(StorageError::SyncNotConfigured),
        }
    }

    #[cfg(feature = "turso-sync")]
    pub async fn sync_checkpoint(&self) -> StorageResult<()> {
        match &self._database {
            TursoDatabaseAuthority::Sync(database) => {
                database.checkpoint().await.map_err(map_turso_error)
            }
            TursoDatabaseAuthority::Local(_) => Err(StorageError::SyncNotConfigured),
        }
    }

    #[cfg(feature = "turso-sync")]
    pub async fn sync_stats(&self) -> StorageResult<TursoSyncStatsReceipt> {
        match &self._database {
            TursoDatabaseAuthority::Sync(database) => {
                let stats = database.stats().await.map_err(map_turso_error)?;
                Ok(TursoSyncStatsReceipt {
                    network_received_bytes: stats.network_received_bytes as u64,
                    network_sent_bytes: stats.network_sent_bytes as u64,
                    main_wal_size: stats.main_wal_size,
                    revision: stats.revision,
                    last_pull_unix_time: stats.last_pull_unix_time,
                    last_push_unix_time: stats.last_push_unix_time,
                })
            }
            TursoDatabaseAuthority::Local(_) => Err(StorageError::SyncNotConfigured),
        }
    }

    pub async fn append_session_events_batch(
        &self,
        records: Vec<SessionEventRecord>,
    ) -> StorageResult<TursoBatchWriteReceipt> {
        let transaction_mode = self.config.batch_transaction_mode;
        if records.is_empty() {
            return Ok(TursoBatchWriteReceipt {
                transaction_mode,
                item_count: 0,
                rows_affected: 0,
                retry_count: 0,
            });
        }

        let connection = self.connections.acquire().await?;
        const MAX_ATTEMPTS: u32 = 8;
        let mut retry_count = 0;
        for attempt in 0..MAX_ATTEMPTS {
            match append_session_events_transaction(&connection, transaction_mode, &records).await {
                Ok(rows_affected) => {
                    return Ok(TursoBatchWriteReceipt {
                        transaction_mode,
                        item_count: records.len(),
                        rows_affected,
                        retry_count,
                    });
                }
                Err(BatchAttemptError::Domain(error)) => {
                    let _ = connection.execute("ROLLBACK", ()).await;
                    return Err(error);
                }
                Err(BatchAttemptError::Backend(error))
                    if is_retryable_turso_error(&error) && attempt + 1 < MAX_ATTEMPTS =>
                {
                    let _ = connection.execute("ROLLBACK", ()).await;
                    retry_count += 1;
                    let backoff_ms = 1_u64 << attempt.min(5);
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
                Err(BatchAttemptError::Backend(error)) => {
                    let _ = connection.execute("ROLLBACK", ()).await;
                    return Err(map_turso_error(error));
                }
            }
        }

        Err(StorageError::Backend {
            message: "Turso batch retry loop exhausted unexpectedly".to_string(),
        })
    }

    pub async fn put_memory_embedding(
        &self,
        record: TursoMemoryEmbeddingRecord,
    ) -> StorageResult<()> {
        let embedding_json = record.embedding.json()?;
        let dimension = record.embedding.dimension() as i64;
        let connection = self.connections.acquire().await?;
        execute_retrying(
            &connection,
            "INSERT INTO memory_embeddings (
                project_id,
                memory_key,
                dimension,
                embedding,
                updated_at_unix_ms
            ) VALUES (?1, ?2, ?3, vector8(?4), ?5)
            ON CONFLICT (project_id, memory_key) DO UPDATE SET
                dimension = excluded.dimension,
                embedding = excluded.embedding,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            || {
                vec![
                    text_value(record.project_id.clone()),
                    text_value(record.memory_key.clone()),
                    turso::Value::Integer(dimension),
                    turso::Value::Text(embedding_json.clone()),
                    turso::Value::Integer(record.updated_at_unix_ms),
                ]
            },
        )
        .await?;
        Ok(())
    }

    pub async fn search_memory_embeddings(
        &self,
        request: TursoMemorySearchRequest,
    ) -> StorageResult<Vec<TursoMemorySearchMatch>> {
        let embedding_json = request.embedding.json()?;
        let dimension = request.embedding.dimension() as i64;
        let connection = self.connections.acquire().await?;
        let mut statement = connection
            .prepare_cached(
                "SELECT
                    memory_key,
                    vector_distance_cos(embedding, vector8(?2)) AS distance,
                    updated_at_unix_ms
                FROM memory_embeddings
                WHERE project_id = ?1 AND dimension = ?3
                ORDER BY distance, memory_key
                LIMIT ?4",
            )
            .await
            .map_err(map_turso_error)?;
        let mut rows = statement
            .query(vec![
                text_value(request.project_id),
                turso::Value::Text(embedding_json),
                turso::Value::Integer(dimension),
                turso::Value::Integer(i64::from(request.limit.get())),
            ])
            .await
            .map_err(map_turso_error)?;
        let mut matches = Vec::new();
        while let Some(row) = rows.next().await.map_err(map_turso_error)? {
            matches.push(TursoMemorySearchMatch {
                memory_key: MemoryKey::new(text_at(&row, 0)?)?,
                cosine_distance: row.get(1).map_err(map_turso_error)?,
                updated_at_unix_ms: integer_at(&row, 2)?,
            });
        }
        Ok(matches)
    }

    pub fn config(&self) -> &TursoAgentStorageConfig {
        &self.config
    }

    pub fn optimization_receipt(&self) -> TursoOptimizationReceipt {
        self.config.optimization_profile.receipt()
    }

    pub fn operational_receipt(&self) -> TursoOperationalReceipt {
        let optimization = self.optimization_receipt();
        TursoOperationalReceipt {
            optimization,
            sync_feature: if cfg!(feature = "turso-sync") {
                TursoSyncFeatureStatus::Compiled
            } else {
                TursoSyncFeatureStatus::NotCompiled
            },
            checkpoint_mode: optimization.mvcc_checkpoint,
            checkpoint_stats: TursoSdkTelemetryStatus::NotExposedBySdk,
            database_stats: TursoSdkTelemetryStatus::NotExposedBySdk,
        }
    }

    pub async fn schema_snapshot(&self) -> StorageResult<StorageSchemaSnapshot> {
        let connection = self.connections.acquire().await?;
        Ok(StorageSchemaSnapshot {
            lifecycle: crate::records::StorageSchemaLifecycle::DevelopmentBaseline,
            tables: list_schema_tables(&connection).await?,
        })
    }

    async fn bootstrap(&self) -> StorageResult<()> {
        let connection = self.connections.acquire().await?;
        if self.config.optimization_profile.receipt().mvcc == TursoMvccMode::Required {
            drain_rows(&connection, "PRAGMA journal_mode = 'mvcc'").await?;
        }

        for statement in STORAGE_SCHEMA {
            execute_retrying(&connection, statement, Vec::new).await?;
        }
        Ok(())
    }
}
