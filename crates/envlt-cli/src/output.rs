use clap::ValueEnum;
use comfy_table::{Cell, ContentArrangement, Table};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Raw,
    Json,
}

pub fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(
        headers
            .iter()
            .map(|header| Cell::new(*header))
            .collect::<Vec<_>>(),
    );

    for row in rows {
        table.add_row(row.iter().map(Cell::new).collect::<Vec<_>>());
    }

    table.to_string()
}

pub fn render_raw_rows(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|row| row.join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
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
