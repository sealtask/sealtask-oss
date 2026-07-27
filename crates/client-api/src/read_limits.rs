use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind};

use crate::{
    CommentResponse, MyTasksResponse, TaskDetailResponse, TaskListResponse, WorkListDetailResponse,
    WorkListResponse,
};

pub const MAX_SMALL_RESPONSE_BYTES: usize = 256 * 1024;
pub const MAX_DETAIL_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_COLLECTION_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

pub const MAX_WORK_LISTS: usize = 1_000;
pub const MAX_SECTIONS_PER_WORK_LIST: usize = 128;
pub const MAX_MEMBERS_PER_WORK_LIST: usize = 1_000;
pub const MAX_TASKS: usize = 10_000;
pub const MAX_COMMENTS: usize = 10_000;
pub const MAX_MY_TASK_PAGE_ITEMS: usize = 100;
pub const MAX_MY_TASK_PAGES: usize = 100;
pub const MAX_MY_TASKS: usize = 10_000;
pub const MAX_MY_TASK_COLLECTION_BYTES: usize = MAX_COLLECTION_RESPONSE_BYTES;

pub(crate) fn validate_work_lists(work_lists: &[WorkListResponse]) -> PublicResult<()> {
    validate_count(
        work_lists.len(),
        MAX_WORK_LISTS,
        "API work-list collection contains too many work lists",
    )?;
    for work_list in work_lists {
        validate_sections(work_list)?;
    }
    Ok(())
}

pub(crate) fn validate_work_list_detail(work_list: &WorkListDetailResponse) -> PublicResult<()> {
    validate_sections(&work_list.work_list)?;
    validate_count(
        work_list.members.len(),
        MAX_MEMBERS_PER_WORK_LIST,
        "API work-list response contains too many members",
    )
}

pub(crate) fn validate_task_list(tasks: &TaskListResponse) -> PublicResult<()> {
    validate_count(
        tasks.tasks.len(),
        MAX_TASKS,
        "API task collection contains too many tasks",
    )
}

pub(crate) fn validate_task_detail(task: &TaskDetailResponse) -> PublicResult<()> {
    validate_comments(&task.comments)
}

pub(crate) fn validate_comments(comments: &[CommentResponse]) -> PublicResult<()> {
    validate_count(
        comments.len(),
        MAX_COMMENTS,
        "API comment collection contains too many comments",
    )
}

pub(crate) fn validate_my_tasks_page(
    page: &MyTasksResponse,
    requested_limit: Option<i64>,
) -> PublicResult<()> {
    let returned_end = page
        .offset
        .checked_add(i64::try_from(page.tasks.len()).unwrap_or(i64::MAX));
    if page.limit <= 0
        || page.limit > MAX_MY_TASK_PAGE_ITEMS as i64
        || page.total < 0
        || page.offset < 0
        || page.offset > page.total
        || page.tasks.len() > MAX_MY_TASK_PAGE_ITEMS
        || page.tasks.len() > page.limit as usize
        || page.total > MAX_MY_TASKS as i64
        || requested_limit
            .is_some_and(|limit| page.limit > limit || page.tasks.len() > limit as usize)
        || returned_end.is_none_or(|returned_end| returned_end > page.total)
    {
        return Err(schema_error(
            "API /me/tasks response contains invalid or excessive pagination metadata",
        ));
    }
    Ok(())
}

pub(crate) fn validate_my_tasks_request(
    limit: Option<i64>,
    offset: Option<i64>,
) -> PublicResult<()> {
    if limit.is_some_and(|limit| !(1..=MAX_MY_TASK_PAGE_ITEMS as i64).contains(&limit)) {
        return Err(PublicError::validation(format!(
            "/me/tasks limit must be between 1 and {MAX_MY_TASK_PAGE_ITEMS}"
        )));
    }
    if offset.is_some_and(|offset| offset < 0) {
        return Err(PublicError::validation(
            "/me/tasks offset must be zero or greater",
        ));
    }
    Ok(())
}

fn validate_sections(work_list: &WorkListResponse) -> PublicResult<()> {
    validate_count(
        work_list.section_snapshots.len(),
        MAX_SECTIONS_PER_WORK_LIST,
        "API work-list response contains too many sections",
    )
}

fn validate_count(count: usize, maximum: usize, message: &'static str) -> PublicResult<()> {
    if count > maximum {
        return Err(schema_error(message));
    }
    Ok(())
}

fn schema_error(message: &'static str) -> PublicError {
    PublicError::response(ResponseFailureKind::JsonSchema, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_count_bounds_accept_exact_limit_and_reject_one_more() {
        for (maximum, message) in [
            (MAX_WORK_LISTS, "work lists"),
            (MAX_SECTIONS_PER_WORK_LIST, "sections"),
            (MAX_MEMBERS_PER_WORK_LIST, "members"),
            (MAX_TASKS, "tasks"),
            (MAX_COMMENTS, "comments"),
        ] {
            validate_count(maximum, maximum, message).expect("exact limit");
            let error =
                validate_count(maximum + 1, maximum, message).expect_err("one item over the limit");
            assert_eq!(
                error.response_failure_kind(),
                Some(ResponseFailureKind::JsonSchema)
            );
        }
    }

    #[test]
    fn my_task_request_bounds_are_exact() {
        validate_my_tasks_request(Some(MAX_MY_TASK_PAGE_ITEMS as i64), Some(0))
            .expect("exact page limit");
        assert!(
            validate_my_tasks_request(Some(MAX_MY_TASK_PAGE_ITEMS as i64 + 1), Some(0)).is_err()
        );
        assert!(validate_my_tasks_request(Some(1), Some(-1)).is_err());
    }
}
