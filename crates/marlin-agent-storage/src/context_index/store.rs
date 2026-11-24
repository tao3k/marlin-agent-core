// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::Path;

use marlin_agent_gitops::{
    GitBlobId, GitBlobLocator, GitCommitId, GitRepositoryPath, RepositoryInstanceId,
};
use mrr_data_backend::turso_driver as turso;
use tokio::sync::Mutex;
use turso::transaction::TransactionBehavior;

use super::model::{
    ContextCut, ContextCutChange, ContextIndexRead, ContextIndexUpdate, ContextLocator,
    ContextLocatorKey, ContextOwnerRevision,
};
use crate::{StorageError, StorageResult};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS context_owner_cursors (
    repo TEXT PRIMARY KEY, cursor INTEGER NOT NULL, frontier INTEGER NOT NULL,
    receipt TEXT NOT NULL, CHECK (cursor >= 0 AND cursor <= frontier)
);
CREATE TABLE IF NOT EXISTS context_locators (
    repo TEXT NOT NULL, cut TEXT NOT NULL, path TEXT NOT NULL, tip TEXT NOT NULL,
    blob TEXT NOT NULL, generation INTEGER NOT NULL CHECK (generation BETWEEN 1 AND 65534),
    PRIMARY KEY (repo, cut, path)
);
CREATE TABLE IF NOT EXISTS context_retired_cuts (
    repo TEXT NOT NULL, cut TEXT NOT NULL, tip TEXT NOT NULL, reason TEXT NOT NULL,
    owner_sequence INTEGER NOT NULL,
    PRIMARY KEY (repo, cut, owner_sequence)
)";

/// Derived locator storage for one home or DEV scope. Source owners hold the bytes.
pub struct TursoContextIndex {
    connection: Mutex<turso::Connection>,
}

#[derive(Default)]
struct Cursor {
    applied: i64,
    frontier: i64,
    receipt: String,
}

impl TursoContextIndex {
    pub async fn open(path: &Path) -> StorageResult<Self> {
        let path = path.to_str().ok_or(StorageError::InvalidIdentifier {
            kind: "Context index path",
        })?;
        let database = turso::Builder::new_local(path)
            .build()
            .await
            .map_err(backend)?;
        let connection = database.connect().map_err(backend)?;
        connection.execute_batch(SCHEMA).await.map_err(backend)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Return metadata for one already visible owner. Authorize the actual read separately.
    pub async fn lookup(
        &self,
        owner: &RepositoryInstanceId,
        key: &ContextLocatorKey,
        path: &GitRepositoryPath,
    ) -> StorageResult<ContextIndexRead> {
        let connection = self.connection.lock().await;
        let mut statement = connection
            .prepare_cached(
                "SELECT c.cursor = c.frontier, l.tip, l.blob, l.generation
             FROM context_owner_cursors c LEFT JOIN context_locators l
             ON l.repo = c.repo AND l.cut = ?2 AND l.path = ?3 WHERE c.repo = ?1",
            )
            .await
            .map_err(backend)?;
        let mut rows = statement
            .query((owner.as_str(), key.as_str(), path.as_str()))
            .await
            .map_err(backend)?;
        let Some(row) = rows.next().await.map_err(backend)? else {
            return Ok(ContextIndexRead::Incomplete);
        };
        if row.get::<i64>(0).map_err(backend)? == 0 {
            return Ok(ContextIndexRead::Incomplete);
        }
        let Some(tip) = row.get::<Option<String>>(1).map_err(backend)? else {
            return Ok(ContextIndexRead::Absent);
        };
        Ok(ContextIndexRead::Current(ContextLocator {
            generation: u16::try_from(row.get::<i64>(3).map_err(backend)?).map_err(backend)?,
            git: GitBlobLocator {
                commit: GitCommitId::parse(&tip).map_err(backend)?,
                path: path.clone(),
                blob: GitBlobId::parse(&row.get::<String>(2).map_err(backend)?).map_err(backend)?,
            },
        }))
    }

    /// Apply one authenticated owner change and its cursor in a single transaction.
    pub async fn apply(
        &self,
        owner: &RepositoryInstanceId,
        revision: &ContextOwnerRevision,
        change: ContextCutChange<'_>,
    ) -> StorageResult<ContextIndexUpdate> {
        if let ContextCutChange::Publish(cut) = change {
            cut.validate()?;
        }
        let mut connection = self.connection.lock().await;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .await
            .map_err(backend)?;
        let repo = owner.as_str();
        let cursor = read_cursor(&transaction, repo).await?;
        let result = apply_change(&transaction, repo, cursor, revision, change).await?;
        transaction.commit().await.map_err(backend)?;
        Ok(result)
    }

    /// Rebuild one owner from a complete authoritative inventory at or above the observed frontier.
    pub async fn rebuild(
        &self,
        owner: &RepositoryInstanceId,
        revision: &ContextOwnerRevision,
        cuts: &[ContextCut],
    ) -> StorageResult<ContextIndexUpdate> {
        validate_inventory(cuts)?;
        let mut connection = self.connection.lock().await;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .await
            .map_err(backend)?;
        let repo = owner.as_str();
        let cursor = read_cursor(&transaction, repo).await?;
        if revision.sequence < cursor.frontier {
            transaction.rollback().await.map_err(backend)?;
            return Ok(ContextIndexUpdate::Superseded);
        }
        if revision.sequence == cursor.applied && revision.receipt != cursor.receipt {
            transaction.rollback().await.map_err(backend)?;
            return Ok(ContextIndexUpdate::Conflict);
        }
        transaction
            .execute("DELETE FROM context_locators WHERE repo = ?1", (repo,))
            .await
            .map_err(backend)?;
        for cut in cuts {
            write_cut(&transaction, repo, cut).await?;
        }
        write_cursor(
            &transaction,
            repo,
            revision.sequence,
            revision.sequence,
            &revision.receipt,
        )
        .await?;
        transaction.commit().await.map_err(backend)?;
        Ok(ContextIndexUpdate::Applied)
    }
}

async fn apply_change(
    connection: &turso::Connection,
    repo: &str,
    cursor: Cursor,
    revision: &ContextOwnerRevision,
    change: ContextCutChange<'_>,
) -> StorageResult<ContextIndexUpdate> {
    let seq = revision.sequence;
    if seq < cursor.applied {
        return Ok(ContextIndexUpdate::Superseded);
    }
    if seq == cursor.applied {
        return Ok(if cursor.receipt == revision.receipt {
            ContextIndexUpdate::Replayed
        } else {
            ContextIndexUpdate::Conflict
        });
    }
    let frontier = cursor.frontier.max(seq);
    if seq != cursor.applied + 1 {
        write_cursor(connection, repo, cursor.applied, frontier, &cursor.receipt).await?;
        return Ok(ContextIndexUpdate::Gap {
            cursor: cursor.applied,
            observed: frontier,
        });
    }
    match change {
        ContextCutChange::Publish(cut) => write_cut(connection, repo, cut).await?,
        ContextCutChange::Retire {
            key,
            expected_tip,
            reason,
        } => {
            let affected = connection
                .execute(
                    "DELETE FROM context_locators WHERE repo = ?1 AND cut = ?2 AND tip = ?3",
                    (repo, key.as_str(), expected_tip.as_str()),
                )
                .await
                .map_err(backend)?;
            if affected == 0 {
                return Ok(ContextIndexUpdate::Conflict);
            }
            connection
                .execute(
                    "INSERT INTO context_retired_cuts VALUES (?1, ?2, ?3, ?4, ?5)",
                    (
                        repo,
                        key.as_str(),
                        expected_tip.as_str(),
                        reason.as_str(),
                        seq,
                    ),
                )
                .await
                .map_err(backend)?;
        }
    }
    write_cursor(connection, repo, seq, frontier, &revision.receipt).await?;
    Ok(ContextIndexUpdate::Applied)
}

async fn read_cursor(connection: &turso::Connection, repo: &str) -> StorageResult<Cursor> {
    let mut rows = connection
        .query(
            "SELECT cursor, frontier, receipt FROM context_owner_cursors WHERE repo = ?1",
            (repo,),
        )
        .await
        .map_err(backend)?;
    let Some(row) = rows.next().await.map_err(backend)? else {
        return Ok(Cursor::default());
    };
    Ok(Cursor {
        applied: row.get(0).map_err(backend)?,
        frontier: row.get(1).map_err(backend)?,
        receipt: row.get(2).map_err(backend)?,
    })
}

async fn write_cursor(
    connection: &turso::Connection,
    repo: &str,
    applied: i64,
    frontier: i64,
    receipt: &str,
) -> StorageResult<()> {
    connection
        .execute(
            "INSERT INTO context_owner_cursors VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (repo) DO UPDATE SET cursor = excluded.cursor,
             frontier = excluded.frontier, receipt = excluded.receipt",
            (repo, applied, frontier, receipt),
        )
        .await
        .map_err(backend)?;
    Ok(())
}

async fn write_cut(
    connection: &turso::Connection,
    repo: &str,
    cut: &ContextCut,
) -> StorageResult<()> {
    connection
        .execute(
            "DELETE FROM context_locators WHERE repo = ?1 AND cut = ?2",
            (repo, cut.key.as_str()),
        )
        .await
        .map_err(backend)?;
    let mut statement = connection
        .prepare_cached("INSERT INTO context_locators VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .await
        .map_err(backend)?;
    for entry in &cut.entries {
        statement
            .execute((
                repo,
                cut.key.as_str(),
                entry.git.path.as_str(),
                entry.git.commit.as_str(),
                entry.git.blob.as_str(),
                i64::from(entry.generation),
            ))
            .await
            .map_err(backend)?;
    }
    Ok(())
}

fn validate_inventory(cuts: &[ContextCut]) -> StorageResult<()> {
    let mut keys = std::collections::BTreeSet::new();
    for cut in cuts {
        cut.validate()?;
        if !keys.insert(cut.key.as_str()) {
            return Err(StorageError::InvalidIdentifier {
                kind: "duplicate Context cut",
            });
        }
    }
    Ok(())
}

fn backend(error: impl std::fmt::Display) -> StorageError {
    StorageError::Backend {
        message: error.to_string(),
    }
}
