#![cfg_attr(test, allow(clippy::unwrap_used))]

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
mod reconciliation;
mod storage;
mod unlock_daemon;
mod upload_lifecycle;

pub use client::RuntimeClient;
pub use inputs::{
    ArchiveTaskArgs, AttachmentUploadPassword, CommentInput, CreateCommentArgs, CreateNoteArgs,
    CreateTaskArgs, DeleteCommentArgs, DeleteNoteArgs, DeleteTaskArgs, DeleteTaskAttachmentArgs,
    MoveTaskArgs, MoveTaskInput, NoteCreateInput, NoteUpdateInput,
    QuarantineTaskReferenceSchemeArgs, RepairTaskReferenceSchemeArgs, TaskCompletionArgs,
    TaskCreateInput, TaskFieldPatch, TaskUpdateInput, UnarchiveTaskArgs, UpdateCommentArgs,
    UpdateNoteArgs, UpdateTaskArgs, UploadTaskAttachmentArgs,
};
pub use models::{
    AgentAttachment, AgentComment, AgentDelegation, AgentMembership, AgentNote, AgentTaskDetail,
    AgentTaskReferenceHistoryStatus, AgentTaskReferenceSchemeStatus, AgentTaskSummary,
    AgentWorkListDetail, AgentWorkListSummary, DownloadedAttachment, ReadError, ReadableAttachment,
    ReadableAttachmentContentFormat, ReadableAttachmentSourceKind,
    TaskReferenceHistoryAvailability,
};
pub use operation_cancellation::OperationCancellation;
pub use unlock_daemon::{
    SessionKey, UnlockStatus, clear_session, fetch_data_key, lock, serve, session_key, socket_path,
    unlock, unlock_status,
};
pub use upload_lifecycle::AttachmentUploadFailureReport;
