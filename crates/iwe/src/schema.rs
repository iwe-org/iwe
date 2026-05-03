use liwe::schema::FieldSchema;

fn format_types(field: &FieldSchema) -> String {
    field
        .types
        .iter()
        .map(|t| format!("{} ({:.0}%)", t.yaml_type, t.percentage))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_coverage(field: &FieldSchema) -> String {
    format!(
        "{} ({:.0}%)",
        field.coverage.count, field.coverage.percentage
    )
}

fn format_values(field: &FieldSchema) -> String {
    if field.values.is_empty() {
        "---".to_string()
    } else {
        field
            .values
            .iter()
            .map(|v| format!("{} ({})", v.value, v.count))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

const ALIGNED_COLS: usize = 4;

pub fn render_schema(fields: &[FieldSchema]) -> String {
    let headers = ["Field", "Types", "Coverage", "Distinct", "Values"];

    let rows: Vec<[String; 5]> = fields
        .iter()
        .map(|f| {
            [
                f.name.clone(),
                format_types(f),
                format_coverage(f),
                f.distinct.to_string(),
                format_values(f),
            ]
        })
        .collect();

    let widths: [usize; ALIGNED_COLS] = std::array::from_fn(|i| {
        let header_w = headers[i].len();
        let max_row_w = rows.iter().map(|r| r[i].len()).max().unwrap_or(0);
        header_w.max(max_row_w)
    });

    let mut out = String::new();

    out.push_str(&format_row(&headers.map(String::from), &widths));
    out.push('|');
    for w in &widths {
        out.push(' ');
        for _ in 0..*w {
            out.push('-');
        }
        out.push_str(" |");
    }
    out.push_str(" --- |\n");

    for row in &rows {
        out.push_str(&format_row(row, &widths));
    }

    out
}

fn format_row(cells: &[String; 5], widths: &[usize; ALIGNED_COLS]) -> String {
    let mut row = String::from("|");
    for (cell, width) in cells[..ALIGNED_COLS].iter().zip(widths.iter()) {
        row.push_str(&format!(" {:<width$} |", cell, width = width));
    }
    row.push_str(&format!(" {} |\n", cells[ALIGNED_COLS]));
    row
}
