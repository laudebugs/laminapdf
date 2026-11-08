//! Runs every experiment, writes the PDFs to target/flow-layout/ and prints the measurements.

use std::time::Instant;

use lamina_flow_layout::{fonts, inputs, own, repo, typst_frames, typst_pdf, typst_world::MemoryWorld};

/// Best of `n` runs, in milliseconds.
fn best<T>(n: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut best = f64::INFINITY;
    let mut last = None;
    for _ in 0..n {
        let start = Instant::now();
        let out = f();
        best = best.min(start.elapsed().as_secs_f64() * 1000.0);
        last = Some(out);
    }
    (best, last.unwrap_or_else(|| unreachable!()))
}

fn main() {
    let out = repo().join("spikes/flow-layout/target/flow-layout");
    std::fs::create_dir_all(&out).unwrap();
    let fonts = fonts();
    let invoice_json = std::fs::read(repo().join("spikes/flow-layout/inputs/invoice.json")).unwrap();
    let invoice_typ = std::fs::read_to_string(repo().join("spikes/flow-layout/inputs/invoice.typ")).unwrap();
    let report = inputs::report();

    // Experiment A: our own paginator over Surface (two passes for "Page X of Y").
    let invoice = inputs::invoice(std::str::from_utf8(&invoice_json).unwrap()).unwrap();
    for (name, doc) in [("invoice", &invoice), ("report", &report)] {
        let runs = if name == "report" { 10 } else { 30 };
        let save = || {
            let (mut d, pages, gaps) = own::build(doc, &fonts).unwrap();
            (d.save(&laminapdf::SaveOptions::default()).unwrap().bytes, pages, gaps)
        };
        let (ms, (pdf, pages, gaps)) = best(runs, save);
        let again = save().0;
        std::fs::write(out.join(format!("a-{name}.pdf")), &pdf).unwrap();
        println!("A {name}: {pages} pages, {} B, {ms:.1} ms (both passes), deterministic {}, {gaps:?}", pdf.len(), again == pdf);
    }

    // Experiment B: Typst. Typst memoises across compiles (comemo), so a cold compile evicts the
    // cache first; a warm one is what a long-running service would see.
    let worlds = [
        ("invoice", MemoryWorld::new(invoice_typ, &[("invoice.json", invoice_json.clone())], &fonts)),
        ("report", MemoryWorld::new(inputs::to_typst(&report), &[], &fonts)),
    ];
    for (name, world) in &worlds {
        let runs = if *name == "report" { 10 } else { 30 };
        let (cold, b) = best(runs, || {
            comemo::evict(0);
            typst_pdf::build(world).unwrap()
        });
        let (warm, _) = best(runs, || typst_pdf::build(world).unwrap());
        std::fs::write(out.join(format!("b-{name}.pdf")), &b.pdf).unwrap();
        let again = typst_pdf::build(world).unwrap();
        let edit = typst_pdf::edit(&b.pdf);
        println!(
            "B {name}: {} pages, {} B, {} warnings, cold {cold:.1} ms, warm {warm:.1} ms, deterministic {}",
            b.pages,
            b.pdf.len(),
            b.warnings,
            again.pdf == b.pdf,
        );
        match edit {
            Ok(e) => println!(
                "  lamina: {} pages, tagged {}, incremental +{} B, reopened title {:?}",
                e.pages, e.tagged, e.incremental_bytes, e.reopened_title
            ),
            Err(e) => println!("  lamina: error {} {}", e.code(), e),
        }
    }

    // Experiment C: Typst's frames drawn through Surface, first page only (the time box).
    for (name, world) in &worlds {
        let doc = typst_pdf::layout(world).unwrap();
        let (pdf, unmapped) = typst_frames::draw(&doc, 1).unwrap();
        std::fs::write(out.join(format!("c-{name}.pdf")), &pdf).unwrap();
        println!("C {name} page 1: {} B, {unmapped:?}", pdf.len());
    }
}
