//! UI rendering for TUI

use crate::permissions::PermissionResult;
use crate::provider::{ContentBlock, Message};
use crate::tui::app::App;
use crate::tui::completion::CompletionType;
use crate::tui::permission_dialog::PermissionDialog;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};

/// Render the TUI
pub fn render(f: &mut Frame, app: &App) {
    let area = f.size();

    // Calculate input area height based on content
    let input_lines = app.editor.lines().len();
    let input_height: u16 = (input_lines + 2).min(8).max(3) as u16; // Min 3, max 8 lines

    // Progress area height (show when active)
    let progress_height: u16 = if app.progress_manager.is_active() {
        let tool_count = app.progress_manager.active_tool_count();
        // 1 line for main spinner + 1 line per tool
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
            Constraint::Min(3),                 // Messages area
            Constraint::Length(progress_height), // Progress area (dynamic)
            Constraint::Length(input_height),   // Input area
            Constraint::Length(1),              // Status bar
        ])
        .split(area);

    // Render messages area
    render_messages(f, app, chunks[0]);

    // Render progress area (if active)
    if progress_height > 0 && chunks[1].height > 0 {
        render_progress(f, app, chunks[1]);
    }

    // Render input area
    render_input(f, app, chunks[2]);

    // Render completion popup (above input area)
    if completion_height > 0 {
        render_completion(f, app, chunks[2]);
    }

    // Render status bar
    render_status(f, app, chunks[3]);

    // Render permission dialog if pending
    if let Some(permission) = &app.pending_permission {
        if let Some(dialog) = &app.permission_dialog {
            dialog.render(f);
        } else {
            // Render inline permission request if dialog not initialized
            let result = PermissionResult::ask(&permission.message);
            let temp_dialog = PermissionDialog::new(permission.message.clone(), result);
            temp_dialog.render(f);
        }
    }
}

fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages: Vec<Line> = app
        .messages
        .iter()
        .flat_map(|msg| render_message_lines(msg))
        .collect();

    // Add streaming text if currently streaming
    let all_lines: Vec<Line> = if app.is_streaming && !app.streaming_text.is_empty() {
        let streaming_lines: Vec<Line> = app.streaming_text.lines()
            .map(|line| Line::from(Span::styled(line.to_string(), Style::default().fg(Color::Green))))
            .collect();
        messages.into_iter().chain(streaming_lines).collect()
    } else {
        messages
    };

    // Auto-scroll to follow latest output when streaming or loading
    let total_lines = all_lines.len();
    let visible_lines = area.height as usize;

    // Calculate max scroll position (don't scroll past content)
    let max_scroll = if total_lines > visible_lines {
        total_lines - visible_lines
    } else {
        0
    };

    let scroll_offset = if app.auto_scroll || app.is_streaming || app.is_loading {
        // Follow latest: scroll to bottom
        max_scroll
    } else {
        // User scrolling: use manual scroll position, but clamp to max
        (app.scroll as usize).min(max_scroll)
    };

    // No border for messages area
    let paragraph = Paragraph::new(all_lines)
        .scroll((scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Render progress indicators
fn render_progress(f: &mut Frame, app: &App, area: Rect) {
    let progress_lines = app.progress_manager.render(area.width as usize);

    let block = Block::default()
        .borders(Borders::NONE);

    let paragraph = Paragraph::new(progress_lines).block(block);

    f.render_widget(paragraph, area);
}

/// Render a message as lines
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

                // Split text into lines for display (no prefix)
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

                // Show truncated content
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

        // Cursor position within input area
        let cursor_x = area.x + 1 + cursor_display_x as u16;
        let cursor_y = area.y + 1 + cursor_row as u16;

        // Make sure cursor is within bounds
        if cursor_y < area.y + area.height - 1 {
            f.set_cursor(cursor_x, cursor_y);
        }
    }
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if app.is_loading {
        Line::from(Span::styled(
            " ⏳ Processing with tools... ",
            Style::default().fg(Color::Yellow),
        ))
    } else if app.ctrl_c_count > 0 {
        Line::from(Span::styled(
            " ⚠️ Press Ctrl+C again to quit ",
            Style::default().fg(Color::Red),
        ))
    } else {
        Line::from(Span::styled(
            format!(" {}", app.status),
            Style::default().fg(Color::DarkGray),
        ))
    };

    let status_widget = Paragraph::new(status_text);
    f.render_widget(status_widget, area);
}

/// Render completion popup above input area
fn render_completion(f: &mut Frame, app: &App, input_area: Rect) {
    if !app.completion_state.is_visible() {
        return;
    }

    let suggestions = app.completion_state.suggestions();
    let selected_idx = app.completion_state.selected_index();

    // Calculate popup height
    let popup_height = (suggestions.len() + 2).min(10) as u16;
    let popup_width = 40_u16; // Fixed width for command completions

    // Position popup above input area
    let popup_y = input_area.y.saturating_sub(popup_height);
    let popup_area = Rect {
        x: input_area.x,
        y: popup_y,
        width: popup_width.min(input_area.width),
        height: popup_height,
    };

    // Clear the area first
    f.render_widget(Clear, popup_area);

    // Build completion lines
    let lines: Vec<Line> = suggestions
        .iter()
        .enumerate()
        .map(|(i, suggestion)| {
            let is_selected = i == selected_idx;

            // Color based on completion type
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

            // Add description if available
            if let Some(desc) = &suggestion.description {
                let desc_text = Span::styled(
                    format!(" - {}", desc),
                    if is_selected {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
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