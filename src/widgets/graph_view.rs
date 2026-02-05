use crate::gitlab::models::PipelineStatus;
use crate::util::ctrl_chars::CtrlChars;
use crate::widgets::branches_view::BranchItem;
use crate::widgets::list::StatefulList;
use git_graph::graph::GitGraph;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use std::collections::HashMap;
use std::iter::Iterator;
use unicode_width::UnicodeWidthStr;

const SCROLL_MARGIN: usize = 3;
const SCROLLBAR_STR: &str = "\u{2588}";
const SHA_LENGTH: u16 = 7;

#[derive(Default)]
pub struct GraphViewState {
    pub graph: Option<GitGraph>,
    pub graph_lines: Vec<String>,
    pub text_lines: Vec<String>,
    pub indices: Vec<usize>,
    pub offset: usize,
    pub selected: Option<usize>,
    pub branches: Option<StatefulList<BranchItem>>,
    pub secondary_selected: Option<usize>,
    pub secondary_changed: bool,
    pub pipeline_statuses: HashMap<String, PipelineStatus>,
    pub animation_tick: u8,
}

impl GraphViewState {
    pub fn pipeline_info(&self, commit_idx: usize) -> Option<(PipelineStatus, u8)> {
        let graph = self.graph.as_ref()?;
        let commit_info = graph.commits.get(commit_idx)?;
        let sha = commit_info.oid.to_string();
        let status = self.pipeline_statuses.get(&sha)?;
        Some((*status, self.animation_tick))
    }
}

fn pipeline_base_color(status: PipelineStatus) -> (u8, u8, u8) {
    match status {
        PipelineStatus::Success => (163, 190, 140),
        PipelineStatus::Running | PipelineStatus::Pending | PipelineStatus::Preparing => {
            (94, 129, 172)
        }
        PipelineStatus::Failed => (191, 97, 106),
        PipelineStatus::Canceled | PipelineStatus::Canceling => (136, 192, 208),
        PipelineStatus::Skipped | PipelineStatus::Manual | PipelineStatus::WaitingForResource => {
            (208, 135, 112)
        }
        PipelineStatus::Created | PipelineStatus::Scheduled => (216, 222, 233),
    }
}

fn pipeline_status_to_color(status: PipelineStatus) -> Color {
    let (r, g, b) = pipeline_base_color(status);
    Color::Rgb(r, g, b)
}

fn sweep_color(
    base_r: u8,
    base_g: u8,
    base_b: u8,
    char_pos: u16,
    total_chars: u16,
    tick: u8,
) -> Color {
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

    Color::Rgb(
        mix(base_r, 236, glow),
        mix(base_g, 239, glow),
        mix(base_b, 244, glow),
    )
}

impl GraphViewState {
    pub fn commit_index_for_line(&self, line_idx: usize) -> Option<usize> {
        self.indices.iter().position(|&line| line == line_idx)
    }

    pub fn move_selection(&mut self, steps: usize, down: bool) -> bool {
        let changed = if let Some(sel) = self.selected {
            let new_idx = if down {
                std::cmp::min(
                    sel.saturating_add(steps),
                    self.indices.len().saturating_sub(1),
                )
            } else {
                sel.saturating_sub(steps)
            };
            self.selected = Some(new_idx);
            new_idx != sel
        } else if !self.graph_lines.is_empty() {
            self.selected = Some(0);
            true
        } else {
            false
        };
        if changed {
            self.secondary_changed = false;
        }
        changed
    }
    pub fn move_secondary_selection(&mut self, steps: usize, down: bool) -> bool {
        let changed = if let Some(sel) = self.secondary_selected {
            let new_idx = if down {
                std::cmp::min(
                    sel.saturating_add(steps),
                    self.indices.len().saturating_sub(1),
                )
            } else {
                sel.saturating_sub(steps)
            };
            self.secondary_selected = Some(new_idx);
            new_idx != sel
        } else if !self.graph_lines.is_empty() {
            if let Some(sel) = self.selected {
                let new_idx = if down {
                    std::cmp::min(
                        sel.saturating_add(steps),
                        self.indices.len().saturating_sub(1),
                    )
                } else {
                    sel.saturating_sub(steps)
                };
                self.secondary_selected = Some(new_idx);
                new_idx != sel
            } else {
                false
            }
        } else {
            false
        };
        if changed {
            self.secondary_changed = true;
        }
        changed
    }
}

#[derive(Default)]
pub struct GraphView<'a> {
    block: Option<Block<'a>>,
    highlight_symbol: Option<&'a str>,
    secondary_highlight_symbol: Option<&'a str>,
    style: Style,
    highlight_style: Style,
}

impl<'a> GraphView<'a> {
    pub fn block(mut self, block: Block<'a>) -> GraphView<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> GraphView<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(
        mut self,
        highlight_symbol: &'a str,
        secondary_highlight_symbol: &'a str,
    ) -> GraphView<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self.secondary_highlight_symbol = Some(secondary_highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> GraphView<'a> {
        self.highlight_style = style;
        self
    }
}

impl StatefulWidget for GraphView<'_> {
    type State = GraphViewState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        if state.graph_lines.is_empty() || state.indices.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;

        let mut start = state.offset;

        let height = std::cmp::min(
            list_height,
            state.graph_lines.len().saturating_sub(state.offset),
        );
        let mut end = start + height;

        let max_graph_idx = state.graph_lines.len().saturating_sub(1);
        let max_indices_idx = state.indices.len().saturating_sub(1);

        let selected_row = state
            .selected
            .and_then(|idx| state.indices.get(idx).copied());
        let selected = selected_row.unwrap_or(0).min(max_graph_idx);

        let secondary_selected_row = state
            .secondary_selected
            .and_then(|idx| state.indices.get(idx).copied());
        let secondary_selected = secondary_selected_row.unwrap_or(0).min(max_graph_idx);

        let selected_index = if state.secondary_changed {
            state.secondary_selected.unwrap_or(0).min(max_indices_idx)
        } else {
            state.selected.unwrap_or(0).min(max_indices_idx)
        };
        let move_to_selected = if state.secondary_changed {
            secondary_selected
        } else {
            selected
        };

        let move_to_end = if selected_index >= max_indices_idx {
            max_graph_idx
        } else {
            state.indices[selected_index + 1]
                .saturating_sub(1)
                .max(move_to_selected + SCROLL_MARGIN)
                .min(max_graph_idx)
        };
        let move_to_start = move_to_selected.saturating_sub(SCROLL_MARGIN);

        if move_to_end >= end {
            let diff = move_to_end + 1 - end;
            end += diff;
            start += diff;
        }
        if move_to_start < start {
            let diff = start - move_to_start;
            end -= diff;
            start -= diff;
        }
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let secondary_highlight_symbol = self.secondary_highlight_symbol.unwrap_or("");

        let blank_symbol = " ".repeat(highlight_symbol.width());

        let style = Style::default();
        for (current_height, (i, (graph_item, text_item))) in state
            .graph_lines
            .iter()
            .zip(state.text_lines.iter())
            .enumerate()
            .skip(state.offset)
            .take(end - start)
            .enumerate()
        {
            let (x, y) = (list_area.left(), list_area.top() + current_height as u16);

            let is_selected = selected_row.map(|s| s == i).unwrap_or(false);
            let is_sec_selected = secondary_selected_row.map(|s| s == i).unwrap_or(false);
            let elem_x = {
                let symbol = if is_selected {
                    highlight_symbol
                } else if is_sec_selected {
                    secondary_highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, list_area.width as usize, style);
                x
            };

            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: 1,
            };

            let max_element_width = (list_area.width - (elem_x - x)) as usize;

            let commit_info = state.commit_index_for_line(i).and_then(|commit_idx| {
                let (status, tick) = state.pipeline_info(commit_idx)?;
                let graph = state.graph.as_ref()?;
                let info = graph.commits.get(commit_idx)?;
                let sha = &info.oid.to_string()[..SHA_LENGTH as usize];
                Some((sha.to_string(), status, tick))
            });

            let mut body = CtrlChars::parse(graph_item).into_text();
            body.extend(CtrlChars::parse(&format!("  {}", text_item)).into_text());

            let mut x = elem_x;
            let mut remaining_width = max_element_width as u16;
            for txt in body {
                for line in txt.lines {
                    if remaining_width == 0 {
                        break;
                    }
                    let pos = buf.set_line(x, y, &line, remaining_width);
                    let w = pos.0.saturating_sub(x);
                    x = pos.0;
                    remaining_width = remaining_width.saturating_sub(w);
                }
            }

            if let Some((sha, status, tick)) = commit_info {
                for search_x in elem_x..list_area.right() {
                    let mut found = true;
                    for (offset, sha_char) in sha.chars().enumerate() {
                        let check_x = search_x + offset as u16;
                        if check_x >= list_area.right() {
                            found = false;
                            break;
                        }
                        let cell = buf.cell((check_x, y));
                        if cell.map(|c| c.symbol()) != Some(&sha_char.to_string()) {
                            found = false;
                            break;
                        }
                    }
                    if found {
                        let (base_r, base_g, base_b) = pipeline_base_color(status);
                        if status.is_active() {
                            for offset in 0..SHA_LENGTH {
                                let cx = search_x + offset;
                                if cx < list_area.right() {
                                    let color = sweep_color(
                                        base_r, base_g, base_b, offset, SHA_LENGTH, tick,
                                    );
                                    buf.set_style(
                                        Rect {
                                            x: cx,
                                            y,
                                            width: 1,
                                            height: 1,
                                        },
                                        Style::default().fg(color),
                                    );
                                }
                            }
                        } else {
                            buf.set_style(
                                Rect {
                                    x: search_x,
                                    y,
                                    width: SHA_LENGTH,
                                    height: 1,
                                },
                                Style::default().fg(pipeline_status_to_color(status)),
                            );
                        }
                        break;
                    }
                }
            }

            if is_selected || is_sec_selected {
                buf.set_style(area, self.highlight_style);
            }
        }

        let scroll_start = list_area.top() as usize
            + (((list_height * start) as f32 / state.graph_lines.len() as f32).ceil() as usize)
                .min(list_height - 1);
        let scroll_height = (((list_height * list_height) as f32 / state.graph_lines.len() as f32)
            .floor() as usize)
            .clamp(1, list_height);

        if scroll_height < list_height {
            for y in scroll_start..(scroll_start + scroll_height) {
                buf.set_string(
                    list_area.left() + list_area.width,
                    y as u16,
                    SCROLLBAR_STR,
                    self.style,
                );
            }
        }
    }
}

impl Widget for GraphView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = GraphViewState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
