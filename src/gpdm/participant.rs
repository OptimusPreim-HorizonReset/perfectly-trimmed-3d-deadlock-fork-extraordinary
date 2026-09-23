use ultraviolet::Vec3;

/// Minimal Participant snapshot for Phase 1 with basic lifecycle metadata.
#[derive(Clone, Debug)]
pub struct ParticipantState {
    pub participant_id: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    /// Whether this participant is considered active (not removed) on the host.
    pub active: bool,
    /// Frame when this participant was removed (if any).
    pub removed_frame: Option<usize>,
}

impl ParticipantState {
    pub fn from_body(id: u64, pos: Vec3, vel: Vec3, mass: f32) -> Self {
        Self {
            participant_id: id,
            position: pos,
            velocity: vel,
            mass,
            active: true,
            removed_frame: None,
        }
    }

    /// Mark this participant as removed at the given host frame.
    pub fn mark_removed(&mut self, frame: usize) {
        self.active = false;
        self.removed_frame = Some(frame);
        // keep last-known mass/pose but zero velocity to indicate retirement
        self.velocity = Vec3::zero();
    }
}
