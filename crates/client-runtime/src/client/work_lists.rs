use super::RuntimeClient;
use crate::models::{AgentWorkListDetail, AgentWorkListSummary};
use uuid::Uuid;
use worklist_client_api::PublicApiClient;
use worklist_client_core::PublicResult;

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
        let mut client = PublicApiClient::with_credentials(&self.api_url, credentials);
        let lists = client
            .list_work_lists_with_archived(include_archived)
            .await?;
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
        let mut client = PublicApiClient::with_credentials(&self.api_url, credentials);
        let work_list = client.archive_work_list(work_list_id).await?;
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
        let mut client = PublicApiClient::with_credentials(&self.api_url, credentials);
        let work_list = client.unarchive_work_list(work_list_id).await?;
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
        let mut client = PublicApiClient::with_credentials(&self.api_url, credentials);
        let detail = client.get_work_list(work_list_id).await?;
        Ok(self.project_work_list_detail(detail, Some(&data_key)))
    }
}
