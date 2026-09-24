use ultraviolet::Vec3;

/// Minimal host DTOs for Phase 0. These are intentionally small and
/// use primitive types to decouple GPDM from the host internals.

#[derive(Clone, Debug)]
pub struct HostBody {
    pub id: u64,
    pub pos: Vec3,
    pub vel: Vec3,
    pub mass: f32,
}

#[derive(Clone, Debug)]
pub struct HostSnapshot {
    pub frame: usize,
    pub dt: f32,
    pub bodies: Vec<HostBody>,
}

#[derive(Clone, Debug)]
pub enum HostEvent {
    Spawned { id: u64 },
    Merged { survivor: u64, removed: u64 },
    GasIgnited { id: u64 },
    Removed { id: u64 },
    StateChanged { ids: Vec<u64>, operation: &'static str },
    ConfigReloaded,
}

#[derive(Clone, Debug, Default)]
pub struct VelocityImpulse {
    pub participant_id: u64,
    pub impulse: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct GasHeat {
    pub participant_id: u64,
    pub heat: f64,
}

#[derive(Clone, Debug, Default)]
pub struct MassTransfer {
    pub from: u64,
    pub to: u64,
    pub mass: f64,
}

#[derive(Clone, Debug, Default)]
pub struct HostEffects {
    pub velocity_impulses: Vec<VelocityImpulse>,
    pub gas_heat: Vec<GasHeat>,
    pub mass_transfers: Vec<MassTransfer>,
}

impl HostEffects {
    /// Convenience: empty effects
    pub fn empty() -> Self {
        Self::default()
    }
}
