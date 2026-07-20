pub(super) const STORAGE_SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS session_events (
        project_id TEXT NOT NULL,
        session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        turn_id TEXT NOT NULL,
        event_id TEXT NOT NULL,
        event_kind TEXT NOT NULL,
        causality_parent_event_id TEXT,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, session_id, turn_id, event_id)
    )",
    "CREATE TABLE IF NOT EXISTS artifacts (
        project_id TEXT NOT NULL,
        artifact_hash TEXT NOT NULL,
        artifact_kind TEXT NOT NULL,
        producer_session_id TEXT NOT NULL,
        producer_agent_id TEXT NOT NULL,
        producer_event_id TEXT NOT NULL,
        media_type TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, artifact_hash)
    )",
    "CREATE TABLE IF NOT EXISTS artifact_pointers (
        project_id TEXT NOT NULL,
        pointer_key TEXT NOT NULL,
        target_artifact_hash TEXT NOT NULL,
        updated_by_session_id TEXT NOT NULL,
        updated_by_agent_id TEXT NOT NULL,
        updated_by_event_id TEXT NOT NULL,
        updated_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, pointer_key)
    )",
    "CREATE TABLE IF NOT EXISTS visibility_receipts (
        project_id TEXT NOT NULL,
        receipt_id TEXT NOT NULL,
        receipt_kind TEXT NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, receipt_id)
    )",
    "CREATE INDEX IF NOT EXISTS visibility_receipts_by_created
        ON visibility_receipts (project_id, created_at_unix_ms, receipt_id)",
    "CREATE TABLE IF NOT EXISTS memory_proposals (
        project_id TEXT NOT NULL,
        proposal_id TEXT NOT NULL,
        memory_key TEXT NOT NULL,
        proposal_status TEXT NOT NULL,
        source_artifact_hash TEXT NOT NULL,
        source_session_id TEXT NOT NULL,
        source_agent_id TEXT NOT NULL,
        source_event_id TEXT NOT NULL,
        org_source_path TEXT NOT NULL,
        org_source_begin INTEGER NOT NULL,
        org_source_end INTEGER NOT NULL,
        memory_kind TEXT NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, proposal_id)
    )",
    "CREATE INDEX IF NOT EXISTS memory_proposals_by_key
        ON memory_proposals (project_id, memory_key, created_at_unix_ms, proposal_id)",
    "CREATE INDEX IF NOT EXISTS memory_proposals_by_created
        ON memory_proposals (project_id, created_at_unix_ms, proposal_id)",
    "CREATE TABLE IF NOT EXISTS topology_edges (
        project_id TEXT NOT NULL,
        edge_id TEXT NOT NULL,
        from_node_id TEXT NOT NULL,
        to_node_id TEXT NOT NULL,
        edge_kind TEXT NOT NULL,
        source_session_id TEXT NOT NULL,
        source_agent_id TEXT NOT NULL,
        source_event_id TEXT NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, edge_id)
    )",
    "CREATE INDEX IF NOT EXISTS topology_edges_by_from_node
        ON topology_edges (project_id, from_node_id, created_at_unix_ms, edge_id)",
    "CREATE INDEX IF NOT EXISTS topology_edges_by_created
        ON topology_edges (project_id, created_at_unix_ms, edge_id)",
    "CREATE TABLE IF NOT EXISTS memory_embeddings (
        project_id TEXT NOT NULL,
        memory_key TEXT NOT NULL,
        dimension INTEGER NOT NULL,
        embedding BLOB NOT NULL,
        updated_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, memory_key)
    )",
    "CREATE INDEX IF NOT EXISTS memory_embeddings_by_dimension
        ON memory_embeddings (project_id, dimension, updated_at_unix_ms, memory_key)",
];
