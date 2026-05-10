//! UI rendering for TUI

use crate::provider::{ContentBlock, Message};
use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render the TUI
pub fn render(f: &mut Frame, app: &App) {
    let area = f.size();

    // Create main layout: messages area, input area, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(3),    // Messages area
            Constraint::Length(3), // Input area
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    // Render messages area
    render_messages(f, app, chunks[0]);

    // Render input area
    render_input(f, app, chunks[1]);

    // Render status bar
    render_status(f, app, chunks[2]);
}

fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .flat_map(|msg| render_message(msg))
        .collect();

    let messages_widget = List::new(messages)
        .block(
            Block::default()
                .title(" Messages ")
                .title_style(Style::default().add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

    f.render_widget(messages_widget, area);
}

/// Render a single message as multiple list items
fn render_message(msg: &Message) -> Vec<ListItem<'_>> {
    let items: Vec<ListItem> = msg
        .content
        .iter()
        .map(|block| {
            match block {
                ContentBlock::Text { text } => {
                    let (prefix, style) = match msg.role.as_str() {
                        "user" => ("You: ", Style::default().fg(Color::Cyan)),
                        "assistant" => ("Assistant: ", Style::default().fg(Color::Green)),
                        "system" => ("System: ", Style::default().fg(Color::Yellow)),
                        _ => ("", Style::default()),
                    };
                    let content = format!("{}{}", prefix, text);
                    ListItem::new(Text::from(content)).style(style)
                }
                ContentBlock::ToolUse { id, name, input } => {
                    let input_str = serde_json::to_string_pretty(&input)
                        .unwrap_or_else(|_| input.to_string());
                    let content = format!("🔧 Tool Call: {} ({})\n{}", name, id, input_str);
                    ListItem::new(Text::from(content))
                        .style(Style::default().fg(Color::Magenta))
                }
                ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                    let prefix = if is_error.unwrap_or(false) { "❌" } else { "✅" };
                    let truncated = if content.len() > 500 {
                        format!("{} Tool Result ({})\n{}...", prefix, tool_use_id, &content[..500])
                    } else {
                        format!("{} Tool Result ({})\n{}", prefix, tool_use_id, content)
                    };
                    let style = if is_error.unwrap_or(false) {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::LightGreen)
                    };
                    ListItem::new(Text::from(truncated)).style(style)
                }
            }
        })
        .collect();
    items
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.is_loading {
        " Waiting for response... "
    } else {
        " Input (Enter to send, Ctrl+C twice to quit) "
    };

    let style = if app.is_loading {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::White)
    };

    let input_widget = Paragraph::new(app.input.as_str())
        .style(style)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(if app.is_loading {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::Green)
                }),
        );

    f.render_widget(input_widget, area);

    // Always show cursor when not loading
    if !app.is_loading {
        let cursor_x = area.x + 1 + app.input.len() as u16;
        let cursor_y = area.y + 1;
        f.set_cursor(cursor_x, cursor_y);
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