use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Created,
    WaitingForResource,
    Preparing,
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Canceling,
    Skipped,
    Manual,
    Scheduled,
}

const SPINNER_FRAMES: &[&str] = &["◜", "◠", "◝", "◞", "◡", "◟"];

impl PipelineStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Success => "●",
            Self::Running => "◐",
            Self::Pending | Self::WaitingForResource | Self::Preparing => "○",
            Self::Failed => "✕",
            Self::Canceled | Self::Canceling => "⊘",
            Self::Skipped => "⊘",
            Self::Manual => "▶",
            Self::Created | Self::Scheduled => "◯",
        }
    }

    pub fn animated_symbol(&self, tick: u8) -> &'static str {
        match self {
            Self::Running => {
                let frame = (tick as usize / 4) % SPINNER_FRAMES.len();
                SPINNER_FRAMES[frame]
            }
            Self::Pending | Self::WaitingForResource | Self::Preparing => {
                let frame = (tick as usize / 6) % SPINNER_FRAMES.len();
                SPINNER_FRAMES[frame]
            }
            _ => self.symbol(),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Running | Self::Pending | Self::WaitingForResource | Self::Preparing
        )
    }
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Created => "created",
            Self::WaitingForResource => "waiting",
            Self::Preparing => "preparing",
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
            Self::Canceling => "canceling",
            Self::Skipped => "skipped",
            Self::Manual => "manual",
            Self::Scheduled => "scheduled",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pipeline {
    pub id: u64,
    pub iid: Option<u64>,
    pub status: PipelineStatus,
    pub sha: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub web_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: PipelineStatus,
    pub stage: String,
    pub web_url: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration: Option<f64>,
    pub allow_failure: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct Stage {
    pub name: String,
    pub jobs: Vec<Job>,
}

impl Stage {
    pub fn new(name: String) -> Self {
        Self {
            name,
            jobs: Vec::new(),
        }
    }

    pub fn status(&self) -> PipelineStatus {
        let mut has_running = false;
        let mut has_pending = false;
        let mut has_failed = false;

        for job in &self.jobs {
            match job.status {
                PipelineStatus::Failed => {
                    if !job.allow_failure.unwrap_or(false) {
                        has_failed = true;
                    }
                }
                PipelineStatus::Running => has_running = true,
                PipelineStatus::Pending
                | PipelineStatus::WaitingForResource
                | PipelineStatus::Preparing => has_pending = true,
                _ => {}
            }
        }

        if has_failed {
            PipelineStatus::Failed
        } else if has_running {
            PipelineStatus::Running
        } else if has_pending {
            PipelineStatus::Pending
        } else if self
            .jobs
            .iter()
            .all(|j| j.status == PipelineStatus::Success)
        {
            PipelineStatus::Success
        } else if self
            .jobs
            .iter()
            .all(|j| j.status == PipelineStatus::Skipped)
        {
            PipelineStatus::Skipped
        } else {
            PipelineStatus::Created
        }
    }

    pub fn has_mixed_failure(&self) -> bool {
        let has_real_failure = self
            .jobs
            .iter()
            .any(|j| j.status == PipelineStatus::Failed && !j.allow_failure.unwrap_or(false));
        let has_non_failure = self
            .jobs
            .iter()
            .any(|j| j.status != PipelineStatus::Failed || j.allow_failure.unwrap_or(false));
        has_real_failure && has_non_failure
    }
}

#[derive(Debug, Clone, Default)]
pub struct PipelineDetails {
    pub pipeline: Option<Pipeline>,
    pub stages: Vec<Stage>,
}

impl PipelineDetails {
    pub fn from_jobs(pipeline: Pipeline, jobs: Vec<Job>) -> Self {
        let mut stages: Vec<Stage> = Vec::new();

        for job in jobs {
            let stage_name = job.stage.clone();
            if let Some(stage) = stages.iter_mut().find(|s| s.name == stage_name) {
                stage.jobs.push(job);
            } else {
                let mut new_stage = Stage::new(stage_name);
                new_stage.jobs.push(job);
                stages.push(new_stage);
            }
        }

        stages.sort_by_key(|s| s.jobs.iter().map(|j| j.id).min().unwrap_or(u64::MAX));

        Self {
            pipeline: Some(pipeline),
            stages,
        }
    }

    pub fn status(&self) -> Option<PipelineStatus> {
        self.pipeline.as_ref().map(|p| p.status)
    }
}
