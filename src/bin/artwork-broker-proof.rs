//! Separate temporary operator-started Identify proof; never an installed service.
fn main() -> Result<(), String> {
    native_home::broker_server::main_entry(&std::env::args().skip(1).collect::<Vec<_>>())
}
