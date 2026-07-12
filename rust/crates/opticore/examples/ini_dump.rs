//! Dev harness: dump a parsed OptiScaler.ini as section|key|type|value lines
//! for parity diffing against the Python reader.

use opticore::ini::{read_file, ValueKind};
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("ini path argument");
    let doc = read_file(Path::new(&path)).expect("readable ini");
    let mut lines: Vec<String> = Vec::new();
    for section in &doc.sections {
        for entry in &section.entries {
            let kind = match &entry.kind {
                ValueKind::BoolOptions => "bool_options",
                ValueKind::Options(_) => "options",
                ValueKind::Int => "int",
                ValueKind::Float => "float",
                ValueKind::Text => "string",
            };
            lines.push(format!(
                "{}|{}|{}|{}",
                section.name, entry.key, kind, entry.value
            ));
        }
    }
    lines.sort();
    println!("{}", lines.join("\n"));
}
