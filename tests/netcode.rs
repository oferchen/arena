use netcode::message::InputFrame;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::collections::BTreeMap;

#[derive(Default)]
struct PredictionState {
    last_confirmed: u32,
    pending: Vec<InputFrame>,
}

#[test]
fn reconciliation_handles_variable_rtt() {
    let mut state = PredictionState::default();
    let mut rng = SmallRng::seed_from_u64(7);
    let mut schedule: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let total_frames = 60u32;

    for frame in 1..=total_frames {
        state.pending.push(InputFrame { frame, data: Vec::new() });
        let rtt_ms: u32 = rng.gen_range(20..=150);
        let delay_frames = (rtt_ms as f32 / (1000.0 / 60.0)).ceil() as u32;
        schedule.entry(frame + delay_frames).or_default().push(frame);
        if let Some(arrivals) = schedule.remove(&frame) {
            for confirmed in arrivals {
                state.last_confirmed = confirmed;
                state.pending.retain(|f| f.frame > confirmed);
            }
        }
    }

    for frame in (total_frames + 1)..=(total_frames + 20) {
        if let Some(arrivals) = schedule.remove(&frame) {
            for confirmed in arrivals {
                state.last_confirmed = confirmed;
                state.pending.retain(|f| f.frame > confirmed);
            }
        }
    }

    assert!(state.pending.is_empty());
    assert_eq!(state.last_confirmed, total_frames);
}
