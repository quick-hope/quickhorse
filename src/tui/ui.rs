//! UI rendering for TUI

use crate::provider::{ContentBlock, Message};
use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    // Render status bar
    render_status(f, app, chunks[3]);
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
            .map(|line| Line::from(vec![
                Span::styled("Assistant: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(line.to_string(), Style::default().fg(Color::Green)),
            ]))
            .collect();
        messages.into_iter().chain(streaming_lines).collect()
    } else {
        messages
    };

    let block = Block::default()
        .title(" Messages ")
        .title_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(all_lines)
        .block(block)
        .scroll((app.scroll, 0));

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
                let (prefix, style) = match msg.role.as_str() {
                    "user" => ("You: ", Style::default().fg(Color::Cyan)),
                    "assistant" => ("Assistant: ", Style::default().fg(Color::Green)),
                    "system" => ("System: ", Style::default().fg(Color::Yellow)),
                    _ => ("", Style::default()),
                };

                // Split text into lines for display
                for line_text in text.lines() {
                    let line = Line::from(vec![
                        Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                        Span::styled(line_text.to_string(), style),
                    ]);
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