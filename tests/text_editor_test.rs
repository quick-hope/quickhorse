//! Unit tests for TextEditor (multiline input handling)

use quickhorse::tui::TextEditor;

#[test]
fn test_text_editor_new() {
    let editor = TextEditor::new();
    assert!(editor.is_empty());
    assert_eq!(editor.lines().len(), 1);
    assert_eq!(editor.text(), "");
}

#[test]
fn test_text_editor_insert_char() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_char('c');
    assert_eq!(editor.text(), "abc");
    assert_eq!(editor.lines().len(), 1);
}

#[test]
fn test_text_editor_insert_newline() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_newline();
    editor.insert_char('c');
    editor.insert_char('d');
    assert_eq!(editor.text(), "ab\ncd");
    assert_eq!(editor.lines().len(), 2);
}

#[test]
fn test_text_editor_backspace() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_char('c');
    editor.backspace();
    assert_eq!(editor.text(), "ab");
    editor.backspace();
    editor.backspace();
    assert_eq!(editor.text(), "");
}

#[test]
fn test_text_editor_backspace_merge_lines() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_newline();
    editor.insert_char('b');
    assert_eq!(editor.text(), "a\nb");
    // Move cursor to beginning of second line
    editor.move_home();
    editor.backspace(); // Should merge lines
    assert_eq!(editor.text(), "ab");
    assert_eq!(editor.lines().len(), 1);
}

#[test]
fn test_text_editor_delete() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_char('c');
    editor.move_left(); // Move cursor before 'c'
    editor.delete();
    assert_eq!(editor.text(), "ab");
}

#[test]
fn test_text_editor_cursor_movement() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_char('c');

    let (row, col) = editor.cursor_position();
    assert_eq!(row, 0);
    assert_eq!(col, 3); // After 'abc'

    editor.move_left();
    let (_, col) = editor.cursor_position();
    assert_eq!(col, 2); // Before 'c'

    editor.move_left();
    editor.move_left();
    let (_, col) = editor.cursor_position();
    assert_eq!(col, 0); // At start
}

#[test]
fn test_text_editor_cursor_vertical_movement() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_newline();
    editor.insert_char('c');
    editor.insert_char('d');

    let (row, _) = editor.cursor_position();
    assert_eq!(row, 1); // On second line

    editor.move_up();
    let (row, _) = editor.cursor_position();
    assert_eq!(row, 0); // On first line

    editor.move_down();
    let (row, _) = editor.cursor_position();
    assert_eq!(row, 1); // Back on second line
}

#[test]
fn test_text_editor_home_end() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_char('c');

    editor.move_home();
    let (_, col) = editor.cursor_position();
    assert_eq!(col, 0);

    editor.move_end();
    let (_, col) = editor.cursor_position();
    assert_eq!(col, 3);
}

#[test]
fn test_text_editor_clear() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.insert_char('c');
    editor.clear();
    assert!(editor.is_empty());
    assert_eq!(editor.text(), "");
}

#[test]
fn test_text_editor_unicode_handling() {
    let mut editor = TextEditor::new();
    // Chinese characters
    editor.insert_char('你');
    editor.insert_char('好');
    editor.insert_char('世');
    editor.insert_char('界');
    assert_eq!(editor.text(), "你好世界");

    // Test cursor movement with UTF-8
    editor.move_left();
    let (_, col) = editor.cursor_position();
    assert_eq!(col, 9); // 3 Chinese chars * 3 bytes each = 9 bytes

    editor.move_left();
    let (_, col) = editor.cursor_position();
    assert_eq!(col, 6); // 2 Chinese chars * 3 bytes = 6 bytes
}

#[test]
fn test_text_editor_display_width_cjk() {
    let mut editor = TextEditor::new();
    editor.insert_char('你');
    editor.insert_char('好');

    // Each Chinese char has display width of 2
    let display_x = editor.cursor_display_x();
    assert_eq!(display_x, 4); // 2 chars * 2 width each
}

#[test]
fn test_text_editor_mixed_ascii_unicode() {
    let mut editor = TextEditor::new();
    editor.insert_char('a');
    editor.insert_char('你'); // UTF-8 3 bytes, display width 2
    editor.insert_char('b');

    assert_eq!(editor.text(), "a你b");

    let display_x = editor.cursor_display_x();
    assert_eq!(display_x, 4); // a(1) + 你(2) + b(1) = 4
}