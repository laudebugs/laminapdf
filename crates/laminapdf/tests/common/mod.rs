//! The element-flow example's report (`fixtures/text/element-flow-report.json`), for the test files
//! that build it. Each one declares the example as `mod element_flow`.

use laminapdf_test_fixtures::json;

/// The report's text, owned, so a test can change a field before borrowing it.
pub(crate) struct FlowReport {
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) notes: String,
    pub(crate) body: String,
}

impl FlowReport {
    /// The fixture's report.
    pub(crate) fn load() -> Self {
        let report = json("text/element-flow-report.json");
        let field = |name: &str| {
            let text = report.get(name).and_then(serde_json::Value::as_str);
            text.unwrap_or_else(|| panic!("element-flow-report.json: no {name}"))
                .to_owned()
        };
        Self {
            title: field("title"),
            summary: field("summary"),
            notes: field("notes"),
            body: field("body"),
        }
    }

    /// The report as the example takes it.
    pub(crate) fn as_report(&self) -> crate::element_flow::Report<'_> {
        crate::element_flow::Report {
            title: &self.title,
            summary: &self.summary,
            notes: &self.notes,
            body: &self.body,
        }
    }
}
