use super::RuntimeClient;
use crate::models::{AgentWorkListDetail, AgentWorkListSummary};
use crate::read_cache::ReadCacheQuery;
use sealtask_client_api::{WorkListDetailResponse, WorkListResponse};
use sealtask_client_core::PublicResult;
use uuid::Uuid;

impl RuntimeClient {
    pub async fn list_work_lists(
        &self,
        password_stdin: bool,
    ) -> PublicResult<Vec<AgentWorkListSummary>> {
        self.list_work_lists_with_archived(password_stdin, false)
            .await
    }

    pub async fn list_work_lists_with_archived(
        &self,
        password_stdin: bool,
        include_archived: bool,
    ) -> PublicResult<Vec<AgentWorkListSummary>> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt work lists.",
            )
            .await?;
        let query = ReadCacheQuery::WorkLists { include_archived };
        let lists: Vec<WorkListResponse> = if self.is_offline() {
            self.read_cache
                .read_offline(&credentials, &data_key, &query)?
        } else if let Some(cached) = self.read_cache.memoized(&credentials, &query)? {
            cached
        } else {
            let cache_guard = self.read_cache.begin_online_read(&credentials)?;
            let mut client = self.api_client_with_credentials(credentials.clone())?;
            let lists = client
                .list_work_lists_with_archived(include_archived)
                .await?;
            self.read_cache
                .record_online(cache_guard.as_ref(), &data_key, &query, &lists)?;
            lists
        };
        Ok(lists
            .into_iter()
            .map(|list| self.project_work_list_summary(list, Some(&data_key)))
            .collect())
    }

    pub async fn archive_work_list(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentWorkListSummary> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt archived work list data.",
            )
            .await?;
        let mut client = self.api_client_with_credentials(credentials)?;
        let result = client.archive_work_list(work_list_id).await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let work_list = result?;
        Ok(self.project_work_list_summary(work_list, Some(&data_key)))
    }

    pub async fn unarchive_work_list(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentWorkListSummary> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt restored work list data.",
            )
            .await?;
        let mut client = self.api_client_with_credentials(credentials)?;
        let result = client.unarchive_work_list(work_list_id).await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let work_list = result?;
        Ok(self.project_work_list_summary(work_list, Some(&data_key)))
    }

    pub async fn get_work_list(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentWorkListDetail> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt work list data.",
            )
            .await?;
        let query = ReadCacheQuery::WorkList { work_list_id };
        let detail: WorkListDetailResponse = if self.is_offline() {
            self.read_cache
                .read_offline(&credentials, &data_key, &query)?
        } else if let Some(cached) = self.read_cache.memoized(&credentials, &query)? {
            cached
        } else {
            let cache_guard = self.read_cache.begin_online_read(&credentials)?;
            let mut client = self.api_client_with_credentials(credentials.clone())?;
            let detail = client.get_work_list(work_list_id).await?;
            self.read_cache
                .record_online(cache_guard.as_ref(), &data_key, &query, &detail)?;
            detail
        };
        Ok(self.project_work_list_detail(detail, Some(&data_key)))
    }
}
