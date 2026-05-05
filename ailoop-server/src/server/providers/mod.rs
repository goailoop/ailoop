//! Communication providers: notification sinks and reply sources
//!
//! **Pending prompt timeout**: Default is 300 seconds (see `DEFAULT_PROMPT_TIMEOUT_SECS` in
//! `pending_prompt`). Used when a message does not specify a timeout.
//!
//! **Invalid provider reply**: Unparseable or invalid replies from a provider (e.g. gibberish
//! for yes/no) are treated as: authorization/navigation -> deny; question -> empty or error.
//! See FR-010 in spec and `infer_response_type` in `telegram`.

mod pending_prompt;
mod reply_source;
mod sink;
mod telegram;

pub use pending_prompt::{
    PendingPromptCompleter, PendingPromptRegistry, PromptType, RecvTimeoutError,
    DEFAULT_PROMPT_TIMEOUT_SECS,
};
pub use reply_source::{ProviderReply, ReplySource};
pub use sink::NotificationSink;
pub use telegram::{TelegramReplySource, TelegramSink};
