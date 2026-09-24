use std::fs;
use std::path::Path;

fn read_repo_file(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

#[test]
fn host_event_supports_generic_state_change_variant() {
    let host = read_repo_file("src/gpdm/host.rs");
    assert!(host.contains("StateChanged { ids: Vec<u64>, operation: &'static str }"));
}

#[test]
fn simulation_exposes_clear_bodies_helper() {
    let simulation = read_repo_file("src/simulation.rs");
    assert!(simulation.contains("pub(crate) fn clear_bodies_with_gpdm(&mut self)"));
    assert!(simulation.contains("self.record_body_state_change(\"clear_bodies\""));
}

#[test]
fn simulation_exposes_replace_bodies_helper() {
    let simulation = read_repo_file("src/simulation.rs");
    assert!(simulation.contains("pub(crate) fn replace_bodies_with_gpdm(&mut self, bodies: Vec<Body>)"));
    assert!(simulation.contains("self.record_body_state_change(\"replace_bodies_remove\""));
    assert!(simulation.contains("self.record_body_state_change(\"replace_bodies_spawn\""));
}

#[test]
fn execution_bootstrap_uses_gpdm_replace_helper() {
    let execution = read_repo_file("src/execution.rs");
    assert!(execution.contains("simulation.replace_bodies_with_gpdm(bodies);"));
    assert!(!execution.contains("simulation.bodies = bodies;"));
}

#[test]
fn scientific_validation_bootstrap_uses_gpdm_replace_helper() {
    let validation = read_repo_file("src/scientific_validation.rs");
    assert!(validation.contains("simulation.replace_bodies_with_gpdm(bodies);"));
    assert!(!validation.contains("simulation.bodies = bodies;"));
}

#[test]
fn instrumentation_invariant_checks_clear_mutations() {
    let instrumentation = read_repo_file("tests/gpdm_instrumentation.rs");
    assert!(instrumentation.contains("trimmed.contains(\".bodies.clear(\")"));
}
