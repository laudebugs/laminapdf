//! What `cargo xtask wasm` removes from the wasm-bindgen glue of Lamina PDF's modules.
//!
//! `--experimental-reset-state-function` gives the glue `__wbg_reset_state`, and finalizers that
//! skip objects of a replaced instance (ADR 0016 §8). It also checks, at every method call, that
//! the object and its arguments belong to the current instance, and calls `__wbg_call_guard()`,
//! which acts only on a reinit that Rust code schedules (Lamina PDF never does). `@laminapdf/sdk`
//! checks every handle's instance before it calls the glue, so these per-call lines only cost
//! bytes: about 2 KB gzip over the three modules, on a 20 KB budget for the SDK's JavaScript.

use anyhow::{Result, bail};

/// The error the glue throws for an object of a replaced instance.
const STALE: &str = "throw new Error('Invalid stale object from previous Wasm instance');";

/// The call made before every glue call.
const GUARD: &str = "__wbg_call_guard();";

/// `js` without the per-call instance checks and guard calls. Fails when the glue has none of
/// them, or keeps one (a wasm-bindgen whose glue reads differently: update this).
pub(crate) fn strip_call_checks(js: &str) -> Result<String> {
    let lines: Vec<&str> = js.split_inclusive('\n').collect();
    let mut out = String::with_capacity(js.len());
    let (mut checks, mut guards) = (0, 0);
    let mut i = 0;
    while let Some(line) = lines.get(i) {
        let next = |k: usize| lines.get(i + k).map(|l| l.trim());
        if is_stale_check(line.trim()) && next(1) == Some(STALE) && next(2) == Some("}") {
            checks += 1;
            i += 3;
            continue;
        }
        if line.trim() == GUARD {
            guards += 1;
        } else {
            out.push_str(line);
        }
        i += 1;
    }
    if checks == 0 || guards == 0 {
        bail!(
            "the glue has {checks} instance check(s) and {guards} guard call(s) to remove: check wasm-bindgen's output"
        );
    }
    if out.contains(STALE) || out.contains(GUARD) {
        bail!(
            "the glue keeps an instance check or guard call in a form xtask doesn't know: check wasm-bindgen's output"
        );
    }
    Ok(out)
}

/// `if (x.__wbg_inst !== undefined && x.__wbg_inst !== __wbg_instance_id) {`, for `this` or an
/// argument (`(source)`).
fn is_stale_check(line: &str) -> bool {
    line.starts_with("if (")
        && line.contains(".__wbg_inst !== undefined && ")
        && line.ends_with(".__wbg_inst !== __wbg_instance_id) {")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLUE: &str = "class WasmPage {
    index() {
        if (this.__wbg_inst !== undefined && this.__wbg_inst !== __wbg_instance_id) {
            throw new Error('Invalid stale object from previous Wasm instance');
        }
        let ret;
        __wbg_call_guard();
        ret = wasm.wasmpage_index(this.__wbg_ptr);
        return ret;
    }
    static openSource(source) {
        if ((source).__wbg_inst !== undefined && (source).__wbg_inst !== __wbg_instance_id) {
            throw new Error('Invalid stale object from previous Wasm instance');
        }
        __wbg_call_guard();
        return wasm.open(source.__wbg_ptr);
    }
}
function __wbg_call_guard() {
    if (__wbg_reinit_scheduled) {
        __wbg_reset_state();
    }
}
";

    #[test]
    fn the_checks_and_guard_calls_should_go_and_the_rest_stay() {
        let stripped = strip_call_checks(GLUE).ok();
        assert_eq!(
            stripped.as_deref(),
            Some(
                "class WasmPage {
    index() {
        let ret;
        ret = wasm.wasmpage_index(this.__wbg_ptr);
        return ret;
    }
    static openSource(source) {
        return wasm.open(source.__wbg_ptr);
    }
}
function __wbg_call_guard() {
    if (__wbg_reinit_scheduled) {
        __wbg_reset_state();
    }
}
"
            )
        );
    }

    #[test]
    fn glue_without_the_checks_should_fail() {
        assert!(strip_call_checks("class A {\n    f() { return 1; }\n}\n").is_err());
    }

    #[test]
    fn a_check_in_an_unknown_form_should_fail() {
        let glue = GLUE.replace(
            "        if (this.__wbg_inst !== undefined && this.__wbg_inst !== __wbg_instance_id) {\n",
            "        if (this.__wbg_inst !== __wbg_instance_id) {\n",
        );
        assert!(strip_call_checks(&glue).is_err());
    }
}
