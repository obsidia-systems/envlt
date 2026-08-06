use std::io::IsTerminal;

use clap::ValueEnum;
use comfy_table::{presets::NOTHING, Attribute, Cell, Color, ContentArrangement, Table};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

/// Per-cell semantic styling for [`render_table_styled`]. Applied only when
/// [`colors_enabled`] is true, matching the same terminal/`NO_COLOR`
/// detection already used for `--help` styling (see `main.rs`), so piped or
/// `NO_COLOR` output stays plain and grepable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellStyle {
    Plain,
    Bold,
    Ok,
    Warn,
    Danger,
}

/// Whether table cells should be colored/bolded: only when stdout is a real
/// terminal and `NO_COLOR` isn't set. `--format json` never goes through
/// this -- it's always plain, regardless of tty.
fn colors_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn base_table(headers: &[&str]) -> Table {
    let mut table = Table::new();
    table.load_preset(NOTHING);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let header_cells = headers.iter().map(|header| {
        let cell = Cell::new(header.to_uppercase());
        if colors_enabled() {
            cell.add_attribute(Attribute::Bold)
        } else {
            cell
        }
    });
    table.set_header(header_cells.collect::<Vec<_>>());

    table
}

fn styled_cell(text: &str, style: CellStyle) -> Cell {
    let cell = Cell::new(text);
    if !colors_enabled() {
        return cell;
    }

    match style {
        CellStyle::Plain => cell,
        CellStyle::Bold => cell.add_attribute(Attribute::Bold),
        CellStyle::Ok => cell.fg(Color::Green),
        CellStyle::Warn => cell.fg(Color::Yellow),
        CellStyle::Danger => cell.fg(Color::Red).add_attribute(Attribute::Bold),
    }
}

pub fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = base_table(headers);

    for row in rows {
        table.add_row(row.iter().map(Cell::new).collect::<Vec<_>>());
    }

    table.to_string()
}

/// Like [`render_table`], but lets a caller flag specific cells with
/// semantic styling (e.g. a `secret` type, an `error` severity). `styles`
/// must be the same shape as `rows`; a shorter or missing entry falls back
/// to [`CellStyle::Plain`].
pub fn render_table_styled(
    headers: &[&str],
    rows: &[Vec<String>],
    styles: &[Vec<CellStyle>],
) -> String {
    let mut table = base_table(headers);

    for (row, row_styles) in rows.iter().enumerate().map(|(i, row)| (row, styles.get(i))) {
        let cells = row
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let style = row_styles
                    .and_then(|styles| styles.get(i))
                    .copied()
                    .unwrap_or(CellStyle::Plain);
                styled_cell(text, style)
            })
            .collect::<Vec<_>>();
        table.add_row(cells);
    }

    table.to_string()
}

pub fn rows_to_json_objects(headers: &[&str], rows: &[Vec<String>]) -> Value {
    let objects = rows
        .iter()
        .map(|row| {
            let mut object = Map::new();
            for (index, header) in headers.iter().enumerate() {
                let value = row.get(index).cloned().unwrap_or_default();
                object.insert((*header).to_owned(), Value::String(value));
            }
            Value::Object(object)
        })
        .collect::<Vec<_>>();

    Value::Array(objects)
}
