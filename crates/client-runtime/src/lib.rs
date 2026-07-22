#![cfg_attr(test, allow(clippy::unwrap_used))]

mod attachments;
mod client;
mod inputs;
mod models;
mod password;
mod projections;
mod unlock_daemon;

pub use client::RuntimeClient;
pub use inputs::{
    ArchiveTaskArgs, CommentInput, CreateCommentArgs, CreateTaskArgs, DeleteCommentArgs,
    DeleteTaskArgs, MoveTaskArgs, MoveTaskInput, TaskCompletionArgs, TaskCreateInput,
    TaskFieldPatch, TaskUpdateInput, UnarchiveTaskArgs, UpdateCommentArgs, UpdateTaskArgs,
};
pub use models::{
    AgentAttachment, AgentComment, AgentDelegation, AgentMembership, AgentTaskDetail,
    AgentTaskSummary, AgentWorkListDetail, AgentWorkListSummary, DownloadedAttachment, ReadError,
    ReadableAttachment, ReadableAttachmentContentFormat, ReadableAttachmentSourceKind,
};
pub use unlock_daemon::{
    SessionKey, UnlockStatus, clear_session, fetch_data_key, lock, serve, session_key, socket_path,
    unlock, unlock_status,
};
