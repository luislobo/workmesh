use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use csv::{ReaderBuilder, StringRecord};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
}

impl RenderError {
    fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TableMode {
    Minimal,
    Box,
}

impl Default for TableMode {
    fn default() -> Self {
        Self::Minimal
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub key: String,
    pub header: Option<String>,
    pub align: Option<Alignment>,
    pub width: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableStyle {
    pub head: Option<Vec<String>>,
    pub border: Option<Vec<String>>,
    pub compact: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableConfiguration {
    pub mode: Option<TableMode>,
    pub columns: Option<Vec<TableColumn>>,
    pub show_index: Option<bool>,
    pub null_display: Option<String>,
    pub truncate_to: Option<usize>,
    pub max_width: Option<usize>,
    pub fit_to_terminal: Option<bool>,
    pub wrap: Option<bool>,
    pub style: Option<TableStyle>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KvConfiguration {
    pub title: Option<String>,
    pub key_order: Option<Vec<String>>,
    pub max_width: Option<usize>,
    pub wrap: Option<bool>,
    pub key_min_width: Option<usize>,
    pub separator: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatsConfiguration {
    pub compact: Option<bool>,
    pub max_width: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProgressConfiguration {
    pub bar_width: Option<usize>,
    pub show_percent: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TreeConfiguration {
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiffConfiguration {
    pub context: Option<usize>,
    pub show_header: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogsConfiguration {
    pub columns: Option<Vec<String>>,
    pub uppercase_level: Option<bool>,
    pub max_width: Option<usize>,
    pub mode: Option<TableMode>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlertsConfiguration {
    pub include_level: Option<bool>,
    pub include_timestamp: Option<bool>,
    pub compact: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListConfiguration {
    pub ordered: Option<bool>,
    pub checkbox: Option<bool>,
    pub start: Option<i64>,
    pub bullet: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChartBarConfiguration {
    pub width: Option<usize>,
    pub bar_char: Option<String>,
    pub show_values: Option<bool>,
    pub max_label_width: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SparklineConfiguration {
    pub label: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimelineConfiguration {
    pub sort: Option<bool>,
    pub show_status: Option<bool>,
    pub max_width: Option<usize>,
    pub wrap: Option<bool>,
}

#[derive(Debug, Clone, Default)]
struct ResolvedColumn {
    key: String,
    header: String,
    align: Alignment,
    width: Option<usize>,
}

#[derive(Debug, Clone)]
struct TreeNode {
    label: String,
    children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
enum DiffOp {
    Equal(String),
    Remove(String),
    Add(String),
    Skip(usize),
}

pub fn dispatch_tool(tool: &str, arguments: &Value) -> Result<Value, RenderError> {
    let rendered = match tool {
        "render_table" => {
            let data = required_data(arguments)?;
            let format = optional_string(arguments, "format")?;
            let configuration: Option<TableConfiguration> = optional_config(arguments)?;
            render_table(data, format.as_deref(), configuration)
        }
        "render_kv" => {
            let data = required_data(arguments)?;
            let configuration: Option<KvConfiguration> = optional_config(arguments)?;
            render_kv(data, configuration)
        }
        "render_stats" => {
            let data = required_data(arguments)?;
            let configuration: Option<StatsConfiguration> = optional_config(arguments)?;
            render_stats(data, configuration)
        }
        "render_progress" => {
            let data = required_data(arguments)?;
            let configuration: Option<ProgressConfiguration> = optional_config(arguments)?;
            render_progress(data, configuration)
        }
        "render_tree" => {
            let data = required_data(arguments)?;
            let configuration: Option<TreeConfiguration> = optional_config(arguments)?;
            render_tree(data, configuration)
        }
        "render_diff" => {
            let data = required_data(arguments)?;
            let configuration: Option<DiffConfiguration> = optional_config(arguments)?;
            render_diff(data, configuration)
        }
        "render_logs" => {
            let data = required_data(arguments)?;
            let configuration: Option<LogsConfiguration> = optional_config(arguments)?;
            render_logs(data, configuration)
        }
        "render_alerts" => {
            let data = required_data(arguments)?;
            let configuration: Option<AlertsConfiguration> = optional_config(arguments)?;
            render_alerts(data, configuration)
        }
        "render_list" => {
            let data = required_data(arguments)?;
            let configuration: Option<ListConfiguration> = optional_config(arguments)?;
            render_list(data, configuration)
        }
        "render_chart_bar" => {
            let data = required_data(arguments)?;
            let configuration: Option<ChartBarConfiguration> = optional_config(arguments)?;
            render_chart_bar(data, configuration)
        }
        "render_sparkline" => {
            let data = required_data(arguments)?;
            let configuration: Option<SparklineConfiguration> = optional_config(arguments)?;
            render_sparkline(data, configuration)
        }
        "render_timeline" => {
            let data = required_data(arguments)?;
            let configuration: Option<TimelineConfiguration> = optional_config(arguments)?;
            render_timeline(data, configuration)
        }
        _ => Err(RenderError::not_found(format!("Tool not found: {}", tool))),
    }?;

    Ok(json!({ "text": rendered }))
}

fn required_data(arguments: &Value) -> Result<Value, RenderError> {
    let object = as_args_object(arguments)?;
    object
        .get("data")
        .cloned()
        .ok_or_else(|| RenderError::invalid("data is required"))
}

fn optional_string(arguments: &Value, key: &str) -> Result<Option<String>, RenderError> {
    let object = as_args_object(arguments)?;
    match object.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(RenderError::invalid(format!("{} must be a string", key))),
    }
}

fn optional_config<T: DeserializeOwned>(arguments: &Value) -> Result<Option<T>, RenderError> {
    let object = as_args_object(arguments)?;
    let Some(raw) = object.get("configuration") else {
        return Ok(None);
    };
    let parsed = serde_json::from_value(raw.clone())
        .map_err(|err| RenderError::invalid(format!("invalid configuration: {}", err)))?;
    Ok(Some(parsed))
}

fn as_args_object(arguments: &Value) -> Result<&Map<String, Value>, RenderError> {
    arguments
        .as_object()
        .ok_or_else(|| RenderError::invalid("arguments must be an object"))
}

fn parse_maybe_json(value: Value) -> Value {
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Value::String(raw)
            } else {
                serde_json::from_str(trimmed).unwrap_or(Value::String(raw))
            }
        }
        other => other,
    }
}

fn to_array(value: &Value, error: &str) -> Result<Vec<Value>, RenderError> {
    value
        .as_array()
        .cloned()
        .ok_or_else(|| RenderError::invalid(error))
}

fn to_object(value: &Value, error: &str) -> Result<Map<String, Value>, RenderError> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| RenderError::invalid(error))
}

fn as_string(value: &Value, fallback: &str) -> String {
    match value {
        Value::Null => fallback.to_string(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| fallback.to_string()),
    }
}

fn to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    }
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn take_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn truncate(value: &str, max_length: Option<usize>) -> String {
    let Some(max_length) = max_length else {
        return value.to_string();
    };
    if char_len(value) <= max_length {
        return value.to_string();
    }
    if max_length <= 1 {
        return take_chars(value, max_length);
    }
    format!("{}…", take_chars(value, max_length - 1))
}

fn clip_to_width(value: &str, width: usize) -> String {
    if char_len(value) <= width {
        return value.to_string();
    }
    if width <= 1 {
        return take_chars(value, width);
    }
    format!("{}…", take_chars(value, width - 1))
}

fn resolve_viewport_width(configuration: &TableConfiguration) -> Option<usize> {
    let max_width = configuration.max_width;
    let terminal_width = if configuration.fit_to_terminal.unwrap_or(false) {
        std::env::var("COLUMNS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
    } else {
        None
    };

    match (max_width, terminal_width) {
        (Some(max_width), Some(terminal_width)) => Some(min(max_width, terminal_width)),
        (Some(max_width), None) => Some(max_width),
        (None, Some(terminal_width)) => Some(terminal_width),
        (None, None) => None,
    }
}

fn normalize_cell(value: &Value, null_display: &str) -> String {
    if value.is_null() {
        return null_display.to_string();
    }
    as_string(value, null_display)
}

fn pad_cell(value: &str, width: usize, align: Alignment) -> String {
    let len = char_len(value);
    if len >= width {
        return value.to_string();
    }
    let pad = width - len;

    match align {
        Alignment::Right => format!("{}{}", " ".repeat(pad), value),
        Alignment::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), value, " ".repeat(right))
        }
        Alignment::Left => format!("{}{}", value, " ".repeat(pad)),
    }
}

fn wrap_text(value: &str, width: usize, wrap: bool) -> Vec<String> {
    let safe_width = max(1, width);
    let mut wrapped: Vec<String> = Vec::new();

    for line in value.split('\n') {
        if !wrap {
            wrapped.push(clip_to_width(line, safe_width));
            continue;
        }

        if line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut remaining = line.to_string();
        while char_len(&remaining) > safe_width {
            let chunk = take_chars(&remaining, safe_width);
            let last_space = chunk.rfind(' ');

            if let Some(last_space) = last_space {
                if last_space > 0 {
                    let split = take_chars(&chunk, last_space);
                    wrapped.push(split);
                    remaining = remaining
                        .chars()
                        .skip(last_space + 1)
                        .collect::<String>()
                        .trim_start()
                        .to_string();
                    continue;
                }
            }

            wrapped.push(chunk.clone());
            remaining = remaining.chars().skip(safe_width).collect::<String>();
        }

        wrapped.push(remaining);
    }

    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn render_logical_row_cells(
    values: &[String],
    widths: &[usize],
    aligns: &[Alignment],
    wrap: bool,
) -> Vec<Vec<String>> {
    let mut cell_lines: Vec<Vec<String>> = Vec::new();
    for idx in 0..values.len() {
        cell_lines.push(wrap_text(
            values.get(idx).map(String::as_str).unwrap_or(""),
            *widths.get(idx).unwrap_or(&1),
            wrap,
        ));
    }

    let line_count = cell_lines.iter().map(Vec::len).max().unwrap_or(1);
    let mut out: Vec<Vec<String>> = Vec::new();

    for line_idx in 0..line_count {
        let mut row: Vec<String> = Vec::new();
        for col_idx in 0..values.len() {
            let cell = cell_lines
                .get(col_idx)
                .and_then(|lines| lines.get(line_idx))
                .cloned()
                .unwrap_or_default();
            row.push(pad_cell(
                &cell,
                *widths.get(col_idx).unwrap_or(&1),
                *aligns.get(col_idx).unwrap_or(&Alignment::Left),
            ));
        }
        out.push(row);
    }

    out
}

fn render_minimal_table(
    head: &[String],
    aligns: &[Alignment],
    widths: &[usize],
    rows: &[Vec<String>],
    wrap: bool,
) -> String {
    let mut out: Vec<String> = Vec::new();

    for row in render_logical_row_cells(head, widths, aligns, wrap) {
        out.push(row.join(" | "));
    }
    out.push(
        widths
            .iter()
            .map(|width| "-".repeat(max(1, *width)))
            .collect::<Vec<_>>()
            .join("-|-"),
    );

    for values in rows {
        for row in render_logical_row_cells(values, widths, aligns, wrap) {
            out.push(row.join(" | "));
        }
    }

    out.join("\n")
}

fn render_box_table(
    head: &[String],
    aligns: &[Alignment],
    widths: &[usize],
    rows: &[Vec<String>],
    wrap: bool,
) -> String {
    let mut out: Vec<String> = Vec::new();
    let border = format!(
        "+{}+",
        widths
            .iter()
            .map(|width| "-".repeat(width + 2))
            .collect::<Vec<_>>()
            .join("+")
    );

    out.push(border.clone());
    for row in render_logical_row_cells(head, widths, aligns, wrap) {
        out.push(format!("| {} |", row.join(" | ")));
    }
    out.push(border.clone());

    for values in rows {
        for row in render_logical_row_cells(values, widths, aligns, wrap) {
            out.push(format!("| {} |", row.join(" | ")));
        }
    }

    out.push(border);
    out.join("\n")
}

fn compute_base_widths(
    head: &[String],
    rows: &[Vec<String>],
    explicit_widths: &[Option<usize>],
) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();

    for col_idx in 0..head.len() {
        if let Some(width) = explicit_widths.get(col_idx).and_then(|value| *value) {
            out.push(max(1, width));
            continue;
        }

        let mut col_width = char_len(head.get(col_idx).map(String::as_str).unwrap_or(""));
        for row in rows {
            if let Some(cell) = row.get(col_idx) {
                col_width = max(col_width, char_len(cell));
            }
        }
        out.push(max(1, col_width));
    }

    out
}

fn shrink_widths_to_fit(
    widths: &[usize],
    min_widths: &[usize],
    max_width: usize,
    separator_overhead: usize,
) -> Vec<usize> {
    let mut out = widths.to_vec();
    let target_content_width = max(1, max_width.saturating_sub(separator_overhead));
    let mut current_content_width: usize = out.iter().sum();

    while current_content_width > target_content_width {
        let mut widest_idx: Option<usize> = None;
        let mut widest = 0usize;

        for (idx, width) in out.iter().enumerate() {
            let min_width = *min_widths.get(idx).unwrap_or(&1);
            if *width > min_width && *width > widest {
                widest = *width;
                widest_idx = Some(idx);
            }
        }

        let Some(widest_idx) = widest_idx else {
            break;
        };

        out[widest_idx] = out[widest_idx].saturating_sub(1);
        current_content_width = current_content_width.saturating_sub(1);
    }

    out
}

fn parse_csv_rows(data: Value) -> Result<Vec<Map<String, Value>>, RenderError> {
    let raw = match data {
        Value::String(raw) => raw,
        _ => return Err(RenderError::invalid("CSV format expects data as a string")),
    };

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(raw.as_bytes());

    let headers = reader
        .headers()
        .map_err(|err| RenderError::invalid(format!("invalid CSV headers: {}", err)))?
        .clone();

    let mut rows: Vec<Map<String, Value>> = Vec::new();
    for record in reader.records() {
        let record: StringRecord =
            record.map_err(|err| RenderError::invalid(format!("invalid CSV row: {}", err)))?;
        rows.push(csv_record_to_map(&headers, &record));
    }

    Ok(rows)
}

fn csv_record_to_map(headers: &StringRecord, record: &StringRecord) -> Map<String, Value> {
    let mut row: Map<String, Value> = Map::new();
    for idx in 0..headers.len() {
        let key = headers.get(idx).unwrap_or_default().to_string();
        let value = record.get(idx).unwrap_or_default().to_string();
        row.insert(key, Value::String(value));
    }
    row
}

fn parse_json_rows(data: Value) -> Result<Vec<Map<String, Value>>, RenderError> {
    let parsed = parse_maybe_json(data);
    let rows = parsed
        .as_array()
        .ok_or_else(|| RenderError::invalid("JSON format expects an array of objects"))?;

    let mut out: Vec<Map<String, Value>> = Vec::new();
    for row in rows {
        let object = row.as_object().ok_or_else(|| {
            RenderError::invalid("Every row must be an object when using JSON format")
        })?;
        out.push(object.clone());
    }
    Ok(out)
}

fn resolve_columns(
    rows: &[Map<String, Value>],
    configuration: &TableConfiguration,
) -> Vec<ResolvedColumn> {
    if let Some(columns) = &configuration.columns {
        return columns
            .iter()
            .map(|column| ResolvedColumn {
                key: column.key.clone(),
                header: column.header.clone().unwrap_or_else(|| column.key.clone()),
                align: column.align.unwrap_or(Alignment::Left),
                width: column.width,
            })
            .collect();
    }

    let mut keys: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for row in rows {
        for key in row.keys() {
            if seen.insert(key.clone()) {
                keys.push(key.clone());
            }
        }
    }

    keys.into_iter()
        .map(|key| ResolvedColumn {
            header: key.clone(),
            key,
            align: Alignment::Left,
            width: None,
        })
        .collect()
}

pub fn render_table(
    data: Value,
    format: Option<&str>,
    configuration: Option<TableConfiguration>,
) -> Result<String, RenderError> {
    let format = format.unwrap_or("json");
    let configuration = configuration.unwrap_or_default();

    let rows = if format == "csv" {
        parse_csv_rows(data)?
    } else {
        parse_json_rows(data)?
    };

    if rows.is_empty() {
        return Ok("(no rows)".to_string());
    }

    let columns = resolve_columns(&rows, &configuration);
    let null_display = configuration
        .null_display
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let mode = configuration.mode.unwrap_or(TableMode::Minimal);
    let wrap = configuration.wrap.unwrap_or(true);
    let max_width = resolve_viewport_width(&configuration);
    let show_index = configuration.show_index.unwrap_or(false);

    let mut head: Vec<String> = Vec::new();
    let mut aligns: Vec<Alignment> = Vec::new();
    let mut explicit_widths: Vec<Option<usize>> = Vec::new();

    if show_index {
        head.push("#".to_string());
        aligns.push(Alignment::Right);
        explicit_widths.push(None);
    }

    for column in &columns {
        head.push(column.header.clone());
        aligns.push(column.align);
        explicit_widths.push(column.width);
    }

    let mut rendered_rows: Vec<Vec<String>> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let mut out_row: Vec<String> = Vec::new();
        if show_index {
            out_row.push((index + 1).to_string());
        }

        for column in &columns {
            let cell = row
                .get(&column.key)
                .map(|value| normalize_cell(value, &null_display))
                .unwrap_or_else(|| null_display.clone());
            out_row.push(truncate(&cell, configuration.truncate_to));
        }

        rendered_rows.push(out_row);
    }

    let mut widths = compute_base_widths(&head, &rendered_rows, &explicit_widths);

    if let Some(max_width) = max_width {
        if max_width > 0 {
            let mut min_widths: Vec<usize> = Vec::new();
            for (idx, width) in widths.iter().enumerate() {
                let head_col = head.get(idx).map(String::as_str).unwrap_or("");
                let min_by_mode = if wrap {
                    if head_col == "#" {
                        1
                    } else {
                        3
                    }
                } else {
                    max(1, char_len(head_col))
                };
                min_widths.push(min(*width, min_by_mode));
            }

            let separator_overhead = max(0, widths.len().saturating_sub(1)) * 3;
            widths = shrink_widths_to_fit(&widths, &min_widths, max_width, separator_overhead);
        }
    }

    let rendered = match mode {
        TableMode::Minimal => render_minimal_table(&head, &aligns, &widths, &rendered_rows, wrap),
        TableMode::Box => render_box_table(&head, &aligns, &widths, &rendered_rows, wrap),
    };

    Ok(rendered)
}

pub fn render_kv(
    data: Value,
    configuration: Option<KvConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let record = to_object(
        &parsed,
        "render_kv expects an object (or JSON object string)",
    )?;

    let separator = config.separator.unwrap_or_else(|| " : ".to_string());
    let mut keys: Vec<String> = Vec::new();
    let mut ordered = HashSet::new();

    if let Some(key_order) = config.key_order {
        for key in key_order {
            if record.contains_key(&key) {
                ordered.insert(key.clone());
                keys.push(key);
            }
        }
    }

    for key in record.keys() {
        if !ordered.contains(key) {
            keys.push(key.clone());
        }
    }

    if keys.is_empty() {
        return Ok("(no fields)".to_string());
    }

    let key_width = keys
        .iter()
        .map(|key| char_len(key))
        .max()
        .unwrap_or(0)
        .max(config.key_min_width.unwrap_or(0));

    let wrap = config.wrap.unwrap_or(true);
    let value_width = config.max_width.map(|max_width| {
        max(
            8,
            max_width.saturating_sub(key_width + char_len(&separator)),
        )
    });

    let mut out: Vec<String> = Vec::new();
    if let Some(title) = config.title {
        out.push(title.clone());
        out.push("-".repeat(char_len(&title)));
    }

    for key in keys {
        let value = record
            .get(&key)
            .map(|value| as_string(value, "-"))
            .unwrap_or_else(|| "-".to_string());

        let lines = match value_width {
            Some(width) => wrap_text(&value, width, wrap),
            None => vec![value],
        };

        let first = lines.first().cloned().unwrap_or_default();
        out.push(format!(
            "{}{}{}",
            pad_cell(&key, key_width, Alignment::Left),
            separator,
            first
        ));

        for line in lines.iter().skip(1) {
            out.push(format!("{}{}{}", " ".repeat(key_width), separator, line));
        }
    }

    Ok(out.join("\n"))
}

pub fn render_stats(
    data: Value,
    configuration: Option<StatsConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let mut rows: Vec<Map<String, Value>> = Vec::new();

    match parsed {
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                let mut row = Map::new();
                if let Some(object) = item.as_object() {
                    row.insert(
                        "metric".to_string(),
                        Value::String(as_string(
                            object
                                .get("metric")
                                .or_else(|| object.get("label"))
                                .unwrap_or(&Value::String(format!("metric_{}", idx + 1))),
                            "-",
                        )),
                    );
                    row.insert(
                        "value".to_string(),
                        Value::String(as_string(
                            object
                                .get("value")
                                .or_else(|| object.get("amount"))
                                .or_else(|| object.get("current"))
                                .unwrap_or(&Value::String("-".to_string())),
                            "-",
                        )),
                    );
                    if let Some(change) = object.get("change") {
                        row.insert("change".to_string(), Value::String(as_string(change, "-")));
                    }
                    if let Some(trend) = object.get("trend") {
                        row.insert("trend".to_string(), Value::String(as_string(trend, "-")));
                    }
                } else {
                    row.insert(
                        "metric".to_string(),
                        Value::String(format!("metric_{}", idx + 1)),
                    );
                    row.insert("value".to_string(), Value::String(as_string(item, "-")));
                }
                rows.push(row);
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                let mut row = Map::new();
                row.insert("metric".to_string(), Value::String(key));
                row.insert("value".to_string(), Value::String(as_string(&value, "-")));
                rows.push(row);
            }
        }
        _ => {
            return Err(RenderError::invalid(
                "render_stats expects an array/object (or JSON string)",
            ))
        }
    }

    if rows.is_empty() {
        return Ok("(no metrics)".to_string());
    }

    if config.compact.unwrap_or(false) {
        let mut parts: Vec<String> = Vec::new();
        for row in rows {
            let metric = row
                .get("metric")
                .map(|value| as_string(value, "-"))
                .unwrap_or_else(|| "-".to_string());
            let value = row
                .get("value")
                .map(|value| as_string(value, "-"))
                .unwrap_or_else(|| "-".to_string());
            let mut line = format!("{}: {}", metric, value);
            if let Some(change) = row.get("change") {
                line.push_str(&format!(" ({})", as_string(change, "-")));
            }
            parts.push(line);
        }
        return Ok(parts.join(" | "));
    }

    let table_config = TableConfiguration {
        mode: Some(TableMode::Minimal),
        wrap: Some(true),
        max_width: config.max_width,
        columns: Some(vec![
            TableColumn {
                key: "metric".to_string(),
                header: Some("Metric".to_string()),
                align: None,
                width: None,
            },
            TableColumn {
                key: "value".to_string(),
                header: Some("Value".to_string()),
                align: Some(Alignment::Right),
                width: None,
            },
            TableColumn {
                key: "change".to_string(),
                header: Some("Change".to_string()),
                align: Some(Alignment::Right),
                width: None,
            },
            TableColumn {
                key: "trend".to_string(),
                header: Some("Trend".to_string()),
                align: None,
                width: None,
            },
        ]),
        ..Default::default()
    };

    render_table(
        Value::Array(rows.into_iter().map(Value::Object).collect()),
        Some("json"),
        Some(table_config),
    )
}

pub fn render_progress(
    data: Value,
    configuration: Option<ProgressConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let bar_width = max(5, config.bar_width.unwrap_or(24));
    let show_percent = config.show_percent.unwrap_or(true);

    let items = match parsed {
        Value::Array(items) => items,
        other => vec![other],
    };

    let mut rows: Vec<(String, f64, String)> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let object = item.as_object().cloned().unwrap_or_default();
        let label = as_string(
            object
                .get("label")
                .or_else(|| object.get("name"))
                .unwrap_or(&Value::String(format!("step_{}", idx + 1))),
            "-",
        );

        let mut percent = object.get("percent").and_then(to_f64).unwrap_or(0.0);

        if percent == 0.0 {
            if let (Some(current), Some(total)) = (
                object.get("current").and_then(to_f64),
                object.get("total").and_then(to_f64),
            ) {
                if total > 0.0 {
                    percent = (current / total) * 100.0;
                }
            }
        }

        percent = percent.clamp(0.0, 100.0);
        let status = object
            .get("status")
            .map(|value| as_string(value, ""))
            .unwrap_or_default();

        let mut suffix_parts: Vec<String> = Vec::new();
        if show_percent {
            suffix_parts.push(format!("{}%", percent.round() as i64));
        }
        if !status.is_empty() {
            suffix_parts.push(status);
        }

        rows.push((label, percent, suffix_parts.join(" ")));
    }

    let label_width = rows
        .iter()
        .map(|(label, _, _)| char_len(label))
        .max()
        .unwrap_or(0);

    let mut out: Vec<String> = Vec::new();
    for (label, percent, suffix) in rows {
        let filled = ((percent / 100.0) * bar_width as f64).round() as usize;
        let bar = format!("{}{}", "#".repeat(filled), "-".repeat(bar_width - filled));
        let line = format!(
            "{} [{}]{}",
            pad_cell(&label, label_width, Alignment::Left),
            bar,
            if suffix.is_empty() {
                String::new()
            } else {
                format!(" {}", suffix)
            }
        );
        out.push(line);
    }

    Ok(out.join("\n"))
}

fn value_to_tree(label: &str, value: &Value) -> TreeNode {
    if let Some(items) = value.as_array() {
        let mut children: Vec<TreeNode> = Vec::new();
        for (idx, child) in items.iter().enumerate() {
            if let Some(object) = child.as_object() {
                let child_label = as_string(
                    object
                        .get("label")
                        .or_else(|| object.get("name"))
                        .unwrap_or(&Value::String(format!("item_{}", idx + 1))),
                    "-",
                );
                let child_children = object
                    .get("children")
                    .and_then(|value| value.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .enumerate()
                            .map(|(child_idx, grand_child)| {
                                value_to_tree(&format!("item_{}", child_idx + 1), grand_child)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                children.push(TreeNode {
                    label: child_label,
                    children: child_children,
                });
            } else {
                children.push(TreeNode {
                    label: as_string(child, "-"),
                    children: Vec::new(),
                });
            }
        }

        return TreeNode {
            label: label.to_string(),
            children,
        };
    }

    if let Some(object) = value.as_object() {
        if object.is_empty() {
            return TreeNode {
                label: label.to_string(),
                children: Vec::new(),
            };
        }

        return TreeNode {
            label: label.to_string(),
            children: object
                .iter()
                .map(|(key, child)| value_to_tree(key, child))
                .collect(),
        };
    }

    TreeNode {
        label: format!("{}: {}", label, as_string(value, "-")),
        children: Vec::new(),
    }
}

fn normalize_tree_input(value: &Value) -> Vec<TreeNode> {
    if let Some(items) = value.as_array() {
        let mut nodes: Vec<TreeNode> = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            if let Some(object) = item.as_object() {
                let label = as_string(
                    object
                        .get("label")
                        .or_else(|| object.get("name"))
                        .unwrap_or(&Value::String(format!("node_{}", idx + 1))),
                    "-",
                );
                let children = object
                    .get("children")
                    .and_then(|value| value.as_array())
                    .map(|children| normalize_tree_input(&Value::Array(children.clone())))
                    .unwrap_or_default();
                nodes.push(TreeNode { label, children });
            } else {
                nodes.push(value_to_tree(&format!("node_{}", idx + 1), item));
            }
        }
        return nodes;
    }

    if let Some(object) = value.as_object() {
        if let Some(label) = object.get("label") {
            if label.is_string() {
                let children = object
                    .get("children")
                    .and_then(|value| value.as_array())
                    .map(|children| normalize_tree_input(&Value::Array(children.clone())))
                    .unwrap_or_default();
                return vec![TreeNode {
                    label: as_string(label, "-"),
                    children,
                }];
            }
        }

        return object
            .iter()
            .map(|(key, value)| value_to_tree(key, value))
            .collect();
    }

    vec![TreeNode {
        label: as_string(value, "-"),
        children: Vec::new(),
    }]
}

fn render_tree_nodes(
    nodes: &[TreeNode],
    lines: &mut Vec<String>,
    prefix: &str,
    depth: usize,
    max_depth: Option<usize>,
) {
    if let Some(max_depth) = max_depth {
        if depth > max_depth {
            return;
        }
    }

    for (idx, node) in nodes.iter().enumerate() {
        let is_last = idx == nodes.len() - 1;
        let connector = if depth == 0 {
            ""
        } else if is_last {
            "`-- "
        } else {
            "|-- "
        };

        lines.push(format!("{}{}{}", prefix, connector, node.label));

        if !node.children.is_empty() {
            let child_prefix = if depth == 0 {
                prefix.to_string()
            } else if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}|   ", prefix)
            };
            render_tree_nodes(&node.children, lines, &child_prefix, depth + 1, max_depth);
        }
    }
}

pub fn render_tree(
    data: Value,
    configuration: Option<TreeConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let nodes = normalize_tree_input(&parsed);
    let mut lines: Vec<String> = Vec::new();
    render_tree_nodes(&nodes, &mut lines, "", 0, config.max_depth);
    Ok(lines.join("\n"))
}

fn build_diff_ops(before_lines: &[String], after_lines: &[String]) -> Vec<DiffOp> {
    let n = before_lines.len();
    let m = after_lines.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if before_lines[i] == after_lines[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = max(dp[i + 1][j], dp[i][j + 1]);
            }
        }
    }

    let mut ops: Vec<DiffOp> = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < n && j < m {
        if before_lines[i] == after_lines[j] {
            ops.push(DiffOp::Equal(before_lines[i].clone()));
            i += 1;
            j += 1;
            continue;
        }

        if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(DiffOp::Remove(before_lines[i].clone()));
            i += 1;
        } else {
            ops.push(DiffOp::Add(after_lines[j].clone()));
            j += 1;
        }
    }

    while i < n {
        ops.push(DiffOp::Remove(before_lines[i].clone()));
        i += 1;
    }

    while j < m {
        ops.push(DiffOp::Add(after_lines[j].clone()));
        j += 1;
    }

    ops
}

fn apply_diff_context(ops: &[DiffOp], context: usize) -> Vec<DiffOp> {
    let mut out: Vec<DiffOp> = Vec::new();
    let mut idx = 0;

    while idx < ops.len() {
        if !matches!(ops[idx], DiffOp::Equal(_)) {
            out.push(ops[idx].clone());
            idx += 1;
            continue;
        }

        let mut end = idx;
        while end < ops.len() && matches!(ops[end], DiffOp::Equal(_)) {
            end += 1;
        }

        let run_length = end - idx;
        if run_length > context * 2 {
            out.extend_from_slice(&ops[idx..idx + context]);
            out.push(DiffOp::Skip(run_length - context * 2));
            out.extend_from_slice(&ops[end - context..end]);
        } else {
            out.extend_from_slice(&ops[idx..end]);
        }

        idx = end;
    }

    out
}

fn split_lines(value: &str) -> Vec<String> {
    value
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect()
}

pub fn render_diff(
    data: Value,
    configuration: Option<DiffConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let object = to_object(
        &parsed,
        "render_diff expects { before, after } (or a JSON string)",
    )?;

    let before = object
        .get("before")
        .map(|value| as_string(value, ""))
        .unwrap_or_default();
    let after = object
        .get("after")
        .map(|value| as_string(value, ""))
        .unwrap_or_default();

    let context = config.context.unwrap_or(3);

    let before_lines = split_lines(&before);
    let after_lines = split_lines(&after);
    let ops = build_diff_ops(&before_lines, &after_lines);
    let limited = apply_diff_context(&ops, context);

    let mut out: Vec<String> = Vec::new();
    if config.show_header.unwrap_or(true) {
        out.push("--- before".to_string());
        out.push("+++ after".to_string());
    }

    for op in limited {
        match op {
            DiffOp::Equal(line) => out.push(format!(" {}", line)),
            DiffOp::Add(line) => out.push(format!("+{}", line)),
            DiffOp::Remove(line) => out.push(format!("-{}", line)),
            DiffOp::Skip(count) => out.push(format!("... ({} unchanged lines)", count)),
        }
    }

    Ok(out.join("\n"))
}

pub fn render_logs(
    data: Value,
    configuration: Option<LogsConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let logs = to_array(&parsed, "render_logs expects an array of log objects")?;

    let prioritized = config.columns.unwrap_or_else(|| {
        vec![
            "timestamp".to_string(),
            "level".to_string(),
            "source".to_string(),
            "message".to_string(),
        ]
    });

    let mut normalized_rows: Vec<Map<String, Value>> = Vec::new();
    for (idx, item) in logs.iter().enumerate() {
        if let Some(object) = item.as_object() {
            let mut out: Map<String, Value> = Map::new();
            out.insert("idx".to_string(), Value::Number((idx + 1).into()));
            for column in &prioritized {
                let mut value = object
                    .get(column)
                    .map(|value| as_string(value, "-"))
                    .unwrap_or_else(|| "-".to_string());
                if column == "level" && config.uppercase_level.unwrap_or(false) {
                    value = value.to_uppercase();
                }
                out.insert(column.clone(), Value::String(value));
            }

            for (key, value) in object {
                if !out.contains_key(key) {
                    out.insert(key.clone(), Value::String(as_string(value, "-")));
                }
            }
            normalized_rows.push(out);
        } else {
            let mut row = Map::new();
            row.insert("idx".to_string(), Value::Number((idx + 1).into()));
            row.insert("timestamp".to_string(), Value::String("-".to_string()));
            row.insert("level".to_string(), Value::String("INFO".to_string()));
            row.insert("source".to_string(), Value::String("-".to_string()));
            row.insert("message".to_string(), Value::String(as_string(item, "-")));
            normalized_rows.push(row);
        }
    }

    if normalized_rows.is_empty() {
        return Ok("(no rows)".to_string());
    }

    let mut columns: Vec<String> = Vec::new();
    if let Some(first) = normalized_rows.first() {
        for key in first.keys() {
            if key != "idx" {
                columns.push(key.clone());
            }
        }
    }

    let table_columns = columns
        .into_iter()
        .map(|key| TableColumn {
            header: Some(key.clone()),
            align: if key == "level" {
                Some(Alignment::Center)
            } else {
                Some(Alignment::Left)
            },
            key,
            width: None,
        })
        .collect::<Vec<_>>();

    let table_config = TableConfiguration {
        mode: config.mode,
        wrap: Some(true),
        max_width: config.max_width,
        columns: Some(table_columns),
        ..Default::default()
    };

    render_table(
        Value::Array(normalized_rows.into_iter().map(Value::Object).collect()),
        Some("json"),
        Some(table_config),
    )
}

pub fn render_alerts(
    data: Value,
    configuration: Option<AlertsConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let items = if let Some(items) = parsed.as_array() {
        items.clone()
    } else {
        vec![parsed]
    };

    let include_level = config.include_level.unwrap_or(true);
    let include_timestamp = config.include_timestamp.unwrap_or(false);
    let compact = config.compact.unwrap_or(false);

    let mut lines: Vec<String> = Vec::new();
    for item in items {
        let object = item.as_object().cloned().unwrap_or_else(|| {
            let mut map = Map::new();
            map.insert("message".to_string(), item);
            map
        });

        let level = object
            .get("level")
            .map(|value| as_string(value, "info"))
            .unwrap_or_else(|| "info".to_string())
            .to_uppercase();
        let title = object
            .get("title")
            .map(|value| as_string(value, ""))
            .unwrap_or_default();
        let message = object
            .get("message")
            .or_else(|| object.get("text"))
            .map(|value| as_string(value, "-"))
            .unwrap_or_else(|| "-".to_string());
        let timestamp = object
            .get("timestamp")
            .map(|value| as_string(value, ""))
            .unwrap_or_default();

        if compact {
            lines.push(format!(
                "{}{}",
                if include_level {
                    format!("{}: ", level)
                } else {
                    String::new()
                },
                message
            ));
            continue;
        }

        let head = if include_level {
            format!("[{}] ", level)
        } else {
            String::new()
        };
        let with_title = if title.is_empty() {
            message
        } else {
            format!("{} - {}", title, message)
        };
        let with_ts = if include_timestamp && !timestamp.is_empty() {
            format!("{} ({})", with_title, timestamp)
        } else {
            with_title
        };

        lines.push(format!("{}{}", head, with_ts));
    }

    Ok(lines.join("\n"))
}

pub fn render_list(
    data: Value,
    configuration: Option<ListConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let items = to_array(&parsed, "render_list expects an array")?;

    let ordered = config.ordered.unwrap_or(false);
    let checkbox = config.checkbox.unwrap_or(false);
    let start = config.start.unwrap_or(1);
    let bullet = config.bullet.unwrap_or_else(|| "-".to_string());

    let mut out: Vec<String> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let object = item.as_object().cloned().unwrap_or_else(|| {
            let mut map = Map::new();
            map.insert("text".to_string(), item.clone());
            map
        });

        let text = object
            .get("text")
            .or_else(|| object.get("label"))
            .or_else(|| object.get("value"))
            .map(|value| as_string(value, "-"))
            .unwrap_or_else(|| "-".to_string());
        let meta = object
            .get("meta")
            .map(|value| as_string(value, ""))
            .unwrap_or_default();
        let checked = object
            .get("checked")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let list_prefix = if ordered {
            format!("{}.", start + idx as i64)
        } else {
            bullet.clone()
        };

        let checkbox_prefix = if checkbox {
            format!("[{}] ", if checked { "x" } else { " " })
        } else {
            String::new()
        };

        let meta_suffix = if meta.is_empty() {
            String::new()
        } else {
            format!(" ({})", meta)
        };

        out.push(format!(
            "{} {}{}{}",
            list_prefix, checkbox_prefix, text, meta_suffix
        ));
    }

    Ok(out.join("\n"))
}

pub fn render_chart_bar(
    data: Value,
    configuration: Option<ChartBarConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();

    let items: Vec<Value> = match parsed {
        Value::Array(items) => items,
        Value::Object(object) => object
            .into_iter()
            .map(|(label, value)| {
                let mut map = Map::new();
                map.insert("label".to_string(), Value::String(label));
                map.insert("value".to_string(), value);
                Value::Object(map)
            })
            .collect(),
        _ => {
            return Err(RenderError::invalid(
                "render_chart_bar expects array/object",
            ))
        }
    };

    let mut rows: Vec<(String, f64)> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if let Some(object) = item.as_object() {
            let label = as_string(
                object
                    .get("label")
                    .or_else(|| object.get("name"))
                    .unwrap_or(&Value::String(format!("item_{}", idx + 1))),
                "-",
            );
            let value = object
                .get("value")
                .or_else(|| object.get("amount"))
                .and_then(to_f64)
                .unwrap_or(0.0);
            rows.push((label, value));
        } else {
            rows.push((format!("item_{}", idx + 1), to_f64(item).unwrap_or(0.0)));
        }
    }

    if rows.is_empty() {
        return Ok("(no bars)".to_string());
    }

    let width = max(5, config.width.unwrap_or(24));
    let bar_char = config
        .bar_char
        .as_deref()
        .unwrap_or("#")
        .chars()
        .next()
        .unwrap_or('#');
    let show_values = config.show_values.unwrap_or(true);
    let max_label_width = config.max_label_width.unwrap_or_else(|| {
        rows.iter()
            .map(|(label, _)| char_len(label))
            .max()
            .unwrap_or(0)
    });

    let max_abs = rows
        .iter()
        .map(|(_, value)| value.abs())
        .fold(1.0f64, f64::max);

    let mut out: Vec<String> = Vec::new();
    for (label, value) in rows {
        let scaled = ((value.abs() / max_abs) * width as f64).round() as usize;
        let bar = bar_char.to_string().repeat(scaled);
        let sign = if value < 0.0 { "-" } else { "" };
        let label = pad_cell(
            &truncate(&label, Some(max_label_width)),
            max_label_width,
            Alignment::Left,
        );
        let value_suffix = if show_values {
            format!(" {}", value)
        } else {
            String::new()
        };

        out.push(format!("{} | {}{}{}", label, sign, bar, value_suffix));
    }

    Ok(out.join("\n"))
}

pub fn render_sparkline(
    data: Value,
    configuration: Option<SparklineConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();

    let values: Vec<f64> = if let Some(items) = parsed.as_array() {
        items.iter().filter_map(to_f64).collect()
    } else if let Some(object) = parsed.as_object() {
        object
            .get("values")
            .and_then(|value| value.as_array())
            .map(|items| items.iter().filter_map(to_f64).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if values.is_empty() {
        return Ok("(no points)".to_string());
    }

    let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let min_value = config
        .min
        .unwrap_or_else(|| values.iter().copied().fold(f64::INFINITY, f64::min));
    let max_value = config
        .max
        .unwrap_or_else(|| values.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    let range = max_value - min_value;

    let spark = values
        .iter()
        .map(|value| {
            if range <= 0.0 {
                return bars[(bars.len() - 1) / 2].to_string();
            }
            let normalized = (value - min_value) / range;
            let idx = min(
                bars.len() - 1,
                max(0, (normalized * (bars.len() - 1) as f64).round() as usize),
            );
            bars[idx].to_string()
        })
        .collect::<Vec<_>>()
        .join("");

    let prefix = config
        .label
        .map(|label| format!("{}: ", label))
        .unwrap_or_default();

    Ok(format!(
        "{}{} [min={}, max={}]",
        prefix, spark, min_value, max_value
    ))
}

fn parse_time(value: &str) -> i64 {
    if value.is_empty() {
        return i64::MAX;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return dt.timestamp_millis();
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return dt.and_utc().timestamp_millis();
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M") {
        return dt.and_utc().timestamp_millis();
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        if let Some(dt) = date.and_hms_opt(0, 0, 0) {
            return dt.and_utc().timestamp_millis();
        }
    }

    i64::MAX
}

pub fn render_timeline(
    data: Value,
    configuration: Option<TimelineConfiguration>,
) -> Result<String, RenderError> {
    let parsed = parse_maybe_json(data);
    let config = configuration.unwrap_or_default();
    let wrap = config.wrap.unwrap_or(true);

    let items = to_array(&parsed, "render_timeline expects an array of events")?;

    let mut events: Vec<HashMap<String, String>> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let object = item.as_object().cloned().unwrap_or_else(|| {
            let mut map = Map::new();
            map.insert("title".to_string(), Value::String(as_string(item, "-")));
            map.insert("time".to_string(), Value::String(String::new()));
            map.insert("status".to_string(), Value::String(String::new()));
            map
        });

        let mut event = HashMap::new();
        event.insert(
            "time".to_string(),
            as_string(
                object
                    .get("time")
                    .or_else(|| object.get("timestamp"))
                    .unwrap_or(&Value::String(String::new())),
                "",
            ),
        );
        event.insert(
            "title".to_string(),
            as_string(
                object
                    .get("title")
                    .or_else(|| object.get("name"))
                    .unwrap_or(&Value::String(format!("event_{}", idx + 1))),
                "-",
            ),
        );
        event.insert(
            "detail".to_string(),
            as_string(
                object
                    .get("detail")
                    .or_else(|| object.get("description"))
                    .unwrap_or(&Value::String(String::new())),
                "",
            ),
        );
        event.insert(
            "status".to_string(),
            as_string(
                object
                    .get("status")
                    .unwrap_or(&Value::String(String::new())),
                "",
            ),
        );
        events.push(event);
    }

    if config.sort.unwrap_or(false) {
        events.sort_by_key(|event| parse_time(event.get("time").map(String::as_str).unwrap_or("")));
    }

    let status_marker = |status: &str| -> &'static str {
        match status.to_lowercase().as_str() {
            "done" | "completed" => "o",
            "active" | "running" | "in_progress" => "*",
            "blocked" | "error" => "x",
            _ => ".",
        }
    };

    let time_width = events
        .iter()
        .map(|event| char_len(event.get("time").map(String::as_str).unwrap_or("")))
        .max()
        .unwrap_or(0);

    let mut out: Vec<String> = Vec::new();
    for event in &events {
        let time = event.get("time").cloned().unwrap_or_default();
        let title = event.get("title").cloned().unwrap_or_default();
        let detail = event.get("detail").cloned().unwrap_or_default();
        let status = event.get("status").cloned().unwrap_or_default();

        let marker = if config.show_status == Some(false) {
            "-"
        } else {
            status_marker(&status)
        };

        let prefix = format!(
            "{} | {} ",
            pad_cell(&time, time_width, Alignment::Left),
            marker
        );
        out.push(format!("{}{}", prefix, title));

        if !detail.is_empty() {
            let detail_width = config
                .max_width
                .map(|max_width| max(12, max_width.saturating_sub(char_len(&prefix))))
                .unwrap_or_else(|| max(12, 80usize.saturating_sub(char_len(&prefix))));

            for line in wrap_text(&detail, detail_width, wrap) {
                out.push(format!("{} |   {}", " ".repeat(time_width), line));
            }
        }
    }

    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_table_minimal_smoke() {
        let data = json!([
            {"name":"Ada","role":"Engineer"},
            {"name":"Linus","role":"Maintainer"}
        ]);
        let out = render_table(data, Some("json"), None).expect("table");
        assert!(out.contains("name"));
        assert!(out.contains("Ada"));
    }

    #[test]
    fn render_kv_smoke() {
        let data = json!({"env":"dev","build":42});
        let out = render_kv(data, None).expect("kv");
        assert!(out.contains("env"));
        assert!(out.contains("dev"));
    }

    #[test]
    fn render_stats_compact_smoke() {
        let data = json!({"tasks": 10, "done": 8});
        let out = render_stats(
            data,
            Some(StatsConfiguration {
                compact: Some(true),
                ..Default::default()
            }),
        )
        .expect("stats");
        assert!(out.contains("tasks"));
    }

    #[test]
    fn render_diff_smoke() {
        let data = json!({"before":"a\nb","after":"a\nc"});
        let out = render_diff(data, None).expect("diff");
        assert!(out.contains("--- before"));
        assert!(out.contains("+c"));
    }

    #[test]
    fn render_sparkline_smoke() {
        let data = json!([1, 2, 3, 2, 5]);
        let out = render_sparkline(data, None).expect("sparkline");
        assert!(out.contains("min="));
        assert!(out.contains("max="));
    }

    #[test]
    fn dispatch_tool_smoke() {
        let result = dispatch_tool(
            "render_list",
            &json!({"data":[{"text":"a"},{"text":"b"}],"configuration":{"ordered":true}}),
        )
        .expect("dispatch");
        assert!(result["text"].as_str().unwrap_or("").contains("1."));
    }
}
