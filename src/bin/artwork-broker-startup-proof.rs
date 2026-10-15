//! Fixed installed Proof helper; no command-line operation or caller parameters.
fn main() -> Result<(), String> {
    native_home::broker_bootstrap::main_entry(&std::env::args().skip(1).collect::<Vec<_>>())
}
