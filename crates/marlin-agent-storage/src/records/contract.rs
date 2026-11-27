// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use super::pagination::{
    MemoryProposalCursor, MemoryProposalPageRequest, SessionEventCursor, SessionEventPageRequest,
    SessionPageRequest, StoragePage, TopologyEdgeCursor, TopologyEdgePageRequest, VisibilityCursor,
    VisibilityPageRequest,
};

pub type StorageResult<T> = Result<T, StorageError>;
pub type StorageFuture<'a, T> = Pin<Box<dyn Future<Output = StorageResult<T>> + Send + 'a>>;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> StorageResult<Self> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(StorageError::InvalidIdentifier {
                        kind: stringify!($name),
                    });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = StorageError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(ProjectId);
string_id!(SessionId);
string_id!(AgentId);
string_id!(TurnId);
string_id!(EventId);
string_id!(ArtifactHash);
string_id!(ArtifactPointerKey);
string_id!(MemoryKey);
string_id!(MemoryProposalId);
string_id!(TopologyEdgeId);
string_id!(TopologyNodeId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionEventKey {
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub event_id: EventId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventRecord {
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub agent_id: AgentId,
    pub turn_id: TurnId,
    pub event_id: EventId,
    pub event_kind: String,
    pub causality_parent_event_id: Option<EventId>,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

/// Identity and kind of a committed event without its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEventHeader {
    pub agent_id: AgentId,
    pub cursor: SessionEventCursor,
    pub event_kind: String,
    pub causality_parent_event_id: Option<EventId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventBatchWriteReceipt {
    pub item_count: usize,
    pub rows_affected: u64,
}

/// Number of committed events in a project-scoped session.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionEventCount(u64);

impl SessionEventCount {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn get(self) -> u64 {
        self.0
    }
}

impl SessionEventRecord {
    pub fn key(&self) -> SessionEventKey {
        SessionEventKey {
            project_id: self.project_id.clone(),
            session_id: self.session_id.clone(),
            turn_id: self.turn_id.clone(),
            event_id: self.event_id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub project_id: ProjectId,
    pub artifact_hash: ArtifactHash,
    pub artifact_kind: String,
    pub producer_session_id: SessionId,
    pub producer_agent_id: AgentId,
    pub producer_event_id: EventId,
    pub media_type: String,
    pub size_bytes: u64,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPutOutcome {
    pub inserted: bool,
    pub artifact: ArtifactRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPointerRecord {
    pub project_id: ProjectId,
    pub pointer_key: ArtifactPointerKey,
    pub target_artifact_hash: ArtifactHash,
    pub updated_by_session_id: SessionId,
    pub updated_by_agent_id: AgentId,
    pub updated_by_event_id: EventId,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPointerUpdate {
    pub project_id: ProjectId,
    pub pointer_key: ArtifactPointerKey,
    pub expected_artifact_hash: Option<ArtifactHash>,
    pub new_artifact_hash: ArtifactHash,
    pub updated_by_session_id: SessionId,
    pub updated_by_agent_id: AgentId,
    pub updated_by_event_id: EventId,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityReceipt {
    pub project_id: ProjectId,
    pub receipt_id: String,
    pub receipt_kind: String,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalRecord {
    pub project_id: ProjectId,
    pub proposal_id: MemoryProposalId,
    pub memory_key: MemoryKey,
    pub proposal_status: String,
    pub source_artifact_hash: ArtifactHash,
    pub source_session_id: SessionId,
    pub source_agent_id: AgentId,
    pub source_event_id: EventId,
    pub org_source_path: String,
    pub org_source_begin: u32,
    pub org_source_end: u32,
    pub memory_kind: String,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdgeRecord {
    pub project_id: ProjectId,
    pub edge_id: TopologyEdgeId,
    pub from_node_id: TopologyNodeId,
    pub to_node_id: TopologyNodeId,
    pub edge_kind: String,
    pub source_session_id: SessionId,
    pub source_agent_id: AgentId,
    pub source_event_id: EventId,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSchemaSnapshot {
    pub lifecycle: StorageSchemaLifecycle,
    pub tables: Vec<StorageSchemaTable>,
}

impl StorageSchemaSnapshot {
    pub fn has_table(&self, table_name: &str) -> bool {
        self.tables.iter().any(|table| table.name == table_name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageSchemaLifecycle {
    DevelopmentBaseline,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSchemaTable {
    pub name: String,
}

/// Ordered, atomic session event persistence. Implementations may own only
/// session records without providing artifact, memory or topology storage.
pub trait SessionEventStore: Send + Sync {
    /// Seek to the last event in the half-open turn range for one Session.
    fn latest_session_event_header<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
        from_turn: &'a TurnId,
        before_turn: &'a TurnId,
    ) -> StorageFuture<'a, Option<SessionEventHeader>>;

    /// Counts committed session events. Under the append-only storage contract,
    /// a changed count reveals an append between two completed reads. This is
    /// not a transaction, a lease, or a cross-replica consistency guarantee.
    fn count_session_events<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
    ) -> StorageFuture<'a, SessionEventCount>;

    fn list_sessions_page<'a>(
        &'a self,
        request: SessionPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionId, SessionId>>;

    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()>;

    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, SessionEventBatchWriteReceipt>;

    /// For a byte-bounded request, report the last returned cursor when
    /// another record remains. An oversized first body returns an error.
    fn list_session_events_page<'a>(
        &'a self,
        request: SessionEventPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionEventRecord, SessionEventCursor>>;
}

/// Complete agent storage, composed from the session event store and the
/// separate artifact, visibility, memory and topology operations.
pub trait AgentStorage: SessionEventStore {
    fn put_artifact<'a>(&'a self, record: ArtifactRecord) -> StorageFuture<'a, ArtifactPutOutcome>;

    fn get_artifact<'a>(
        &'a self,
        project_id: &'a ProjectId,
        artifact_hash: &'a ArtifactHash,
    ) -> StorageFuture<'a, Option<ArtifactRecord>>;

    fn compare_and_swap_artifact_pointer<'a>(
        &'a self,
        update: ArtifactPointerUpdate,
    ) -> StorageFuture<'a, ArtifactPointerRecord>;

    fn get_artifact_pointer<'a>(
        &'a self,
        project_id: &'a ProjectId,
        pointer_key: &'a ArtifactPointerKey,
    ) -> StorageFuture<'a, Option<ArtifactPointerRecord>>;

    fn record_visibility<'a>(&'a self, receipt: VisibilityReceipt) -> StorageFuture<'a, ()>;

    fn list_visibility_page<'a>(
        &'a self,
        request: VisibilityPageRequest,
    ) -> StorageFuture<'a, StoragePage<VisibilityReceipt, VisibilityCursor>>;

    fn put_memory_proposal<'a>(&'a self, record: MemoryProposalRecord) -> StorageFuture<'a, ()>;

    fn list_memory_proposals_page<'a>(
        &'a self,
        request: MemoryProposalPageRequest,
    ) -> StorageFuture<'a, StoragePage<MemoryProposalRecord, MemoryProposalCursor>>;

    fn append_topology_edge<'a>(&'a self, record: TopologyEdgeRecord) -> StorageFuture<'a, ()>;

    fn list_topology_edges_page<'a>(
        &'a self,
        request: TopologyEdgePageRequest,
    ) -> StorageFuture<'a, StoragePage<TopologyEdgeRecord, TopologyEdgeCursor>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    InvalidIdentifier {
        kind: &'static str,
    },
    InvalidEmbedding {
        reason: &'static str,
    },
    InvalidMemorySearchLimit {
        limit: u32,
    },
    InvalidPageLimit {
        limit: u16,
    },
    MissingTurnPageCursor,
    SessionEventBodyExceedsPageBudget {
        budget: u32,
    },
    InvalidSyncConfiguration {
        reason: &'static str,
    },
    SyncNotConfigured,
    DuplicateSessionEvent {
        key: SessionEventKey,
    },
    ArtifactHashCollision {
        project_id: ProjectId,
        artifact_hash: ArtifactHash,
    },
    MissingArtifact {
        project_id: ProjectId,
        artifact_hash: ArtifactHash,
    },
    ArtifactPointerConflict {
        project_id: ProjectId,
        pointer_key: ArtifactPointerKey,
        expected: Option<ArtifactHash>,
        actual: Option<ArtifactHash>,
    },
    Backend {
        message: String,
    },
    DuplicateMemoryProposal {
        project_id: ProjectId,
        proposal_id: MemoryProposalId,
    },
    DuplicateTopologyEdge {
        project_id: ProjectId,
        edge_id: TopologyEdgeId,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::InvalidIdentifier { kind } => {
                write!(f, "invalid empty storage identifier: {kind}")
            }
            StorageError::InvalidEmbedding { reason } => {
                write!(f, "invalid memory embedding: {reason}")
            }
            StorageError::InvalidMemorySearchLimit { limit } => {
                write!(
                    f,
                    "memory vector search limit must be between 1 and 100: {limit}"
                )
            }
            StorageError::InvalidPageLimit { limit } => {
                write!(f, "storage page limit must be between 1 and 1000: {limit}")
            }
            StorageError::MissingTurnPageCursor => {
                f.write_str("turn-scoped Session event page requires an exclusive cursor")
            }
            StorageError::SessionEventBodyExceedsPageBudget { budget } => {
                write!(
                    f,
                    "Session event body exceeds the {budget}-byte page budget"
                )
            }
            StorageError::InvalidSyncConfiguration { reason } => {
                write!(f, "invalid Turso sync configuration: {reason}")
            }
            StorageError::SyncNotConfigured => {
                f.write_str("Turso sync operation requires a sync-backed storage authority")
            }
            StorageError::DuplicateSessionEvent { key } => {
                write!(
                    f,
                    "duplicate session event {}/{}/{}/{}",
                    key.project_id, key.session_id, key.turn_id, key.event_id
                )
            }
            StorageError::ArtifactHashCollision {
                project_id,
                artifact_hash,
            } => {
                write!(
                    f,
                    "artifact hash collision in project {project_id}: {artifact_hash}"
                )
            }
            StorageError::MissingArtifact {
                project_id,
                artifact_hash,
            } => {
                write!(
                    f,
                    "artifact {artifact_hash} is missing in project {project_id}"
                )
            }
            StorageError::ArtifactPointerConflict {
                project_id,
                pointer_key,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "artifact pointer conflict in project {project_id} for {pointer_key}: expected {:?}, actual {:?}",
                    expected.as_ref().map(ArtifactHash::as_str),
                    actual.as_ref().map(ArtifactHash::as_str)
                )
            }
            StorageError::Backend { message } => f.write_str(message),
            StorageError::DuplicateMemoryProposal {
                project_id,
                proposal_id,
            } => {
                write!(
                    f,
                    "duplicate memory proposal in project {project_id}: {proposal_id}"
                )
            }
            StorageError::DuplicateTopologyEdge {
                project_id,
                edge_id,
            } => {
                write!(
                    f,
                    "duplicate topology edge in project {project_id}: {edge_id}"
                )
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// Validate artifact byte-length parity, SQLite signed-integer bounds, and
/// cryptographic `sha256:<64-hex>` content-addressable digest integrity (SM31).
pub(crate) fn validate_artifact_record(record: &ArtifactRecord) -> StorageResult<()> {
    let actual_len = record.body.len() as u64;
    if record.size_bytes != actual_len {
        return Err(StorageError::Backend {
            message: format!(
                "artifact size_bytes ({}) does not match body length ({actual_len})",
                record.size_bytes
            ),
        });
    }
    if i64::try_from(record.size_bytes).is_err() {
        return Err(StorageError::Backend {
            message: format!("artifact size_bytes exceeds i64 range: {}", record.size_bytes),
        });
    }
    if let Some(hex) = record.artifact_hash.as_str().strip_prefix("sha256:")
        && hex.len() == 64
        && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        let expected = decode_hex_32(hex).ok_or_else(|| StorageError::ArtifactHashCollision {
            project_id: record.project_id.clone(),
            artifact_hash: record.artifact_hash.clone(),
        })?;
        let computed = sha256_bytes(&record.body);
        if !constant_time_eq_32(&expected, &computed) {
            return Err(StorageError::ArtifactHashCollision {
                project_id: record.project_id.clone(),
                artifact_hash: record.artifact_hash.clone(),
            });
        }
    }
    Ok(())
}

fn constant_time_eq_32(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= left[i] ^ right[i];
    }
    std::hint::black_box(diff) == 0
}

fn decode_hex_32(hex: &str) -> Option<[u8; 32]> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = hex_nibble(bytes[2 * i])?;
        let lo = hex_nibble(bytes[2 * i + 1])?;
        *slot = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// FIPS 180-4 SHA-256 for zero-dependency content-addressed artifact verification.
fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((data.len() + 9 + 63) & !63);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let off = i * 4;
            *word = u32::from_be_bytes([
                chunk[off],
                chunk[off + 1],
                chunk[off + 2],
                chunk[off + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut digest = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}
