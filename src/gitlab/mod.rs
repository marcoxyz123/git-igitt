pub mod models;

use models::{Job, Pipeline, PipelineDetails};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};

pub struct GitLabClient {
    client: Client,
    base_url: String,
    token: String,
}

impl GitLabClient {
    pub fn new(base_url: &str, token: &str) -> Result<Self, String> {
        let base_url = base_url.trim_end_matches('/').to_string();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            base_url,
            token: token.to_string(),
        })
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "PRIVATE-TOKEN",
            HeaderValue::from_str(&self.token).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers
    }

    pub fn get_pipeline_for_commit(
        &self,
        project_id: &str,
        sha: &str,
    ) -> Result<Option<Pipeline>, String> {
        let url = format!(
            "{}/api/v4/projects/{}/pipelines?sha={}",
            self.base_url,
            urlencoded(project_id),
            sha
        );

        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("GitLab API error: {}", response.status()));
        }

        let pipelines: Vec<Pipeline> = response
            .json()
            .map_err(|e| format!("Failed to parse pipelines: {}", e))?;

        Ok(pipelines.into_iter().next())
    }

    pub fn get_pipeline_jobs(
        &self,
        project_id: &str,
        pipeline_id: u64,
    ) -> Result<Vec<Job>, String> {
        let url = format!(
            "{}/api/v4/projects/{}/pipelines/{}/jobs?per_page=100",
            self.base_url,
            urlencoded(project_id),
            pipeline_id
        );

        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("GitLab API error: {}", response.status()));
        }

        response
            .json()
            .map_err(|e| format!("Failed to parse jobs: {}", e))
    }

    pub fn get_pipeline_details(
        &self,
        project_id: &str,
        sha: &str,
    ) -> Result<Option<PipelineDetails>, String> {
        let pipeline = match self.get_pipeline_for_commit(project_id, sha)? {
            Some(p) => p,
            None => return Ok(None),
        };

        let jobs = self.get_pipeline_jobs(project_id, pipeline.id)?;
        Ok(Some(PipelineDetails::from_jobs(pipeline, jobs)))
    }
}

fn urlencoded(s: &str) -> String {
    s.replace('/', "%2F")
}
