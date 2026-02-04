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
const SCROLLBAR_STR: &str = "\u{2588}";

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

    pub fn row_count(&self, available_width: u16) -> u16 {
        let details = match &self.details {
            Some(d) if !d.stages.is_empty() => d,
            _ => return 0,
        };
        let num_stages = details.stages.len();
        let ideal_widths: Vec<u16> = details.stages.iter().map(calculate_stage_width).collect();
        let connector_total = (num_stages as u16).saturating_sub(1) * CONNECTOR_WIDTH;
        let total_ideal: u16 = ideal_widths.iter().copied().sum::<u16>() + connector_total;

        if total_ideal <= available_width {
            return 1;
        }

        let avail_for_stages = available_width.saturating_sub(connector_total);
        let sum_ideal: u16 = ideal_widths.iter().copied().sum();
        if sum_ideal > 0 && avail_for_stages >= num_stages as u16 * MIN_STAGE_WIDTH {
            let shrunk_total: u16 = ideal_widths
                .iter()
                .map(|&w| {
                    ((w as u32 * avail_for_stages as u32) / sum_ideal as u32)
                        .max(MIN_STAGE_WIDTH as u32) as u16
                })
                .sum::<u16>()
                + connector_total;
            if shrunk_total <= available_width {
                return 1;
            }
        }

        let mut row_start = 0;
        let mut rows = 0u16;
        while row_start < num_stages {
            let mut row_end = row_start;
            let mut row_width: u16 = 0;
            let wrap_margin: u16 = if rows > 0 { 3 } else { 0 };
            let row_avail = available_width.saturating_sub(wrap_margin);
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
            rows += 1;
            row_start = row_end;
        }
        rows
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

fn status_color_dimmed(status: PipelineStatus) -> ratatui::style::Color {
    let (r, g, b) = status_base_rgb(status);
    ratatui::style::Color::Rgb(
        (r as f32 * 0.75) as u8,
        (g as f32 * 0.75) as u8,
        (b as f32 * 0.75) as u8,
    )
}

fn sweep_color(
    base_r: u8,
    base_g: u8,
    base_b: u8,
    char_pos: u16,
    total_chars: u16,
    tick: u8,
) -> ratatui::style::Color {
    let phase = (tick.wrapping_mul(8) as f32 / 255.0) * 2.0;
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

/// Compute the perimeter position for a given (bx, by) cell of a box at (x, y, w, h).
/// Returns None if the cell is not on the border.
/// Perimeter layout (clockwise from top-left):
///   Top: 0..w-1, Right: w-1..w+h-2, Bottom: w+h-2..2w+h-3, Left: 2w+h-3..2(w+h-2)
fn perimeter_pos(bx: u16, by: u16, x: u16, y: u16, w: u16, h: u16) -> Option<u16> {
    if bx < x || bx >= x + w || by < y || by >= y + h {
        return None;
    }
    let right = x + w - 1;
    let bottom = y + h - 1;

    if by == y {
        // Top border: left to right
        Some(bx - x)
    } else if bx == right && by > y && by < bottom {
        // Right border: top to bottom (excluding corners)
        Some((w - 1) + (by - y))
    } else if by == bottom {
        // Bottom border: right to left
        Some((w - 1) + (h - 1) + (right - bx))
    } else if bx == x && by > y && by < bottom {
        // Left border: bottom to top (excluding corners)
        Some(2 * (w - 1) + (h - 1) + (bottom - by))
    } else {
        None
    }
}

/// Compute glow intensity (0.0–1.0) for a border cell at perimeter position `pos`.
/// Two lights start at `name_pos` and travel in opposite directions to `conn_pos`,
/// arriving simultaneously. Cycle: 16 ticks (3.2s at 200ms).
fn border_anim_glow(pos: u16, perim_len: u16, name_pos: u16, conn_pos: u16, tick: u8) -> f32 {
    if perim_len == 0 {
        return 0.0;
    }
    let p = perim_len as f32;
    let cycle = (tick % 16) as f32 / 16.0;

    // CW distance from name to connector
    let cw_dist = ((conn_pos as i32 - name_pos as i32).rem_euclid(perim_len as i32)) as f32;
    // CCW distance = perim - cw
    let ccw_dist = p - cw_dist;

    let radius = 3.0_f32;

    if cycle < 0.75 {
        // Travel phase: lights moving from name to connector
        let travel_t = cycle / 0.75;

        // CW light position (fractional perimeter position)
        let cw_light = (name_pos as f32 + cw_dist * travel_t) % p;
        // CCW light position
        let ccw_light = ((name_pos as f32 - ccw_dist * travel_t) % p + p) % p;

        let pos_f = pos as f32;

        // Circular distance to CW light
        let d_cw = (pos_f - cw_light).abs().min(p - (pos_f - cw_light).abs());
        // Circular distance to CCW light
        let d_ccw = (pos_f - ccw_light).abs().min(p - (pos_f - ccw_light).abs());

        let glow_cw = ((1.0 - d_cw / radius).max(0.0)).powi(2);
        let glow_ccw = ((1.0 - d_ccw / radius).max(0.0)).powi(2);

        glow_cw.max(glow_ccw)
    } else {
        // Arrival/fade phase: both lights at connector, fading
        let fade = 1.0 - (cycle - 0.75) / 0.25;
        let pos_f = pos as f32;
        let conn_f = conn_pos as f32;
        let d = (pos_f - conn_f).abs().min(p - (pos_f - conn_f).abs());
        let glow = ((1.0 - d / radius).max(0.0)).powi(2);
        glow * fade
    }
}

/// Compute glow intensity for a connector arrow character between stages.
/// Active during the arrival phase (0.75–1.0 of the cycle).
/// Light flows from ├ (position 0) to ┤ (position arrow_len-1).
fn arrow_anim_glow(arrow_pos: u16, arrow_len: u16, tick: u8) -> f32 {
    if arrow_len == 0 {
        return 0.0;
    }
    let cycle = (tick % 16) as f32 / 16.0;
    if cycle < 0.75 {
        return 0.0;
    }
    let arrow_t = (cycle - 0.75) / 0.25; // 0.0 to 1.0 over the arrow phase
    let light_pos = arrow_t * (arrow_len as f32 - 1.0);
    let dist = (arrow_pos as f32 - light_pos).abs();
    let radius = 2.5_f32;
    ((1.0 - dist / radius).max(0.0)).powi(2)
}

/// Post-render overlay: iterates over all border cells of an active stage box
/// and applies glow coloring based on the border animation.
fn apply_border_glow(
    buf: &mut Buffer,
    sl: &StageLayout,
    stage_status: PipelineStatus,
    header_len: u16,
    conn_pos: u16,
    tick: u8,
    bounds: Rect,
) {
    if !stage_status.is_active() {
        return;
    }

    let x = sl.x;
    let y = sl.y;
    let w = sl.width;
    let h = sl.height;
    if w < 4 || h < 3 {
        return;
    }

    let perim_len = 2 * (w - 1) + 2 * (h - 1);
    let name_pos = 3 + header_len / 2;

    let (r, g, b) = status_base_rgb(stage_status);
    let base_r = (r as f32 * 0.75) as u8;
    let base_g = (g as f32 * 0.75) as u8;
    let base_b = (b as f32 * 0.75) as u8;

    let mix = |base: u8, target: u8, t: f32| -> u8 {
        (base as f32 + (target as f32 - base as f32) * t).clamp(0.0, 255.0) as u8
    };

    // Iterate over all border cells
    for by in y..y + h {
        if by < bounds.top() || by >= bounds.bottom() {
            continue;
        }
        for bx in x..x + w {
            if bx >= bounds.right() {
                break;
            }
            // Only process border cells
            let is_border = by == y || by == y + h - 1 || bx == x || bx == x + w - 1;
            if !is_border {
                continue;
            }

            // Skip the header text area on the top border to preserve sweep text effect
            if by == y && bx >= x + 3 && bx < x + 3 + header_len {
                continue;
            }

            if let Some(pos) = perimeter_pos(bx, by, x, y, w, h) {
                let glow = border_anim_glow(pos, perim_len, name_pos, conn_pos, tick);
                if glow > 0.01 {
                    let r = mix(base_r, 236, glow);
                    let g = mix(base_g, 239, glow);
                    let b = mix(base_b, 244, glow);
                    let cell = buf.cell_mut((bx, by));
                    if let Some(cell) = cell {
                        cell.set_style(
                            Style::default()
                                .fg(Color::Rgb(r, g, b))
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        }
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
    let border_color = if is_selected_stage && stage_status.is_active() {
        let (r, g, b) = status_base_rgb(stage_status);
        Color::Rgb(
            (r as f32 * 0.75) as u8,
            (g as f32 * 0.75) as u8,
            (b as f32 * 0.75) as u8,
        )
    } else if is_selected_stage {
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

#[allow(clippy::too_many_arguments)]
fn render_wrap_connectors(
    buf: &mut Buffer,
    layout: &LayoutInfo,
    scroll_y: u16,
    content_area: Rect,
    style: Style,
    selected_stage: usize,
    stages: &[Stage],
    has_running_stage: bool,
    tick: u8,
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
                } else if s.status().is_active() {
                    status_color_dimmed(s.status())
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
                } else if s.status().is_active() {
                    status_color_dimmed(s.status())
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
                buf.set_string(arrow_x, sy, "▶", active_style);
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

        // Wrap connector glow for active source stage
        let src_status = stages.get(i).map(|s| s.status());
        let glow_wrap = src_status.is_some_and(|st| {
            if has_running_stage {
                st == PipelineStatus::Running
            } else {
                st.is_active()
            }
        });
        if glow_wrap {
            let st = src_status.unwrap();
            let (r, g, b) = status_base_rgb(st);
            let base_r = (r as f32 * 0.75) as u8;
            let base_g = (g as f32 * 0.75) as u8;
            let base_b = (b as f32 * 0.75) as u8;

            // Segment 1: ┬ → ╯ → ─(right-to-left) → ╭
            let mut seg1: Vec<(u16, u16)> = Vec::new();
            if let Some(sy) = in_view(tee_y) {
                if tee_x < content_area.right() {
                    seg1.push((tee_x, sy));
                }
            }
            if let Some(sy) = in_view(turn_y) {
                if tee_x < content_area.right() {
                    seg1.push((tee_x, sy));
                }
                for hx in ((left_x + 1)..tee_x.min(content_area.right())).rev() {
                    seg1.push((hx, sy));
                }
                if left_x < content_area.right() {
                    seg1.push((left_x, sy));
                }
            }
            // Segment 2: │'s → ╰
            let mut seg2: Vec<(u16, u16)> = Vec::new();
            for vy in (turn_y + 1)..conn_target_y {
                if let Some(sy) = in_view(vy) {
                    if left_x < content_area.right() {
                        seg2.push((left_x, sy));
                    }
                }
            }
            if let Some(sy) = in_view(conn_target_y) {
                if left_x < content_area.right() {
                    seg2.push((left_x, sy));
                }
            }
            // Segment 3: ─'s → ▶ → ┤
            let mut seg3: Vec<(u16, u16)> = Vec::new();
            if let Some(sy) = in_view(conn_target_y) {
                let arrow_x = next.x.saturating_sub(1);
                for hx in (left_x + 1)..arrow_x.min(content_area.right()) {
                    seg3.push((hx, sy));
                }
                if arrow_x < content_area.right() && arrow_x > left_x {
                    seg3.push((arrow_x, sy));
                }
                if next.x < content_area.right() {
                    seg3.push((next.x, sy));
                }
            }

            let segments = [&seg1, &seg2, &seg3];
            let seg_lens: Vec<usize> = segments.iter().map(|s| s.len()).collect();
            let total: usize = seg_lens.iter().sum();
            if total > 0 {
                let mix = |base: u8, target: u8, t: f32| -> u8 {
                    (base as f32 + (target as f32 - base as f32) * t).clamp(0.0, 255.0) as u8
                };
                let cycle = (tick % 16) as f32 / 16.0;
                if cycle >= 0.75 {
                    let flow_t = (cycle - 0.75) / 0.25;
                    // Fixed time: 50% horizontal, 25% vertical, 25% arrow
                    let time_slots: [f32; 3] = [0.5, 0.25, 0.25];
                    let mut t_start = 0.0_f32;
                    for (seg_idx, seg) in segments.iter().enumerate() {
                        let seg_len = seg_lens[seg_idx];
                        if seg_len == 0 {
                            t_start += time_slots[seg_idx];
                            continue;
                        }
                        let t_frac = time_slots[seg_idx];
                        let t_end = t_start + t_frac;
                        let radius = if seg_idx == 0 {
                            (seg_len as f32 / 3.5).max(3.0)
                        } else {
                            2.5
                        };
                        if flow_t >= t_start && flow_t < t_end {
                            let local_t = (flow_t - t_start) / t_frac;
                            let light_pos = local_t * (seg_len as f32 - 1.0);
                            for (idx, &(cx, cy)) in seg.iter().enumerate() {
                                let dist = (idx as f32 - light_pos).abs();
                                let glow = ((1.0 - dist / radius).max(0.0)).powi(2);
                                if glow > 0.01 {
                                    let nr = mix(base_r, 236, glow);
                                    let ng = mix(base_g, 239, glow);
                                    let nb = mix(base_b, 244, glow);
                                    if let Some(cell) = buf.cell_mut((cx, cy)) {
                                        cell.set_style(
                                            Style::default()
                                                .fg(Color::Rgb(nr, ng, nb))
                                                .add_modifier(Modifier::BOLD),
                                        );
                                    }
                                }
                            }
                        }
                        t_start = t_end;
                    }
                }
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

        if let Some(sl) = layout.stages.get(state.selected_stage) {
            let stage_top = sl.y.saturating_sub(content_area.y);
            let stage_bottom = stage_top + sl.height;
            if stage_bottom > state.scroll_y + content_area.height {
                state.scroll_y = stage_bottom.saturating_sub(content_area.height);
            }
            if stage_top < state.scroll_y + 1 {
                state.scroll_y = stage_top.saturating_sub(1);
            }
        }

        let max_scroll = layout.total_height.saturating_sub(content_area.height);
        if state.scroll_y > max_scroll {
            state.scroll_y = max_scroll;
        }
        let scroll_y = state.scroll_y;

        let conn_style = Style::default().fg(theme::BORDER);
        let has_running_stage = details
            .stages
            .iter()
            .any(|s| s.status() == PipelineStatus::Running);

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

            let stage_status = stage.status();
            let glow_this_stage = if has_running_stage {
                stage_status == PipelineStatus::Running
            } else {
                stage_status.is_active()
            };
            if glow_this_stage {
                let icon = stage_status.animated_symbol(state.animation_tick);
                let header_text = format!("{} {}", icon, stage.name);
                let inner_w = (scrolled_sl.width - 2) as usize;
                let max_header = inner_w.saturating_sub(3);
                let header = truncate_str(&header_text, max_header);

                // Determine connector position: right border (├) for same-row,
                // bottom border (┬) for wrap
                let w = sl.width;
                let h = sl.height;
                let next_wraps =
                    stage_idx + 1 < layout.stages.len() && layout.stages[stage_idx + 1].y != sl.y;
                let conn_pos = if next_wraps {
                    // ┬ on bottom border: tee_x offset from right
                    let tee_rel = (w).saturating_sub(5).max(1);
                    // Bottom border runs right-to-left: perim pos = (w-1)+(h-1)+(w-1-tee_rel)
                    (w - 1) + (h - 1) + (w - 1 - tee_rel)
                } else {
                    // ├ at middle of right border
                    (w - 1) + h / 2
                };

                apply_border_glow(
                    buf,
                    &scrolled_sl,
                    stage_status,
                    header.len() as u16,
                    conn_pos,
                    state.animation_tick,
                    content_area,
                );
            }

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
                        } else if stage.status().is_active() {
                            status_color_dimmed(stage.status())
                        } else {
                            status_color(stage.status())
                        };
                        let dst_status_color = details
                            .stages
                            .get(stage_idx + 1)
                            .map(|s| {
                                if s.has_mixed_failure() {
                                    theme::INFO
                                } else if s.status().is_active() {
                                    status_color_dimmed(s.status())
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
                            buf.set_string(conn_x, cy, "────▶", mid_style);
                        }
                        // ┤ on left border of destination box
                        if left_border_x < content_area.right()
                            && next_scrolled_y <= cy
                            && cy < next_scrolled_y + next.height
                        {
                            buf.set_string(left_border_x, cy, "┤", dst_border);
                        }

                        // Arrow glow: ├────→┤ for active source stages
                        let arrow_glow_this = if has_running_stage {
                            stage.status() == PipelineStatus::Running
                        } else {
                            stage.status().is_active()
                        };
                        if arrow_glow_this {
                            let arrow_total = CONNECTOR_WIDTH + 2; // ├ + ──▶ + ┤
                            let (r, g, b) = status_base_rgb(stage.status());
                            let ar = (r as f32 * 0.75) as u8;
                            let ag = (g as f32 * 0.75) as u8;
                            let ab = (b as f32 * 0.75) as u8;
                            let mix = |base: u8, target: u8, t: f32| -> u8 {
                                (base as f32 + (target as f32 - base as f32) * t).clamp(0.0, 255.0)
                                    as u8
                            };
                            for ap in 0..arrow_total {
                                let ax = right_border_x + ap;
                                if ax >= content_area.right() {
                                    break;
                                }
                                let glow = arrow_anim_glow(ap, arrow_total, state.animation_tick);
                                if glow > 0.01 {
                                    let r = mix(ar, 236, glow);
                                    let g = mix(ag, 239, glow);
                                    let b = mix(ab, 244, glow);
                                    let cell = buf.cell_mut((ax, cy));
                                    if let Some(cell) = cell {
                                        cell.set_style(
                                            Style::default()
                                                .fg(Color::Rgb(r, g, b))
                                                .add_modifier(Modifier::BOLD),
                                        );
                                    }
                                }
                            }
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
            has_running_stage,
            state.animation_tick,
        );

        if layout.total_height > content_area.height && content_area.height > 0 {
            let track_height = content_area.height as usize;
            let total = layout.total_height as usize;
            let thumb_height =
                ((track_height * track_height) as f32 / total as f32).floor() as usize;
            let thumb_height = thumb_height.clamp(1, track_height);
            let thumb_start = content_area.top() as usize
                + ((track_height * scroll_y as usize) as f32 / total as f32).ceil() as usize;
            let scroll_x = content_area.right();
            for y in thumb_start..(thumb_start + thumb_height).min(content_area.bottom() as usize) {
                buf.set_string(
                    scroll_x,
                    y as u16,
                    SCROLLBAR_STR,
                    Style::default().fg(theme::TEXT_DIM),
                );
            }
        }

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
