//! The committed schemas, `laminapdf.error/1` and `laminapdf.extract/1`, compiled, to validate
//! what the binary prints.
#![allow(clippy::indexing_slicing, reason = "test helpers: a missing key is a test failure")]

use serde_json::Value;

use super::root;

/// Every committed schema, and the two they refer to, compiled.
pub struct Validator {
    schemas: boon::Schemas,
    compiler_ids: Vec<(String, boon::SchemaIndex)>,
}

/// A JSON file under the repository root.
fn read(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
}

impl Validator {
    pub fn new() -> Self {
        let mut compiler = boon::Compiler::new();
        for path in [
            "packages/sdk/schemas/error.schema.json",
            "crates/laminapdf-extract/schemas/extract.schema.json",
        ] {
            let schema = read(path);
            compiler
                .add_resource(schema["$id"].as_str().unwrap(), schema.clone())
                .unwrap();
        }
        let mut ids = Vec::new();
        for (file, schema) in laminapdf_cli::schema::all() {
            let id = schema["$id"].as_str().unwrap().to_owned();
            compiler.add_resource(&id, schema).unwrap();
            ids.push((file.to_owned(), id));
        }
        let extract = read("crates/laminapdf-extract/schemas/extract.schema.json")["$id"]
            .as_str()
            .unwrap()
            .to_owned();
        ids.push(("extract.schema.json".to_owned(), extract));
        let mut schemas = boon::Schemas::new();
        let compiler_ids = ids
            .into_iter()
            .map(|(file, id)| (file, compiler.compile(&id, &mut schemas).unwrap()))
            .collect();
        Self { schemas, compiler_ids }
    }

    /// Panics unless `value` matches the committed schema `file` (or `extract.schema.json`).
    pub fn check(&self, file: &str, value: &Value) {
        let (_, index) = self.compiler_ids.iter().find(|(f, _)| f == file).unwrap();
        if let Err(e) = self.schemas.validate(value, *index) {
            panic!("{value} does not match {file}:\n{e}");
        }
    }
}
