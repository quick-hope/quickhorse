//! DatabaseTool - SQLite database operations
//!
//! Provides:
//! - SQLite database queries
//! - SQL execution
//! - Output formatting (table/JSON)
//! - Database info queries

use crate::tools::{Tool, ToolContext, ToolResult, build_schema};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

/// DatabaseTool - SQLite database operations
pub struct DatabaseTool;

impl DatabaseTool {
    pub fn new() -> Self {
        Self
    }
}

/// Output format type
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Table format with headers
    Table,
    /// JSON format
    Json,
}

/// Database input parameters
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DatabaseInput {
    /// Path to SQLite database file
    pub database_path: String,
    /// SQL query to execute
    pub query: String,
    /// Output format (default: table)
    #[serde(default = "default_format")]
    pub format: OutputFormat,
}

fn default_format() -> OutputFormat {
    OutputFormat::Table
}

/// Database output result
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseOutput {
    /// Database path
    pub database_path: String,
    /// Query executed
    pub query: String,
    /// Number of rows returned/affected
    pub rows: u64,
    /// Column names
    pub columns: Vec<String>,
    /// Query results as strings
    pub data: Vec<Vec<String>>,
    /// Formatted output string
    pub formatted: String,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Format query results as table
fn format_table(columns: &[String], data: &[Vec<String>]) -> String {
    if columns.is_empty() && data.is_empty() {
        return "Empty result".to_string();
    }

    // Calculate column widths
    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
    for row in data {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Build header
    let header: String = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let width = widths.get(i).copied().unwrap_or(col.len());
            format!(" {:width$} ", col, width = width)
        })
        .collect::<Vec<_>>()
        .join("|");

    // Build separator
    let separator: String = widths
        .iter()
        .map(|w| format!("-{:-<width$}", "", width = w))
        .collect::<Vec<_>>()
        .join("+");

    // Build rows
    let rows: String = data
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, cell)| {
                    let width = widths.get(i).copied().unwrap_or(cell.len());
                    format!(" {:width$} ", cell, width = width)
                })
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect::<Vec<_>>()
        .join("\n");

    if data.is_empty() {
        format!("{}\n{}\n(0 rows)", header, separator)
    } else {
        format!("{}\n{}\n{}\n({} rows)", header, separator, rows, data.len())
    }
}

/// Format query results as JSON
fn format_json(columns: &[String], data: &[Vec<String>]) -> String {
    let rows: Vec<serde_json::Value> = data
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                let val = row.get(i).cloned().unwrap_or_default();
                obj.insert(col.clone(), serde_json::Value::String(val));
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

#[async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str {
        "Database"
    }

    fn description(&self) -> String {
        "Execute SQL queries on SQLite databases. Get table info, run queries, and format results as table or JSON."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<DatabaseInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        // Parse input
        let params: DatabaseInput = serde_json::from_value(input)?;

        let db_path = Path::new(&params.database_path);

        // Check if read-only operation
        let is_read = params.query.trim().to_lowercase().starts_with("select")
            || params.query.trim().to_lowercase().starts_with("pragma");

        if is_read && !db_path.exists() {
            return Ok(ToolResult::error(format!(
                "Database file not found: {}",
                params.database_path
            )));
        }

        // Execute query (using sqlite3 CLI)
        let start_time = std::time::Instant::now();

        let output = execute_sqlite_query(&params.database_path, &params.query, &params.format);

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match output {
            Ok((columns, data, rows)) => {
                let formatted = match params.format {
                    OutputFormat::Table => format_table(&columns, &data),
                    OutputFormat::Json => format_json(&columns, &data),
                };

                let result = DatabaseOutput {
                    database_path: params.database_path.clone(),
                    query: params.query.clone(),
                    rows,
                    columns,
                    data,
                    formatted,
                    execution_time_ms,
                };

                Ok(ToolResult::success(serde_json::to_string(&result).unwrap_or_else(|_| {
                    format!("Query returned {} rows in {}ms", rows, execution_time_ms)
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("Database error: {}", e))),
        }
    }

    fn is_read_only(&self, input: &serde_json::Value) -> bool {
        if let Ok(params) = serde_json::from_value::<DatabaseInput>(input.clone()) {
            let query_lower = params.query.to_lowercase();
            query_lower.starts_with("select")
                || query_lower.starts_with("pragma")
                || query_lower.starts_with("describe")
        } else {
            true // Default to read-only if can't parse
        }
    }
}

/// Execute SQLite query using sqlite3 CLI
fn execute_sqlite_query(
    db_path: &str,
    query: &str,
    format: &OutputFormat,
) -> Result<(Vec<String>, Vec<Vec<String>>, u64), String> {
    use std::process::Command;

    // Check if sqlite3 is available
    let check = Command::new("sqlite3").arg("--version").output();
    if check.is_err() {
        return Err("sqlite3 command not found. Install SQLite to use DatabaseTool.".to_string());
    }

    // Build command
    let mut cmd = Command::new("sqlite3");
    cmd.arg(db_path);

    // Set output format
    match format {
        OutputFormat::Json => {
            cmd.arg("-json");
        }
        OutputFormat::Table => {
            cmd.arg("-header");
            cmd.arg("-column");
        }
    }

    cmd.arg(query);

    let output = cmd.output().map_err(|e| format!("Failed to execute sqlite3: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse output
    match format {
        OutputFormat::Json => {
            // Parse JSON output
            let rows_value: serde_json::Value = if stdout.is_empty() {
                serde_json::Value::Array(vec![])
            } else {
                serde_json::from_str(&stdout)
                    .map_err(|e| format!("Failed to parse JSON: {}", e))?
            };

            // Check if empty array
            let rows_arr = rows_value.as_array().cloned().unwrap_or_default();
            if rows_arr.is_empty() {
                return Ok((vec![], vec![], 0));
            }

            // Get columns from first row
            let columns: Vec<String> = if let Some(first) = rows_arr.first() {
                if let Some(obj) = first.as_object() {
                    obj.keys().cloned().collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            // Extract data
            let data: Vec<Vec<String>> = rows_arr
                .iter()
                .map(|row| {
                    if let Some(obj) = row.as_object() {
                        columns
                            .iter()
                            .map(|col| {
                                obj.get(col)
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| {
                                        obj.get(col).map(|v| v.to_string()).unwrap_or_default()
                                    })
                            })
                            .collect()
                    } else {
                        vec![]
                    }
                })
                .collect();

            let rows_count = data.len() as u64;
            Ok((columns, data, rows_count))
        }
        OutputFormat::Table => {
            // Parse column output
            let lines: Vec<&str> = stdout.lines().collect();

            if lines.is_empty() {
                return Ok((vec![], vec![], 0));
            }

            // First line is header
            let columns: Vec<String> = lines[0]
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

            // Skip separator line (if present)
            let data_start = if lines.len() > 1 && lines[1].contains('-') {
                2
            } else {
                1
            };

            let data: Vec<Vec<String>> = lines[data_start..]
                .iter()
                .map(|line| {
                    line.split('|')
                        .map(|s| s.trim().to_string())
                        .collect()
                })
                .collect();

            let rows_count = data.len() as u64;
            Ok((columns, data, rows_count))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_tool_creation() {
        let tool = DatabaseTool::new();
        assert_eq!(tool.name(), "Database");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_database_input_schema() {
        let tool = DatabaseTool::new();
        let schema = tool.input_schema();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("properties"));
    }

    #[test]
    fn test_default_format() {
        assert!(matches!(default_format(), OutputFormat::Table));
    }

    #[test]
    fn test_format_table_empty() {
        let result = format_table(&[], &[]);
        assert_eq!(result, "Empty result");
    }

    #[test]
    fn test_format_table_single_row() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let data = vec![vec!["1".to_string(), "Alice".to_string()]];
        let result = format_table(&columns, &data);
        assert!(result.contains("id"));
        assert!(result.contains("name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("1"));
        assert!(result.contains("1 rows"));
    }

    #[test]
    fn test_format_table_multiple_rows() {
        let columns = vec!["key".to_string(), "value".to_string()];
        let data = vec![
            vec!["1".to_string(), "a".to_string()],
            vec!["2".to_string(), "b".to_string()],
        ];
        let result = format_table(&columns, &data);
        assert!(result.contains("2 rows"));
    }

    #[test]
    fn test_format_json_empty() {
        let result = format_json(&[], &[]);
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_format_json_single_row() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let data = vec![vec!["1".to_string(), "Alice".to_string()]];
        let result = format_json(&columns, &data);
        assert!(result.contains("id"));
        assert!(result.contains("name"));
        assert!(result.contains("Alice"));
    }

    #[test]
    fn test_database_input_deserialization() {
        let json = r#"{"database_path": "/path/to/db.sqlite", "query": "SELECT * FROM users"}"#;
        let input: DatabaseInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.database_path, "/path/to/db.sqlite");
        assert_eq!(input.query, "SELECT * FROM users");
        assert!(matches!(input.format, OutputFormat::Table));
    }

    #[test]
    fn test_database_input_with_format() {
        let json = r#"{"database_path": "/path/to/db.sqlite", "query": "SELECT 1", "format": "json"}"#;
        let input: DatabaseInput = serde_json::from_str(json).unwrap();
        assert!(matches!(input.format, OutputFormat::Json));
    }

    #[test]
    fn test_is_read_only_select() {
        let input = serde_json::json!({
            "database_path": "test.db",
            "query": "SELECT * FROM users"
        });
        let tool = DatabaseTool::new();
        assert!(tool.is_read_only(&input));
    }

    #[test]
    fn test_is_read_only_insert() {
        let input = serde_json::json!({
            "database_path": "test.db",
            "query": "INSERT INTO users VALUES (1, 'test')"
        });
        let tool = DatabaseTool::new();
        assert!(!tool.is_read_only(&input));
    }

    #[test]
    fn test_database_output_serialization() {
        let output = DatabaseOutput {
            database_path: "/db.sqlite".to_string(),
            query: "SELECT 1".to_string(),
            rows: 1,
            columns: vec!["col1".to_string()],
            data: vec![vec!["1".to_string()]],
            formatted: "table".to_string(),
            execution_time_ms: 10,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("database_path"));
        assert!(json.contains("query"));
        assert!(json.contains("rows"));
    }
}