//! UI rendering for TUI using new widget system

use crate::permissions::PermissionResult;
use crate::provider::{ContentBlock, Message};
use crate::tui::app::App;
use crate::tui::completion::CompletionType;
use crate::tui::history_cell::HistoryCell;
use crate::tui::permission_dialog::PermissionDialog;
use crate::tui::transcript_cache::TranscriptViewCache;
use crate::tui::widgets::{ChatWidget, ChatTheme, HeaderWidget, HeaderData, FooterWidget, FooterProps};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};

/// Render the TUI with new widget system
pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.size();

    // Calculate layout areas
    let header_height: u16 = 2; // Header + border
    let input_lines = app.editor.lines().len();
    let input_height: u16 = (input_lines + 2).min(8).max(3) as u16;
    let footer_height: u16 = 1;

    // Progress area height (show when active)
    let progress_height: u16 = if app.progress_manager.is_active() {
        let tool_count = app.progress_manager.active_tool_count();
        (1 + tool_count).min(5) as u16
    } else {
        0
    };

    // Completion popup height (when visible)
    let completion_height: u16 = if app.completion_state.is_visible() {
        (app.completion_state.count() + 2).min(10) as u16
    } else {
        0
    };

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(header_height),    // Header
            Constraint::Min(3),                   // Chat/Messages area
            Constraint::Length(progress_height),  // Progress area (dynamic)
            Constraint::Length(input_height),     // Input area
            Constraint::Length(footer_height),    // Footer/Status bar
        ])
        .split(area);

    // Render header widget
    render_header(f, app, chunks[0]);

    // Render chat widget (messages area) - using pre-computed cache
    render_chat(f, app, chunks[1]);

    // Render progress area (if active)
    if progress_height > 0 && chunks[2].height > 0 {
        render_progress(f, app, chunks[2]);
    }

    // Render input area
    render_input(f, app, chunks[3]);

    // Render completion popup (above input area)
    if completion_height > 0 {
        render_completion(f, app, chunks[3]);
    }

    // Render footer widget
    render_footer(f, app, chunks[4]);

    // Render permission dialog if pending
    if let Some(permission) = &app.pending_permission {
        if let Some(dialog) = &app.permission_dialog {
            dialog.render(f);
        } else {
            let result = PermissionResult::ask(&permission.message);
            let temp_dialog = PermissionDialog::new(permission.message.clone(), result);
            temp_dialog.render(f);
        }
    }
}

/// Render header widget
fn render_header(f: &mut Frame, app: &App, area: Rect) {
    // Determine status frame based on streaming state
    let status_frame = if app.is_streaming {
        Some("●".to_string())
    } else if app.is_loading {
        Some("⋯".to_string())
    } else {
        None
    };

    // Get workspace name
    let workspace = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "/".to_string());

    let header_data = HeaderData {
        model: app.model_name.clone(),
        provider: app.provider_name.clone(),
        workspace,
        is_streaming: app.is_streaming,
        status_frame,
    };

    let header = HeaderWidget::new(header_data);
    f.render_widget(header, area);
}

/// Render chat widget with history cells (uses pre-computed cache)
fn render_chat(f: &mut Frame, app: &mut App, area: Rect) {
    // Build theme
    let theme = ChatTheme {
        background: Color::Reset,
        scroll_track: Color::DarkGray,
        scroll_thumb: Color::Gray,
        jump_border: Color::Gray,
        jump_arrow: Color::Cyan,
    };

    // Use history_cells directly for rendering
    // Filter out system cells (internal prompts not meant for display)
    let display_cells: Vec<HistoryCell> = app.history_cells
        .iter()
        .filter(|c| !c.is_empty() && !matches!(c, HistoryCell::System { .. }))
        .cloned()
        .collect();

    // Build revisions for display cells
    let revisions: Vec<u64> = display_cells.iter().map(|_| 1).collect();

    // First compute total lines to resolve scroll
    let cell_lines: Vec<Vec<Line<'static>>> = display_cells
        .iter()
        .map(|c| c.lines(area.width))
        .collect();
    let total_lines: usize = cell_lines.iter().map(|l| l.len()).sum();
    let visible_lines = area.height as usize;

    // Resolve scroll with pending delta applied
    let resolved_scroll = app.resolve_scroll_for_render(total_lines, visible_lines);

    // Now build cache and get visible lines
    let mut cache = TranscriptViewCache::new();
    cache.ensure(&cell_lines, &revisions, area.width);

    // Get visible lines based on resolved scroll
    let (final_scroll, top) = resolved_scroll.resolve_top(total_lines, visible_lines);
    let visible_content = cache.visible_lines(top, visible_lines);

    // Create chat widget from visible lines
    let chat = ChatWidget::from_lines(visible_content, area, &theme);
    f.render_widget(chat, area);
}

/// Render progress area
fn render_progress(f: &mut Frame, app: &App, area: Rect) {
    let progress_lines = app.progress_manager.render(area.width as usize);

    let block = Block::default().borders(Borders::NONE);
    let paragraph = Paragraph::new(progress_lines).block(block);

    f.render_widget(paragraph, area);
}

/// Render input area
fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.is_loading {
        " Waiting for response... "
    } else {
        " Input (Enter=send, Ctrl+Enter=newline) "
    };

    let style = if app.is_loading {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::White)
    };

    let border_style = if app.is_loading {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::Green)
    };

    // Build text with all lines
    let text_lines: Vec<Line> = app.editor.lines()
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), style)))
        .collect();

    let input_widget = Paragraph::new(text_lines)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(border_style),
        );

    f.render_widget(input_widget, area);

    // Show cursor when not loading
    if !app.is_loading {
        let (cursor_row, _cursor_col) = app.editor.cursor_position();
        let cursor_display_x = app.editor.cursor_display_x();

        let cursor_x = area.x + 1 + cursor_display_x as u16;
        let cursor_y = area.y + 1 + cursor_row as u16;

        if cursor_y < area.y + area.height - 1 {
            f.set_cursor(cursor_x, cursor_y);
        }
    }
}

/// Render footer widget
fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    // Build footer props based on app state
    let props = if app.is_streaming {
        FooterProps {
            mode_label: "agent",
            mode_color: Color::Cyan,
            status_label: "thinking",
            status_color: Color::Yellow,
            tokens: None,
            cost: None,
            duration_secs: None,
            tool_count: Some(app.progress_manager.active_tool_count()),
            background: Color::Reset,
            text_muted: Color::DarkGray,
        }
    } else if app.is_loading {
        FooterProps {
            mode_label: "agent",
            mode_color: Color::Cyan,
            status_label: "working",
            status_color: Color::Magenta,
            tokens: None,
            cost: None,
            duration_secs: None,
            tool_count: Some(app.progress_manager.active_tool_count()),
            background: Color::Reset,
            text_muted: Color::DarkGray,
        }
    } else if app.ctrl_c_count > 0 {
        FooterProps {
            mode_label: "agent",
            mode_color: Color::Red,
            status_label: "press Ctrl+C again to quit",
            status_color: Color::Red,
            tokens: None,
            cost: None,
            duration_secs: None,
            tool_count: None,
            background: Color::Reset,
            text_muted: Color::DarkGray,
        }
    } else {
        FooterProps {
            mode_label: "agent",
            mode_color: Color::Cyan,
            status_label: "ready",
            status_color: Color::Green,
            tokens: None,
            cost: None,
            duration_secs: None,
            tool_count: None,
            background: Color::Reset,
            text_muted: Color::DarkGray,
        }
    };

    let footer = FooterWidget::new(props);
    f.render_widget(footer, area);
}

/// Render completion popup above input area
fn render_completion(f: &mut Frame, app: &App, input_area: Rect) {
    if !app.completion_state.is_visible() {
        return;
    }

    let suggestions = app.completion_state.suggestions();
    let selected_idx = app.completion_state.selected_index();

    let popup_height = (suggestions.len() + 2).min(10) as u16;
    let popup_width: u16 = 40;

    let popup_y = input_area.y.saturating_sub(popup_height);
    let popup_area = Rect {
        x: input_area.x,
        y: popup_y,
        width: popup_width.min(input_area.width),
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let lines: Vec<Line> = suggestions
        .iter()
        .enumerate()
        .map(|(i, suggestion)| {
            let is_selected = i == selected_idx;

            let base_color = match suggestion.completion_type {
                CompletionType::Command => Color::Cyan,
                CompletionType::Path => Color::Yellow,
                CompletionType::Provider => Color::Magenta,
                CompletionType::Model => Color::Green,
            };

            let style = if is_selected {
                Style::default()
                    .fg(base_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let selected_marker = if is_selected { "▶ " } else { "  " };

            let display_text = Span::styled(
                format!("{}{}", selected_marker, suggestion.display_text),
                style,
            );

            if let Some(desc) = &suggestion.description {
                let desc_text = Span::styled(
                    format!(" - {}", desc),
                    Style::default().fg(Color::DarkGray),
                );
                Line::from(vec![display_text, desc_text])
            } else {
                Line::from(display_text)
            }
        })
        .collect();

    let block = Block::default()
        .title(" Commands ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

/// Render a message as lines (legacy, kept for compatibility)
fn render_message_lines(msg: &Message) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for block in &msg.content {
        match block {
            ContentBlock::Text { text } => {
                let style = match msg.role.as_str() {
                    "user" => Style::default().fg(Color::Cyan),
                    "assistant" => Style::default().fg(Color::Green),
                    "system" => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                };

                for line_text in text.lines() {
                    let line = Line::from(Span::styled(line_text.to_string(), style));
                    lines.push(line);
                }
            }
            ContentBlock::ToolUse { id, name, input } => {
                let input_str = serde_json::to_string_pretty(&input)
                    .unwrap_or_else(|_| input.to_string());
                let header = Line::from(Span::styled(
                    format!("🔧 Tool Call: {} ({})", name, id),
                    Style::default().fg(Color::Magenta),
                ));
                lines.push(header);

                for input_line in input_str.lines() {
                    let line = Line::from(Span::styled(
                        format!("  {}", input_line),
                        Style::default().fg(Color::LightMagenta),
                    ));
                    lines.push(line);
                }
            }
            ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                let prefix = if is_error.unwrap_or(false) { "❌" } else { "✅" };
                let style = if is_error.unwrap_or(false) {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::LightGreen)
                };

                let header = Line::from(Span::styled(
                    format!("{} Tool Result: {}", prefix, tool_use_id),
                    style,
                ));
                lines.push(header);

                let display_content = if content.len() > 500 {
                    format!("{}...", &content[..500])
                } else {
                    content.clone()
                };

                for content_line in display_content.lines() {
                    let line = Line::from(Span::styled(
                        format!("  {}", content_line),
                        style,
                    ));
                    lines.push(line);
                }
            }
        }
    }

    lines
}