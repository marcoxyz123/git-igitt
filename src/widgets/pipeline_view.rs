use crate::gitlab::models::{PipelineDetails, PipelineStatus, Stage};
use crate::theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: Option<String>,
    pub content: String,
    pub styled: Vec<(String, Style)>,
    pub duration: Option<String>,
}

const MIN_STAGE_WIDTH: u16 = 16;
const CONNECTOR_WIDTH: u16 = 5;
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
    pub job_log: Vec<LogLine>,
    pub job_log_job_id: Option<u64>,
    pub job_log_scroll: u16,
    pub job_log_loading: bool,
    pub job_log_error: Option<String>,
    job_log_cache: HashMap<u64, String>,
    pub job_log_focused: bool,
    pub job_log_visible_height: u16,
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
        let same_sha = self.current_sha == sha;
        self.current_sha = sha;
        self.details = details;
        if !same_sha {
            self.selected_stage = 0;
            self.selected_job = 0;
            self.scroll_x = 0;
            self.scroll_y = 0;
        }
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
            let mut best: Option<(usize, usize, &str)> = None;
            for (stage_idx, stage) in details.stages.iter().enumerate() {
                for (job_idx, job) in stage.jobs.iter().enumerate() {
                    if job.status == PipelineStatus::Running {
                        let ts = job.started_at.as_deref().unwrap_or("");
                        if best.is_none_or(|(_, _, prev_ts)| ts > prev_ts) {
                            best = Some((stage_idx, job_idx, ts));
                        }
                    }
                }
            }
            if let Some((si, ji, _)) = best {
                self.selected_stage = si;
                self.selected_job = ji;
                return;
            }
            for (stage_idx, stage) in details.stages.iter().enumerate() {
                for (job_idx, job) in stage.jobs.iter().enumerate() {
                    if job.status == PipelineStatus::Failed {
                        self.selected_stage = stage_idx;
                        self.selected_job = job_idx;
                        return;
                    }
                }
            }
        }
    }

    pub fn get_selected_job_id(&self) -> Option<u64> {
        self.details
            .as_ref()
            .and_then(|d| d.stages.get(self.selected_stage))
            .and_then(|s| s.jobs.get(self.selected_job))
            .map(|j| j.id)
    }

    pub fn selected_job_is_running(&self) -> bool {
        self.details
            .as_ref()
            .and_then(|d| d.stages.get(self.selected_stage))
            .and_then(|s| s.jobs.get(self.selected_job))
            .map(|j| matches!(j.status, PipelineStatus::Running | PipelineStatus::Pending))
            .unwrap_or(false)
    }

    pub fn set_job_log(&mut self, job_id: u64, log_text: &str) {
        self.job_log = parse_gitlab_log(log_text);
        self.job_log_job_id = Some(job_id);
        self.job_log_loading = false;
        self.job_log_error = None;
        if self.selected_job_is_running() {
            let visible = self.job_log_visible_height as usize;
            self.job_log_scroll = self.job_log.len().saturating_sub(visible) as u16;
        }
    }

    pub fn cache_job_log(&mut self, job_id: u64, raw_text: String) {
        self.job_log_cache.insert(job_id, raw_text);
    }

    pub fn get_cached_job_log(&self, job_id: u64) -> Option<&String> {
        self.job_log_cache.get(&job_id)
    }

    pub fn job_log_as_text(&self) -> String {
        let width = self.job_log.len().max(1).to_string().len();
        self.job_log
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                let num = format!("{:>w$}", idx + 1, w = width);
                let ts = line.timestamp.as_deref().unwrap_or("        ");
                let dur = line.duration.as_deref().unwrap_or("");
                if dur.is_empty() {
                    format!("{} {}  {}", num, ts, line.content)
                } else {
                    format!("{} {}  {} {}", num, ts, line.content, dur)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn needs_multirow(&self, available_width: u16) -> bool {
        if let Some(details) = &self.details {
            if details.stages.is_empty() {
                return false;
            }
            let ideal_total: u16 = details
                .stages
                .iter()
                .map(calculate_stage_width)
                .sum::<u16>()
                + (details.stages.len() as u16).saturating_sub(1) * CONNECTOR_WIDTH;
            let shrink_min = details.stages.len() as u16 * MIN_STAGE_WIDTH
                + (details.stages.len() as u16).saturating_sub(1) * CONNECTOR_WIDTH;
            return ideal_total > available_width && shrink_min > available_width;
        }
        false
    }

    pub fn clear_job_log(&mut self) {
        self.job_log.clear();
        self.job_log_job_id = None;
        self.job_log_scroll = 0;
        self.job_log_loading = false;
        self.job_log_error = None;
    }
}

fn strip_log_prefix(line: &str) -> (Option<&str>, &str) {
    if line.len() > 32
        && line.as_bytes().get(4) == Some(&b'-')
        && line.as_bytes().get(10) == Some(&b'T')
    {
        let ts = &line[11..19];
        if let Some(pos) = line.get(28..).and_then(|s| s.find(' ')) {
            (Some(ts), &line[28 + pos + 1..])
        } else {
            (Some(ts), line)
        }
    } else {
        (None, line)
    }
}

fn extract_section_timestamp(marker: &str) -> Option<u64> {
    let parts: Vec<&str> = marker.splitn(3, ':').collect();
    if parts.len() >= 2 {
        parts[1].parse::<u64>().ok()
    } else {
        None
    }
}

fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn parse_gitlab_log(raw: &str) -> Vec<LogLine> {
    let mut result = Vec::new();
    let mut section_starts: HashMap<String, u64> = HashMap::new();
    let mut pending_duration: Option<String> = None;

    for raw_line in raw.lines() {
        let line = raw_line.trim_end_matches('\r');

        if let Some(start_pos) = line.find("section_end:") {
            let marker = &line[start_pos + 12..];
            if let Some(end_ts) = extract_section_timestamp(marker) {
                let section_name = marker.splitn(3, ':').nth(2).unwrap_or("");
                if let Some(start_ts) = section_starts.get(section_name) {
                    let duration = end_ts.saturating_sub(*start_ts);
                    pending_duration = Some(format_duration(duration));
                }
            }

            let after_marker = line[start_pos..].find('\n').map(|p| &line[start_pos + p..]);
            if let Some(rest) = after_marker {
                if !rest.trim().is_empty() && !rest.contains("section_start:") {
                    let (ts, _) = strip_log_prefix(line);
                    let cleaned = rest.replace("\x1b[0K", "");
                    let content_cleaned = strip_ansi_for_empty_check(&cleaned);
                    if !content_cleaned.trim().is_empty() {
                        result.push(LogLine {
                            timestamp: ts.map(String::from),
                            styled: parse_ansi_to_styled(&cleaned),
                            content: cleaned,
                            duration: None,
                        });
                    }
                }
            }
            continue;
        }

        if let Some(start_pos) = line.find("section_start:") {
            let marker = &line[start_pos + 14..];
            if let Some(start_ts) = extract_section_timestamp(marker) {
                let section_name = marker.splitn(3, ':').nth(2).unwrap_or("").to_string();
                let section_name = section_name.trim_end_matches('\r').to_string();
                section_starts.insert(section_name, start_ts);
            }
            continue;
        }

        let (ts, body) = strip_log_prefix(line);
        let cleaned = body.replace("\x1b[0K", "").replace("\x1b[0;m", "");
        let content_cleaned = strip_ansi_for_empty_check(&cleaned);
        if content_cleaned.trim().is_empty() {
            continue;
        }

        let duration = pending_duration.take();
        if let Some(ref d) = duration {
            if let Some(last) = result.last_mut() {
                last.duration = Some(d.clone());
                continue;
            }
        }

        result.push(LogLine {
            timestamp: ts.map(String::from),
            styled: parse_ansi_to_styled(&cleaned),
            content: cleaned,
            duration,
        });
    }

    result
}

fn strip_ansi_for_empty_check(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn parse_ansi_to_styled(line: &str) -> Vec<(String, Style)> {
    let mut segments = Vec::new();
    let mut current_style = Style::default();
    let mut current_text = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                let mut code = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == ';' {
                        code.push(c);
                        chars.next();
                    } else if c == 'm' {
                        chars.next();
                        break;
                    } else {
                        break;
                    }
                }
                if !current_text.is_empty() {
                    segments.push((current_text.clone(), current_style));
                    current_text.clear();
                }
                current_style = apply_ansi_code(&code);
            }
        } else {
            current_text.push(ch);
        }
    }
    if !current_text.is_empty() {
        segments.push((current_text, current_style));
    }
    if segments.is_empty() {
        segments.push((String::new(), Style::default()));
    }
    segments
}

fn apply_ansi_code(code: &str) -> Style {
    let parts: Vec<&str> = code.split(';').collect();
    match parts.as_slice() {
        ["0"] | [""] | ["0", ""] => Style::default(),
        ["1"] => Style::default().add_modifier(Modifier::BOLD),
        ["31"] => Style::default().fg(Color::Rgb(191, 97, 106)),
        ["31", "1"] => Style::default()
            .fg(Color::Rgb(191, 97, 106))
            .add_modifier(Modifier::BOLD),
        ["32"] => Style::default().fg(Color::Rgb(163, 190, 140)),
        ["32", "1"] => Style::default()
            .fg(Color::Rgb(163, 190, 140))
            .add_modifier(Modifier::BOLD),
        ["33"] | ["0", "33"] => Style::default().fg(Color::Rgb(235, 203, 139)),
        ["33", "1"] => Style::default()
            .fg(Color::Rgb(235, 203, 139))
            .add_modifier(Modifier::BOLD),
        ["34"] => Style::default().fg(Color::Rgb(94, 129, 172)),
        ["34", "1"] => Style::default()
            .fg(Color::Rgb(94, 129, 172))
            .add_modifier(Modifier::BOLD),
        ["35"] => Style::default().fg(Color::Rgb(180, 142, 173)),
        ["35", "1"] => Style::default()
            .fg(Color::Rgb(180, 142, 173))
            .add_modifier(Modifier::BOLD),
        ["36"] => Style::default().fg(Color::Rgb(136, 192, 208)),
        ["36", "1"] => Style::default()
            .fg(Color::Rgb(136, 192, 208))
            .add_modifier(Modifier::BOLD),
        ["37"] | ["37", "1"] => Style::default().fg(Color::Rgb(216, 222, 233)),
        ["90"] => Style::default().fg(Color::Rgb(76, 86, 106)),
        _ => Style::default(),
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

fn status_base_rgb(status: PipelineStatus) -> (u8, u8, u8) {
    match status {
        PipelineStatus::Success => (163, 190, 140),
        PipelineStatus::Running => (136, 192, 208),
        PipelineStatus::Pending
        | PipelineStatus::WaitingForResource
        | PipelineStatus::Preparing => (235, 203, 139),
        PipelineStatus::Failed => (191, 97, 106),
        PipelineStatus::Canceled | PipelineStatus::Canceling => (136, 192, 208),
        PipelineStatus::Skipped => (76, 86, 106),
        PipelineStatus::Manual => (180, 142, 173),
        PipelineStatus::Created | PipelineStatus::Scheduled => (216, 222, 233),
    }
}

fn status_color(status: PipelineStatus) -> ratatui::style::Color {
    let (r, g, b) = status_base_rgb(status);
    ratatui::style::Color::Rgb(r, g, b)
}

fn sweep_color(
    base_r: u8,
    base_g: u8,
    base_b: u8,
    char_pos: u16,
    total_chars: u16,
    tick: u8,
) -> ratatui::style::Color {
    let phase = (tick as f32 / 255.0) * 2.0;
    let sweep_pos = if phase < 1.0 {
        phase * (total_chars as f32 + 2.0) - 1.0
    } else {
        (2.0 - phase) * (total_chars as f32 + 2.0) - 1.0
    };

    let dist = (char_pos as f32 - sweep_pos).abs();
    let glow = (1.0 - dist / 2.5).max(0.0);
    let glow = glow * glow;

    let mix = |base: u8, target: u8, t: f32| -> u8 {
        (base as f32 + (target as f32 - base as f32) * t).clamp(0.0, 255.0) as u8
    };

    ratatui::style::Color::Rgb(
        mix(base_r, 236, glow),
        mix(base_g, 239, glow),
        mix(base_b, 244, glow),
    )
}

fn render_sweep_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    status: PipelineStatus,
    tick: u8,
    max_width: u16,
) {
    let (base_r, base_g, base_b) = status_base_rgb(status);
    let total_chars = text.len().min(max_width as usize) as u16;
    for (i, ch) in text.chars().enumerate() {
        let cx = x + i as u16;
        if i as u16 >= max_width {
            break;
        }
        let color = sweep_color(base_r, base_g, base_b, i as u16, total_chars, tick);
        buf.set_string(cx, y, ch.to_string(), Style::default().fg(color));
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

fn calculate_stage_width(stage: &Stage) -> u16 {
    // Top border: ╭─ icon name ──╮ needs name.len() + 8
    let header_width = stage.name.len() as u16 + 8;
    // Job line: │ icon name  │ needs job_name.len() + 5
    let max_job_width = stage
        .jobs
        .iter()
        .map(|j| j.name.len() as u16 + 5)
        .max()
        .unwrap_or(0);
    header_width.max(max_job_width).max(MIN_STAGE_WIDTH)
}

struct StageLayout {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

struct LayoutInfo {
    stages: Vec<StageLayout>,
    total_height: u16,
}

fn calculate_layout(details: &PipelineDetails, area: Rect) -> LayoutInfo {
    let num_stages = details.stages.len();
    if num_stages == 0 {
        return LayoutInfo {
            stages: vec![],
            total_height: 0,
        };
    }

    let area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    let ideal_widths: Vec<u16> = details.stages.iter().map(calculate_stage_width).collect();

    let connector_total = (num_stages as u16).saturating_sub(1) * CONNECTOR_WIDTH;
    let total_ideal: u16 = ideal_widths.iter().copied().sum::<u16>() + connector_total;

    // Case 1: All fit in one row with ideal widths — center them
    if total_ideal <= area.width {
        let max_jobs = details
            .stages
            .iter()
            .map(|s| s.jobs.len())
            .max()
            .unwrap_or(0);
        let row_height = max_jobs as u16 + 2;
        let left_pad = (area.width.saturating_sub(total_ideal)) / 2;
        let mut x = area.x + left_pad;
        let stages = ideal_widths
            .iter()
            .enumerate()
            .map(|(i, &w)| {
                let sl = StageLayout {
                    x,
                    y: area.y,
                    width: w,
                    height: row_height,
                };
                x += w;
                if i < num_stages - 1 {
                    x += CONNECTOR_WIDTH;
                }
                sl
            })
            .collect();
        return LayoutInfo {
            stages,
            total_height: row_height,
        };
    }

    // Case 2: Shrink proportionally to fit one row
    let avail_for_stages = area.width.saturating_sub(connector_total);
    let sum_ideal: u16 = ideal_widths.iter().copied().sum();
    if sum_ideal > 0 && avail_for_stages >= num_stages as u16 * MIN_STAGE_WIDTH {
        let shrunk: Vec<u16> = ideal_widths
            .iter()
            .map(|&w| {
                ((w as u32 * avail_for_stages as u32) / sum_ideal as u32)
                    .max(MIN_STAGE_WIDTH as u32) as u16
            })
            .collect();
        let shrunk_total: u16 = shrunk.iter().copied().sum::<u16>() + connector_total;
        if shrunk_total <= area.width {
            let max_jobs = details
                .stages
                .iter()
                .map(|s| s.jobs.len())
                .max()
                .unwrap_or(0);
            let row_height = max_jobs as u16 + 2;
            let left_pad = (area.width.saturating_sub(shrunk_total)) / 2;
            let mut x = area.x + left_pad;
            let stages = shrunk
                .iter()
                .enumerate()
                .map(|(i, &w)| {
                    let sl = StageLayout {
                        x,
                        y: area.y,
                        width: w,
                        height: row_height,
                    };
                    x += w;
                    if i < num_stages - 1 {
                        x += CONNECTOR_WIDTH;
                    }
                    sl
                })
                .collect();
            return LayoutInfo {
                stages,
                total_height: row_height,
            };
        }
    }

    // Case 3: Multi-row wrapping — compute full layout (no height clamping)
    let mut stages = Vec::new();
    let mut row_start = 0;
    let mut row_index = 0u16;
    let mut y = area.y;

    while row_start < num_stages {
        let mut row_end = row_start;
        let mut row_width: u16 = 0;
        // Non-first rows need 3 chars left margin for the wrap connector (╰──┤)
        let wrap_margin: u16 = if row_index > 0 { 3 } else { 0 };
        let row_avail = area.width.saturating_sub(wrap_margin);
        for (i, &ideal_w) in ideal_widths.iter().enumerate().skip(row_start) {
            let w = ideal_w.min(row_avail);
            let conn = if i > row_start { CONNECTOR_WIDTH } else { 0 };
            if row_width + conn + w > row_avail && i > row_start {
                break;
            }
            row_width += conn + w;
            row_end = i + 1;
        }
        if row_end == row_start {
            row_end = row_start + 1;
        }

        let row_stages = &details.stages[row_start..row_end];
        let mut row_widths: Vec<u16> = ideal_widths[row_start..row_end]
            .iter()
            .map(|&w| w.min(row_avail))
            .collect();

        let row_conn = (row_widths.len() as u16).saturating_sub(1) * CONNECTOR_WIDTH;
        let row_total: u16 = row_widths.iter().copied().sum::<u16>() + row_conn;
        if row_total > row_avail {
            let avail = row_avail.saturating_sub(row_conn);
            let sum_w: u16 = row_widths.iter().copied().sum();
            if sum_w > 0 {
                row_widths = row_widths
                    .iter()
                    .map(|&w| {
                        ((w as u32 * avail as u32) / sum_w as u32).max(MIN_STAGE_WIDTH as u32)
                            as u16
                    })
                    .collect();
            }
        }

        let actual_total: u16 = row_widths.iter().copied().sum::<u16>() + row_conn;
        let left_pad = if row_index > 0 {
            // Non-first rows: ensure at least 3 chars left margin for wrap connector
            let natural_pad = (row_avail.saturating_sub(actual_total)) / 2;
            wrap_margin + natural_pad
        } else {
            (area.width.saturating_sub(actual_total)) / 2
        };
        let max_jobs_in_row = row_stages.iter().map(|s| s.jobs.len()).max().unwrap_or(0);
        let row_height = max_jobs_in_row as u16 + 2;

        let mut rx = area.x + left_pad;
        for (i, &w) in row_widths.iter().enumerate() {
            stages.push(StageLayout {
                x: rx,
                y,
                width: w,
                height: row_height,
            });
            rx += w;
            if i < row_widths.len() - 1 {
                rx += CONNECTOR_WIDTH;
            }
        }

        y += row_height;
        if row_end < num_stages {
            y += 1;
        }
        row_start = row_end;
        row_index += 1;
    }

    let total_height = y.saturating_sub(area.y);
    LayoutInfo {
        stages,
        total_height,
    }
}

fn render_stage(
    buf: &mut Buffer,
    stage: &Stage,
    sl: &StageLayout,
    selected: Option<usize>,
    highlight_style: Style,
    tick: u8,
    bounds: Rect,
) {
    let x = sl.x;
    let y = sl.y;
    let width = sl.width;
    let height = sl.height;

    if width < 6 || height < 2 {
        return;
    }

    let is_selected_stage = selected.is_some();
    let stage_status = stage.status();
    let mixed_failure = stage.has_mixed_failure();
    let status_fg = if mixed_failure {
        theme::INFO
    } else {
        status_color(stage_status)
    };
    let border_color = if is_selected_stage {
        status_fg
    } else {
        theme::BORDER
    };
    let border_style = Style::default().fg(border_color);
    let inner_w = (width - 2) as usize;
    let bottom_y = y + height - 1;

    if y >= bounds.top() && y < bounds.bottom() {
        let icon = stage_status.animated_symbol(tick);
        let header_text = format!("{} {}", icon, stage.name);
        let max_header = inner_w.saturating_sub(3);
        let header = truncate_str(&header_text, max_header);
        let fill_count = inner_w.saturating_sub(header.len() + 3);

        buf.set_string(x, y, "╭─", border_style);
        buf.set_string(x + 2, y, " ", border_style);
        let header_x = x + 3;
        let header_fg = if mixed_failure {
            theme::INFO
        } else {
            status_color(stage_status)
        };
        if stage_status.is_active() {
            render_sweep_text(
                buf,
                header_x,
                y,
                &header,
                stage_status,
                tick,
                header.len() as u16,
            );
        } else {
            buf.set_string(header_x, y, &header, Style::default().fg(header_fg));
        }
        let after_header = header_x + header.len() as u16;
        buf.set_string(after_header, y, " ", border_style);
        if fill_count > 0 {
            let fill: String = "─".repeat(fill_count);
            buf.set_string(after_header + 1, y, &fill, border_style);
        }
        if x + width - 1 < bounds.right() {
            buf.set_string(x + width - 1, y, "╮", border_style);
        }
    }

    // ── Bottom border: ╰──...──╯ ──
    if bottom_y >= bounds.top() && bottom_y < bounds.bottom() {
        buf.set_string(x, bottom_y, "╰", border_style);
        let fill: String = "─".repeat(inner_w);
        buf.set_string(x + 1, bottom_y, &fill, border_style);
        if x + width - 1 < bounds.right() {
            buf.set_string(x + width - 1, bottom_y, "╯", border_style);
        }
    }

    // ── Side borders ──
    for iy in (y + 1).max(bounds.top())..bottom_y.min(bounds.bottom()) {
        buf.set_string(x, iy, "│", border_style);
        if x + width - 1 < bounds.right() {
            buf.set_string(x + width - 1, iy, "│", border_style);
        }
    }

    // ── Jobs ──
    let interior_lines = (height - 2) as usize;
    let (visible_jobs, show_more) = if stage.jobs.len() > interior_lines && interior_lines > 0 {
        (interior_lines - 1, true)
    } else {
        (stage.jobs.len().min(interior_lines), false)
    };

    for (job_idx, job) in stage.jobs.iter().enumerate().take(visible_jobs) {
        let job_y = y + 1 + job_idx as u16;
        if job_y >= bottom_y || job_y >= bounds.bottom() {
            break;
        }
        if job_y < bounds.top() {
            continue;
        }

        let is_selected = selected == Some(job_idx);
        let max_name = inner_w.saturating_sub(3);
        let job_name = truncate_str(&job.name, max_name);
        let symbol = job.status.animated_symbol(tick);
        let job_text = if is_selected {
            format!(" ▸ {}", job_name)
        } else {
            format!(" {} {}", symbol, job_name)
        };
        let padded = format!("{:<width$}", job_text, width = inner_w);

        if job.status.is_active() {
            render_sweep_text(buf, x + 1, job_y, &padded, job.status, tick, inner_w as u16);
            if is_selected {
                buf.set_style(
                    Rect {
                        x: x + 1,
                        y: job_y,
                        width: inner_w as u16,
                        height: 1,
                    },
                    highlight_style,
                );
            }
        } else {
            let job_style = Style::default().fg(status_color(job.status));
            let line_style = if is_selected {
                highlight_style.patch(job_style)
            } else {
                job_style
            };
            buf.set_string(x + 1, job_y, &padded, line_style);
        }
    }

    if show_more {
        let more_y = y + 1 + visible_jobs as u16;
        if more_y < bottom_y && more_y >= bounds.top() && more_y < bounds.bottom() {
            let remaining = stage.jobs.len() - visible_jobs;
            let more_text = format!(" +{} more", remaining);
            let padded = format!("{:<width$}", more_text, width = inner_w);
            buf.set_string(x + 1, more_y, &padded, Style::default().fg(theme::TEXT_DIM));
        }
    }
}

fn render_wrap_connectors(
    buf: &mut Buffer,
    layout: &LayoutInfo,
    scroll_y: u16,
    content_area: Rect,
    style: Style,
    selected_stage: usize,
    stages: &[Stage],
) {
    if layout.stages.len() < 2 {
        return;
    }

    let in_view = |vy: u16| -> Option<u16> {
        let off = vy.saturating_sub(content_area.y);
        if off >= scroll_y && off < scroll_y + content_area.height {
            Some(content_area.y + off - scroll_y)
        } else {
            None
        }
    };

    for i in 0..layout.stages.len() - 1 {
        let cur = &layout.stages[i];
        let next = &layout.stages[i + 1];
        if next.y == cur.y {
            continue;
        }

        let is_src_selected = i == selected_stage;
        let src_status_color = stages
            .get(i)
            .map(|s| {
                if s.has_mixed_failure() {
                    theme::INFO
                } else {
                    status_color(s.status())
                }
            })
            .unwrap_or(theme::BORDER);
        let dst_status_color = stages
            .get(i + 1)
            .map(|s| {
                if s.has_mixed_failure() {
                    theme::INFO
                } else {
                    status_color(s.status())
                }
            })
            .unwrap_or(theme::BORDER);
        let active_style = if is_src_selected {
            Style::default().fg(src_status_color)
        } else {
            style
        };

        let tee_x = (cur.x + cur.width).saturating_sub(5).max(cur.x + 1);
        let tee_y = cur.y + cur.height - 1;
        let turn_y = tee_y + 1;

        let left_x = next.x.saturating_sub(3).max(content_area.x);
        let conn_target_y = next.y + next.height / 2;

        if let Some(sy) = in_view(tee_y) {
            if tee_x < content_area.right() {
                buf.set_string(tee_x, sy, "┬", active_style);
            }
        }

        if let Some(sy) = in_view(turn_y) {
            if left_x < content_area.right() {
                buf.set_string(left_x, sy, "╭", active_style);
            }
            for hx in (left_x + 1)..tee_x.min(content_area.right()) {
                buf.set_string(hx, sy, "─", active_style);
            }
            if tee_x < content_area.right() {
                buf.set_string(tee_x, sy, "╯", active_style);
            }
        }

        for vy in (turn_y + 1)..conn_target_y {
            if let Some(sy) = in_view(vy) {
                if left_x < content_area.right() {
                    buf.set_string(left_x, sy, "│", active_style);
                }
            }
        }

        if let Some(sy) = in_view(conn_target_y) {
            if left_x < content_area.right() {
                buf.set_string(left_x, sy, "╰", active_style);
            }
            let arrow_x = next.x.saturating_sub(1);
            for hx in (left_x + 1)..arrow_x.min(content_area.right()) {
                buf.set_string(hx, sy, "─", active_style);
            }
            if arrow_x < content_area.right() && arrow_x > left_x {
                buf.set_string(arrow_x, sy, "→", active_style);
            }
            let is_dst_selected = i + 1 == selected_stage;
            let dst_conn_style = if is_dst_selected {
                Style::default().fg(dst_status_color)
            } else {
                style
            };
            if next.x < content_area.right() {
                buf.set_string(next.x, sy, "┤", dst_conn_style);
            }
        }
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

        if state.loading && state.details.is_none() {
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

        // Clamp scroll_y to valid range
        let max_scroll = layout.total_height.saturating_sub(content_area.height);
        if state.scroll_y > max_scroll {
            state.scroll_y = max_scroll;
        }
        let scroll_y = state.scroll_y;

        let conn_style = Style::default().fg(theme::BORDER);

        for (stage_idx, stage) in details.stages.iter().enumerate() {
            if stage_idx >= layout.stages.len() {
                break;
            }
            let sl = &layout.stages[stage_idx];

            // Apply scroll offset
            let virtual_offset = sl.y.saturating_sub(content_area.y);
            if virtual_offset + sl.height <= scroll_y {
                continue; // entirely above viewport
            }
            if virtual_offset >= scroll_y + content_area.height {
                break; // entirely below viewport
            }
            let scrolled_y = content_area.y + virtual_offset - scroll_y;

            let scrolled_sl = StageLayout {
                x: sl.x,
                y: scrolled_y,
                width: sl.width,
                height: sl.height,
            };

            let selected = if stage_idx == state.selected_stage {
                Some(state.selected_job)
            } else {
                None
            };
            render_stage(
                buf,
                stage,
                &scrolled_sl,
                selected,
                self.highlight_style,
                state.animation_tick,
                content_area,
            );

            // Same-row connectors: wall-connected ├──→──┤
            if stage_idx + 1 < layout.stages.len() {
                let next = &layout.stages[stage_idx + 1];
                if next.y == sl.y {
                    let conn_y_virtual = sl.y + sl.height / 2;
                    let conn_offset = conn_y_virtual.saturating_sub(content_area.y);
                    if conn_offset >= scroll_y && conn_offset < scroll_y + content_area.height {
                        let cy = content_area.y + conn_offset - scroll_y;
                        let right_border_x = sl.x + sl.width - 1;
                        let next_virtual_offset = next.y.saturating_sub(content_area.y);
                        let next_scrolled_y = content_area.y + next_virtual_offset - scroll_y;
                        let left_border_x = next.x;
                        let is_src_selected = stage_idx == state.selected_stage;
                        let is_dst_selected = stage_idx + 1 == state.selected_stage;
                        let src_status_color = if stage.has_mixed_failure() {
                            theme::INFO
                        } else {
                            status_color(stage.status())
                        };
                        let dst_status_color = details
                            .stages
                            .get(stage_idx + 1)
                            .map(|s| {
                                if s.has_mixed_failure() {
                                    theme::INFO
                                } else {
                                    status_color(s.status())
                                }
                            })
                            .unwrap_or(theme::BORDER);
                        let src_border = Style::default().fg(if is_src_selected {
                            src_status_color
                        } else {
                            theme::BORDER
                        });
                        let dst_border = Style::default().fg(if is_dst_selected {
                            dst_status_color
                        } else {
                            theme::BORDER
                        });

                        if right_border_x < content_area.right() {
                            buf.set_string(right_border_x, cy, "├", src_border);
                        }
                        let conn_x = right_border_x + 1;
                        let mid_style = if is_src_selected {
                            Style::default().fg(src_status_color)
                        } else {
                            conn_style
                        };
                        if conn_x + CONNECTOR_WIDTH <= content_area.right() {
                            buf.set_string(conn_x, cy, "────→", mid_style);
                        }
                        // ┤ on left border of destination box
                        if left_border_x < content_area.right()
                            && next_scrolled_y <= cy
                            && cy < next_scrolled_y + next.height
                        {
                            buf.set_string(left_border_x, cy, "┤", dst_border);
                        }
                    }
                }
            }
        }

        // Multi-row wrap connectors
        render_wrap_connectors(
            buf,
            &layout,
            scroll_y,
            content_area,
            conn_style,
            state.selected_stage,
            &details.stages,
        );

        if let Some(pipeline) = &details.pipeline {
            let running_indicator = if state.is_running() { " ⟳" } else { "" };
            let status_text = format!(
                "Pipeline #{} - {}{}",
                pipeline.id, pipeline.status, running_indicator
            );
            let status_line_y = inner_area.bottom().saturating_sub(1);
            let display_text = truncate_str(&status_text, inner_area.width as usize);
            if pipeline.status.is_active() {
                render_sweep_text(
                    buf,
                    inner_area.left(),
                    status_line_y,
                    &display_text,
                    pipeline.status,
                    state.animation_tick,
                    inner_area.width,
                );
            } else {
                buf.set_string(
                    inner_area.left(),
                    status_line_y,
                    &display_text,
                    Style::default().fg(status_color(pipeline.status)),
                );
            }
        }
    }
}

impl Widget for PipelineView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = PipelineViewState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
