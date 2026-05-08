//! Communication providers: notification sinks and reply sources
//!
//! **Pending prompt timeout**: `DEFAULT_PROMPT_TIMEOUT_SECS` (300 s) is retained for reference
//! but is no longer the runtime fallback. Effective timeout is resolved by
//! `resolve_effective_timeout`: message field → env var `AILOOP_DEFAULT_PROMPT_TIMEOUT_SECS`
//! → `Configuration.timeout_seconds` → `None` (infinite wait).
//!
//! **Invalid provider reply**: Unparseable or invalid replies from a provider (e.g. gibberish
//! for yes/no) are treated as: authorization/navigation -> deny; question -> empty or error.
//! See FR-010 in spec and `infer_response_type` in `telegram`.

mod pending_prompt;
mod reply_source;
mod sink;
mod telegram;

pub use pending_prompt::{
    resolve_effective_timeout, PendingPromptCompleter, PendingPromptRegistry, PromptType,
    RecvTimeoutError, DEFAULT_PROMPT_TIMEOUT_SECS,
};
pub use reply_source::{ProviderReply, ReplySource};
pub use sink::NotificationSink;
pub use telegram::{TelegramReplySource, TelegramSink};
