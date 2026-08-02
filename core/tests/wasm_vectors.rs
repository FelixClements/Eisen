#![cfg(target_arch = "wasm32")]

use eisen_core::vector_runner::run_vectors_str;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn all_vectors() {
    run_vectors_str(include_str!("vectors.json")).unwrap();
}
