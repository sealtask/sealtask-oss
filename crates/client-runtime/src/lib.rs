//! Runtime services for SealTask clients.
//!
//! Operations using `password_stdin` own process stdin until EOF. Their reader
//! runs outside Tokio so a one-shot CLI can exit promptly on cancellation. If a
//! caller drops such a future while stdin is pending, it must terminate the
//! process: the reader may still consume stdin until EOF. Long-lived hosts must
//! complete stdin-based unlocking before running cancellable resource operations.

#![cfg_attr(test, allow(clippy::unwrap_used))]

mod agent_grants;
mod attachment_files;
mod attachment_mutations;
mod attachment_reconciliation;
mod attachment_rendering;
mod attachment_transfer;
mod attachments;
mod blocking_crypto;
mod client;
mod inputs;
mod models;
mod operation_cancellation;
mod password;
mod projections;
mod read_cache;
mod reconciliation;
mod storage;
mod unlock_daemon;
mod upload_lifecycle;

pub use agent_grants::{PrepareAgentApprovalGrant, PreparedAgentApprovalGrant};
pub use client::{
    PreparedTaskCreate, PreparedTaskUpdate, ProjectTaskSession, RuntimeClient, TaskMutationPlan,
};
pub use inputs::{
    ArchiveTaskArgs, AttachmentUploadPassword, CommentInput, CreateCommentArgs, CreateNoteArgs,
    CreateTaskArgs, DeleteCommentArgs, DeleteNoteArgs, DeleteTaskArgs, DeleteTaskAttachmentArgs,
    MoveTaskArgs, MoveTaskInput, NoteCreateInput, NoteUpdateInput,
    QuarantineTaskReferenceSchemeArgs, RepairTaskReferenceSchemeArgs, TaskCompletionArgs,
    TaskCreateIdempotencyDerivation, TaskCreateInput, TaskFieldPatch, TaskUpdateInput,
    UnarchiveTaskArgs, UpdateCommentArgs, UpdateNoteArgs, UpdateTaskArgs, UploadTaskAttachmentArgs,
};
pub use models::{
    AgentAttachment, AgentComment, AgentDelegation, AgentMembership, AgentNote, AgentTaskDetail,
    AgentTaskReferenceHistoryStatus, AgentTaskReferenceSchemeStatus, AgentTaskSummary,
    AgentWorkListDetail, AgentWorkListSummary, DownloadedAttachment, ReadError, ReadableAttachment,
    ReadableAttachmentContentFormat, ReadableAttachmentSourceKind,
    TaskReferenceHistoryAvailability,
};
pub use operation_cancellation::OperationCancellation;
pub use read_cache::{
    ReadCacheMode, ReadCacheNotice, ReadCacheOptions, ReadCacheSnapshot, ReadCacheStatus,
    ReadCacheVerification,
};
pub use unlock_daemon::{
    SessionKey, UnlockStatus, clear_session, fetch_data_key, lock, serve, session_key, socket_path,
    unlock, unlock_status,
};
pub use upload_lifecycle::AttachmentUploadFailureReport;
