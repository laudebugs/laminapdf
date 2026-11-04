//! The committed JSON Schemas (`schemas/`): each is what the report types generate
//! (`UPDATE_GOLDEN=1` rewrites them), and every `--json` output of the binary validates
//! against its own, errors and warnings on standard error included.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test helpers: a panic is a test failure"
)]

mod common;

use common::{folder, laminapdf, root, run};
use serde_json::Value;

/// The `schemas/` folder.
fn schemas_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

#[test]
fn the_committed_schemas_should_be_the_generated_ones() {
    let mut stale = Vec::new();
    for (file, schema) in laminapdf_cli::schema::all() {
        let path = schemas_dir().join(file);
        let generated = format!("{}\n", serde_json::to_string_pretty(&schema).unwrap());
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(schemas_dir()).unwrap();
            std::fs::write(&path, &generated).unwrap();
        } else if std::fs::read_to_string(&path).unwrap_or_default() != generated {
            stale.push(file);
        }
    }
    assert!(
        stale.is_empty(),
        "stale schemas {stale:?}: rerun with UPDATE_GOLDEN=1 and review the diff"
    );
}

/// Every committed schema and `laminapdf.error/1`, compiled.
struct Validator {
    schemas: boon::Schemas,
    compiler_ids: Vec<(String, boon::SchemaIndex)>,
}

impl Validator {
    fn new() -> Self {
        let mut compiler = boon::Compiler::new();
        let error: Value =
            serde_json::from_slice(&std::fs::read(root().join("packages/sdk/schemas/error.schema.json")).unwrap())
                .unwrap();
        compiler
            .add_resource(error["$id"].as_str().unwrap(), error.clone())
            .unwrap();
        let mut ids = Vec::new();
        for (file, schema) in laminapdf_cli::schema::all() {
            let id = schema["$id"].as_str().unwrap().to_owned();
            compiler.add_resource(&id, schema).unwrap();
            ids.push((file.to_owned(), id));
        }
        let mut schemas = boon::Schemas::new();
        let compiler_ids = ids
            .into_iter()
            .map(|(file, id)| (file, compiler.compile(&id, &mut schemas).unwrap()))
            .collect();
        Self { schemas, compiler_ids }
    }

    fn check(&self, file: &str, value: &Value) {
        let (_, index) = self.compiler_ids.iter().find(|(f, _)| f == file).unwrap();
        if let Err(e) = self.schemas.validate(value, *index) {
            panic!("{value} does not match {file}:\n{e}");
        }
    }
}

#[test]
fn every_report_should_validate_against_its_schema() {
    let validator = Validator::new();
    let out = folder("schemas").join("out.pdf");
    let cases: [(&str, Vec<&str>); 6] = [
        (
            "info.schema.json",
            vec!["info", "fixtures/golden/links-and-outlines.pdf"],
        ),
        ("info.schema.json", vec!["info", "fixtures/corpus/hello-broken.pdf"]),
        ("check.schema.json", vec!["check", "fixtures/corpus/hello-broken.pdf"]),
        (
            "pages.schema.json",
            vec![
                "pages",
                "rotate",
                "fixtures/corpus/hello.pdf",
                "90",
                "-o",
                out.to_str().unwrap(),
            ],
        ),
        (
            "text.schema.json",
            vec!["extract", "fixtures/golden/build-text-multiscript.pdf"],
        ),
        ("explain.schema.json", vec!["explain", "xref-repaired"]),
    ];
    for (file, mut args) in cases {
        args.extend(["--json", "--force"]);
        let run = laminapdf(&args);
        assert!(run.code == 0 || run.code == 3, "{args:?}: {}", run.stderr);
        let report = run.json();
        validator.check(file, &report);
        assert_eq!(
            report["schema"]
                .as_str()
                .map(|s| s.trim_start_matches("laminapdf.").split('/').next().unwrap().to_owned() + ".schema.json"),
            Some(file.to_owned())
        );
    }
}

#[test]
fn every_error_line_should_validate() {
    let validator = Validator::new();
    let runs = [
        laminapdf(&["info", "fixtures/text/encrypted/text-shaped-aes-256.pdf", "--json"]),
        laminapdf(&["extract", "fixtures/corpus/hello.pdf", "--pages", "9", "--json"]),
        laminapdf(&["explain", "nope", "--json"]),
        laminapdf(&["info", "--bogus", "--json"]),
        run(&["info", "-", "--json"], Some(b"not a pdf"), &[]),
    ];
    for run in &runs {
        let lines = run.stderr_json();
        assert_eq!((lines.len(), run.stdout.is_empty()), (1, true), "{}", run.stderr);
        validator.check("error-line.schema.json", &lines[0]);
    }
}
