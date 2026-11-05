//! `laminapdf schema`: every command's document is valid JSON Schema 2020-12, its `output` is
//! the committed schema of the report, and the index lists every committed schema.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers: a panic is a test failure"
)]

mod common;

use common::{laminapdf, root};
use serde_json::Value;

#[test]
fn every_command_schema_should_be_valid_json_schema_2020_12() {
    for command in laminapdf_cli::schema::COMMANDS {
        let run = laminapdf(&["schema", command, "--json"]);
        assert_eq!(run.code, 0, "{command}: {}", run.stderr);
        let schema = run.json();
        let mut compiler = boon::Compiler::new();
        // Compiling validates the document against the 2020-12 meta-schema.
        compiler.enable_format_assertions();
        for path in [
            "packages/sdk/schemas/error.schema.json",
            "crates/laminapdf-extract/schemas/extract.schema.json",
        ] {
            let resource: Value = serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap();
            compiler
                .add_resource(resource["$id"].as_str().unwrap(), resource.clone())
                .unwrap();
        }
        let id = schema["$id"].as_str().unwrap().to_owned();
        compiler.add_resource(&id, schema).unwrap();
        let mut schemas = boon::Schemas::new();
        compiler
            .compile(&id, &mut schemas)
            .unwrap_or_else(|e| panic!("{command}: {e}"));
    }
}

#[test]
fn a_commands_output_should_be_its_committed_schema() {
    let committed = |file: &str| -> Value {
        serde_json::from_slice(&std::fs::read(root().join("crates/laminapdf-cli/schemas").join(file)).unwrap()).unwrap()
    };
    let info = laminapdf(&["schema", "info"]).json();
    assert_eq!(info["properties"]["output"], committed("info.schema.json"));
    assert_eq!(info["properties"]["summary"], committed("summary-line.schema.json"));
    let rotate = laminapdf(&["schema", "pages-rotate"]).json();
    assert_eq!(rotate["properties"]["output"], committed("pages.schema.json"));
    assert!(rotate["properties"].get("result").is_none());
}

#[test]
fn the_index_should_list_every_committed_schema() {
    let index = laminapdf(&["schema"]).json();
    let ids = index["schemas"].as_array().unwrap();
    assert_eq!(ids.len(), laminapdf_cli::schema::all().len());
    let unknown = laminapdf(&["schema", "inf", "--json"]);
    assert_eq!(unknown.code, 2);
    assert_eq!(unknown.stderr_json()[0]["error"]["didYouMean"][0], "info");
}
