//! Session identity, context visibility, and isolation contracts for agent runtimes.

mod context;
mod id;
mod identity;

pub use context::{
    ContextExpansionPolicy, ContextNamespace, ContextVisibility, SessionIsolationPolicy,
};
pub use id::{SessionId, SessionIdError, SessionKind};
pub use identity::{AgentSessionContext, SessionIdentity, SessionIsolationReceipt};
