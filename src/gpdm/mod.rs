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
}

impl GpdmRuntime {
    /// Create a disabled runtime (zero-cost placeholder for Phase 0).
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Construct from config (Phase 0: returns disabled runtime).
    #[allow(dead_code)]
    pub fn from_config(_cfg: &crate::config::InformationsConfig) -> Self {
        // Phase 0: keep disabled. Phase 1+ will read config values and enable logic.
        Self::disabled()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record an event from the host. No-op while disabled.
    pub fn record_event(&mut self, _e: host::HostEvent) {
        if self.enabled {
            // later: push into internal queue
        }
    }

    /// Tick the runtime on a snapshot and produce host effects. Phase 0: no effects.
    pub fn tick(&mut self, _frame: usize, _dt: f32) -> host::HostEffects {
        // Phase 0: deterministic no-op
        host::HostEffects::default()
    }
}
