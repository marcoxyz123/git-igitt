use crate::gitlab::models::{PipelineDetails, PipelineStatus, Stage};
use crate::theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use std::collections::HashMap;

const MIN_STAGE_WIDTH: u16 = 16;
const MAX_STAGE_WIDTH: u16 = 24;
const CONNECTOR_WIDTH: u16 = 3;
const MAX_CACHE_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub enum CachedPipeline {
    Found(PipelineDetails),
    NotFound,
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct PipelineViewState {
    pub details: Option<PipelineDetails>,
    pub current_sha: Option<String>,
    pub selected_stage: usize,
    pub selected_job: usize,
    pub scroll_x: u16,
    pub scroll_y: u16,
    pub error: Option<String>,
    pub loading: bool,
    cache: HashMap<String, CachedPipeline>,
    cache_order: Vec<String>,
    pub animation_tick: u8,
}

impl PipelineViewState {
    pub fn get_cached(&self, sha: &str) -> Option<&CachedPipeline> {
        self.cache.get(sha)
    }

    pub fn cache_result(&mut self, sha: String, result: CachedPipeline) {
        if self.cache.len() >= MAX_CACHE_SIZE {
            if let Some(old_sha) = self.cache_order.first().cloned() {
                self.cache.remove(&old_sha);
                self.cache_order.remove(0);
            }
        }
        if !self.cache.contains_key(&sha) {
            self.cache_order.push(sha.clone());
        }
        self.cache.insert(sha, result);
    }

    pub fn invalidate_cache(&mut self, sha: &str) {
        self.cache.remove(sha);
        self.cache_order.retain(|s| s != sha);
    }

    pub fn set_pipeline(&mut self, sha: Option<String>, details: Option<PipelineDetails>) {
        if let Some(sha) = &sha {
            let cached = match &details {
                Some(d) => CachedPipeline::Found(d.clone()),
                None => CachedPipeline::NotFound,
            };
            self.cache_result(sha.clone(), cached);
        }
        self.current_sha = sha;
        self.details = details;
        self.selected_stage = 0;
        self.selected_job = 0;
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.error = None;
        self.loading = false;
    }

    pub fn set_error(&mut self, sha: Option<String>, error: String) {
        if let Some(sha) = &sha {
            self.cache_result(sha.clone(), CachedPipeline::Error(error.clone()));
        }
        self.current_sha = sha;
        self.error = Some(error);
        self.loading = false;
    }

    pub fn set_loading(&mut self, sha: Option<String>) {
        let sha_changed = self.current_sha != sha;
        if sha_changed {
            self.details = None;
            self.selected_stage = 0;
            self.selected_job = 0;
        }
        self.current_sha = sha;
        self.loading = true;
        self.error = None;
    }

    pub fn apply_cached(&mut self, sha: &str, cached: &CachedPipeline) {
        self.current_sha = Some(sha.to_string());
        self.loading = false;
        match cached {
            CachedPipeline::Found(details) => {
                self.details = Some(details.clone());
                self.error = None;
            }
            CachedPipeline::NotFound => {
                self.details = None;
                self.error = None;
            }
            CachedPipeline::Error(e) => {
                self.details = None;
                self.error = Some(e.clone());
            }
        }
        self.selected_stage = 0;
        self.selected_job = 0;
    }

    pub fn is_running(&self) -> bool {
        if let Some(details) = &self.details {
            if let Some(pipeline) = &details.pipeline {
                return matches!(
                    pipeline.status,
                    PipelineStatus::Running | PipelineStatus::Pending | PipelineStatus::Preparing
                );
            }
        }
        false
    }

    pub fn select_next_stage(&mut self) {
        if let Some(details) = &self.details {
            if self.selected_stage < details.stages.len().saturating_sub(1) {
                self.selected_stage += 1;
                self.selected_job = 0;
            }
        }
    }

    pub fn select_prev_stage(&mut self) {
        if self.selected_stage > 0 {
            self.selected_stage -= 1;
            self.selected_job = 0;
        }
    }

    pub fn select_next_job(&mut self) {
        if let Some(details) = &self.details {
            if let Some(stage) = details.stages.get(self.selected_stage) {
                if self.selected_job < stage.jobs.len().saturating_sub(1) {
                    self.selected_job += 1;
                }
            }
        }
    }

    pub fn select_prev_job(&mut self) {
        if self.selected_job > 0 {
            self.selected_job -= 1;
        }
    }

    pub fn auto_scroll_to_active(&mut self) {
        if let Some(details) = &self.details {
            for (stage_idx, stage) in details.stages.iter().enumerate() {
                for (job_idx, job) in stage.jobs.iter().enumerate() {
                    if job.status == PipelineStatus::Running || job.status == PipelineStatus::Failed
                    {
                        self.selected_stage = stage_idx;
                        self.selected_job = job_idx;
                        return;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineView<'a> {
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
}

impl<'a> Default for PipelineView<'a> {
    fn default() -> Self {
        Self {
            block: None,
            style: Style::default(),
            highlight_style: Style::default().add_modifier(Modifier::BOLD),
        }
    }
}

impl<'a> PipelineView<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }
}

fn pulse_color(base_r: u8, base_g: u8, base_b: u8, tick: u8) -> ratatui::style::Color {
    let phase = (tick as f32 / 255.0) * std::f32::consts::PI * 2.0;
    let factor = (phase.sin() + 1.0) / 2.0;
    let bright_factor = 0.3 + (factor * 0.7);
    ratatui::style::Color::Rgb(
        (base_r as f32 * bright_factor) as u8,
        (base_g as f32 * bright_factor) as u8,
        (base_b as f32 * bright_factor) as u8,
    )
}

fn status_color(status: PipelineStatus, tick: u8) -> ratatui::style::Color {
    match status {
        PipelineStatus::Success => theme::pipeline::SUCCESS,
        PipelineStatus::Running => pulse_color(136, 192, 208, tick),
        PipelineStatus::Pending
        | PipelineStatus::WaitingForResource
        | PipelineStatus::Preparing => pulse_color(235, 203, 139, tick),
        PipelineStatus::Failed => theme::pipeline::FAILED,
        PipelineStatus::Canceled => theme::pipeline::CANCELED,
        PipelineStatus::Skipped => theme::pipeline::SKIPPED,
        PipelineStatus::Manual => theme::pipeline::MANUAL,
        PipelineStatus::Created | PipelineStatus::Scheduled => theme::pipeline::CREATED,
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 2 {
        format!("{}…", &s[..max_len - 1])
    } else {
        s[..max_len].to_string()
    }
}

fn calculate_stage_width(stage: &Stage, max_width: u16) -> u16 {
    let name_len = stage.name.len() as u16 + 4;
    let max_job_len = stage
        .jobs
        .iter()
        .map(|j| j.name.len() as u16 + 6)
        .max()
        .unwrap_or(MIN_STAGE_WIDTH);
    name_len
        .max(max_job_len)
        .max(MIN_STAGE_WIDTH)
        .min(max_width)
}

struct LayoutInfo {
    stages_per_row: usize,
    stage_width: u16,
    row_height: u16,
}

fn calculate_layout(details: &PipelineDetails, area: Rect) -> LayoutInfo {
    let num_stages = details.stages.len();
    if num_stages == 0 {
        return LayoutInfo {
            stages_per_row: 0,
            stage_width: MIN_STAGE_WIDTH,
            row_height: area.height,
        };
    }

    let max_jobs = details
        .stages
        .iter()
        .map(|s| s.jobs.len())
        .max()
        .unwrap_or(1);

    let preferred_width: u16 = details
        .stages
        .iter()
        .map(|s| calculate_stage_width(s, MAX_STAGE_WIDTH))
        .max()
        .unwrap_or(MIN_STAGE_WIDTH);

    let total_horizontal = (preferred_width + CONNECTOR_WIDTH) * num_stages as u16;

    if total_horizontal <= area.width {
        return LayoutInfo {
            stages_per_row: num_stages,
            stage_width: preferred_width,
            row_height: area.height.saturating_sub(1),
        };
    }

    let usable_width = area.width;
    let stages_per_row =
        ((usable_width + CONNECTOR_WIDTH) / (MIN_STAGE_WIDTH + CONNECTOR_WIDTH)).max(1) as usize;
    let stages_per_row = stages_per_row.min(num_stages);

    let stage_width = if stages_per_row > 0 {
        ((usable_width + CONNECTOR_WIDTH) / stages_per_row as u16)
            .saturating_sub(CONNECTOR_WIDTH)
            .max(MIN_STAGE_WIDTH)
    } else {
        MIN_STAGE_WIDTH
    };

    let row_height = (max_jobs as u16 + 3).max(5);

    LayoutInfo {
        stages_per_row,
        stage_width,
        row_height,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_stage(
    buf: &mut Buffer,
    stage: &Stage,
    x: u16,
    y: u16,
    width: u16,
    max_height: u16,
    is_selected_stage: bool,
    selected_job: usize,
    highlight_style: Style,
    tick: u8,
) {
    let stage_status = stage.status();
    let status_style = Style::default().fg(status_color(stage_status, tick));

    let header = format!(
        "{} {}",
        stage_status.symbol(),
        truncate_str(&stage.name, width.saturating_sub(4) as usize)
    );
    buf.set_string(x + 1, y, &header, status_style);

    let separator: String = "─".repeat(width as usize);
    buf.set_string(x, y + 1, &separator, Style::default().fg(theme::BORDER));

    let available_job_lines = max_height.saturating_sub(3) as usize;

    for (job_idx, job) in stage.jobs.iter().enumerate().take(available_job_lines) {
        let job_y = y + 2 + job_idx as u16;
        if job_y >= y + max_height {
            break;
        }

        let job_status_style = Style::default().fg(status_color(job.status, tick));
        let is_selected = is_selected_stage && job_idx == selected_job;
        let prefix = if is_selected { "▸" } else { " " };

        let job_name = truncate_str(&job.name, width.saturating_sub(5) as usize);
        let job_line = format!("{}{} {}", prefix, job.status.symbol(), job_name);

        let line_style = if is_selected {
            highlight_style.patch(job_status_style)
        } else {
            job_status_style
        };

        buf.set_string(x + 1, job_y, &job_line, line_style);
    }

    if stage.jobs.len() > available_job_lines {
        let more_y = y + max_height.saturating_sub(1);
        let remaining = stage.jobs.len() - available_job_lines;
        let more_text = format!(" +{} more", remaining);
        buf.set_string(
            x + 1,
            more_y,
            &more_text,
            Style::default().fg(theme::TEXT_DIM),
        );
    }
}

impl StatefulWidget for PipelineView<'_> {
    type State = PipelineViewState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        let inner_area = match self.block.take() {
            Some(b) => {
                let inner = b.inner(area);
                b.render(area, buf);
                inner
            }
            None => area,
        };

        if inner_area.width < 1 || inner_area.height < 1 {
            return;
        }

        if state.loading {
            let msg = "Loading pipeline...";
            let x = inner_area.left() + (inner_area.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner_area.top() + inner_area.height / 2;
            buf.set_string(x, y, msg, Style::default().fg(theme::TEXT_DIM));
            return;
        }

        if let Some(error) = &state.error {
            let msg = format!("Error: {}", error);
            let display_msg = truncate_str(&msg, inner_area.width as usize);
            buf.set_string(
                inner_area.left() + 1,
                inner_area.top() + 1,
                &display_msg,
                Style::default().fg(theme::ERROR),
            );
            return;
        }

        let details = match &state.details {
            Some(d) => d,
            None => {
                let msg = "No pipeline for this commit";
                let x = inner_area.left() + (inner_area.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner_area.top() + inner_area.height / 2;
                buf.set_string(x, y, msg, Style::default().fg(theme::TEXT_DIM));
                return;
            }
        };

        if details.stages.is_empty() {
            let msg = "Pipeline has no stages";
            let x = inner_area.left() + (inner_area.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner_area.top() + inner_area.height / 2;
            buf.set_string(x, y, msg, Style::default().fg(theme::TEXT_DIM));
            return;
        }

        let content_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: inner_area.width,
            height: inner_area.height.saturating_sub(1),
        };

        let layout = calculate_layout(details, content_area);

        for (stage_idx, stage) in details.stages.iter().enumerate() {
            let row = stage_idx / layout.stages_per_row;
            let col = stage_idx % layout.stages_per_row;

            let stage_x =
                content_area.left() + (col as u16 * (layout.stage_width + CONNECTOR_WIDTH));
            let stage_y = content_area.top() + (row as u16 * layout.row_height);

            if stage_y >= content_area.bottom() {
                break;
            }

            let available_height =
                (content_area.bottom().saturating_sub(stage_y)).min(layout.row_height);

            render_stage(
                buf,
                stage,
                stage_x,
                stage_y,
                layout.stage_width,
                available_height,
                stage_idx == state.selected_stage,
                state.selected_job,
                self.highlight_style,
                state.animation_tick,
            );

            let is_last_in_row = col == layout.stages_per_row - 1;
            let is_last_stage = stage_idx == details.stages.len() - 1;

            if !is_last_in_row && !is_last_stage {
                let connector_x = stage_x + layout.stage_width;
                if connector_x + 2 < content_area.right() {
                    buf.set_string(
                        connector_x,
                        stage_y,
                        "─→─",
                        Style::default().fg(theme::BORDER),
                    );
                }
            }
        }

        if let Some(pipeline) = &details.pipeline {
            let running_indicator = if state.is_running() { " ⟳" } else { "" };
            let status_text = format!(
                "Pipeline #{} - {}{}",
                pipeline.id, pipeline.status, running_indicator
            );
            let status_color = status_color(pipeline.status, state.animation_tick);
            buf.set_string(
                inner_area.left(),
                inner_area.bottom().saturating_sub(1),
                truncate_str(&status_text, inner_area.width as usize),
                Style::default().fg(status_color),
            );
        }
    }
}

impl Widget for PipelineView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = PipelineViewState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
