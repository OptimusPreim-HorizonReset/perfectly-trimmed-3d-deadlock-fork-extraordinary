use ultraviolet::Vec3;

/// Minimal Participant snapshot for Phase 1.
#[derive(Clone, Debug)]
pub struct ParticipantState {
    pub participant_id: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

impl ParticipantState {
    pub fn from_body(id: u64, pos: Vec3, vel: Vec3, mass: f32) -> Self {
        Self {
            participant_id: id,
            position: pos,
            velocity: vel,
            mass,
        }
    }
}
