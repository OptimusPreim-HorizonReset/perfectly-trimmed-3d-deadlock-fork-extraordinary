//! GPDM facade (Phase 0 skeleton)
//!
//! This file contains a minimal, disabled-by-default runtime so the
//! Simulation can include a GPDM field without changing behavior when the
//! subsystem is not enabled. Full implementation follows the handoff in
//! GPDM_IMPLEMENTATION_HANDOFF.md.

pub mod ids;
pub mod host;
pub mod accounts;
pub mod flow;
pub mod participant;

#[derive(Clone, Debug)]
pub struct GpdmRuntime {
    enabled: bool,
    // test_effects is an escape hatch for unit tests to inject deterministic
    // HostEffects returned by tick(). It is intentionally private and not
    // used in production runtime implementations.
    test_effects: Option<host::HostEffects>,
}

impl GpdmRuntime {
    /// Create a disabled runtime (zero-cost placeholder for Phase 0).
    pub fn disabled() -> Self {
        Self { enabled: false, test_effects: None }
    }

    /// Construct from config (Phase 0: returns disabled runtime).
    #[allow(dead_code)]
    pub fn from_config(_cfg: &crate::config::InformationsConfig) -> Self {
        // Phase 0: keep disabled. Phase 1+ will read config values and enable logic.
        Self::disabled()
    }

    /// Testing constructor that returns a runtime which will yield the given
    /// HostEffects on the next tick and report as enabled. Useful for unit
    /// tests that exercise the host->runtime->host pipeline deterministically.
    #[allow(dead_code)]
    pub fn test_with_effects(effects: host::HostEffects) -> Self {
        Self { enabled: true, test_effects: Some(effects) }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled || self.test_effects.is_some()
    }

    /// Record an event from the host. No-op while disabled.
    pub fn record_event(&mut self, _e: host::HostEvent) {
        if self.enabled {
            // later: push into internal queue
        }
    }

    /// Tick the runtime on a snapshot and produce host effects. Phase 0: no effects.
    pub fn tick(&mut self, _frame: usize, _dt: f32) -> host::HostEffects {
        // If a test-provided effects payload exists, return it once. Otherwise
        // Phase 0: deterministic no-op
        if let Some(e) = self.test_effects.take() {
            e
        } else {
            host::HostEffects::default()
        }
    }
}
