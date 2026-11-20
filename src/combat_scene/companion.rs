//! COMPANION-AI-01 hooks in the combat scene (CF02-00: no-op stubs).
//!
//! Every function here is the identity or does nothing, so no hash and no
//! behaviour changes. 01A-wire fills them in; COMPANION never edits
//! `combat_scene.rs` again (`docs/07-agents/combat-fun-02-workorder.md`,
//! "CF02-00 COMPANION stubs (exact)").

use super::CombatState;
use crate::combat::MoveSpec;
use crate::corridor::SceneSim;
use crate::input::RawInput;
use crate::party::HeroId;

#[derive(Debug, Clone, Default)]
pub(crate) struct CompanionRuntime {}

impl CompanionRuntime {
    /// Attempt restart: no-op.
    pub(crate) fn reset(&mut self) {}

    /// Writes nothing while no hero is companion-driven, so every hash is unchanged; when 01A-wire mixes,
    /// it writes a marker word first (as the party and boss blocks do).
    pub(crate) fn mix_hash(&self, _mix: &mut impl FnMut(u64)) {}

    /// The glance rule for hits dealt by `hero` in move `instance` (stub: identity). CA-20 keys it on whether
    /// that move instance was started by an injected press, and the flag survives a swap.
    pub(crate) fn strike_glance(&self, _hero: HeroId, _instance: u32) -> Glance {
        Glance::default()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Glance {
    ai_started: bool, // false in the stub
}

impl Glance {
    pub(crate) fn apply(&self, spec: &MoveSpec, target_hp: u8) -> MoveSpec {
        glance_hook(spec, target_hp, self.ai_started)
    }
}

/// Stub: returns `*spec` unchanged.
pub(crate) fn glance_hook(spec: &MoveSpec, _target_hp: u8, _ai_started: bool) -> MoveSpec {
    *spec
}

impl CombatState {
    /// Stub: identity. `inputs` is `[Carl, Donut]` as in `step_party`.
    pub(crate) fn companion_inputs(
        &mut self,
        _scene: &SceneSim,
        inputs: [RawInput; 2],
    ) -> [RawInput; 2] {
        inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK};
    use crate::combat_scene::{FIXTURES, FixtureSim};
    use crate::env_kit::EnvKit;
    use std::path::PathBuf;

    #[test]
    fn companion_stub_parity_mix_hash_writes_nothing() {
        let mut runtime = CompanionRuntime::default();
        let mut calls = 0u32;
        runtime.mix_hash(&mut |_| calls += 1);
        runtime.reset();
        runtime.mix_hash(&mut |_| calls += 1);
        assert_eq!(calls, 0, "the stub mixes nothing into any hash");
    }

    #[test]
    fn companion_stub_parity_glance_hook_is_identity() {
        for spec in [CARL_JAB, CARL_HAYMAKER, CARL_PUSH_KICK] {
            for hp in [1, u8::MAX] {
                for ai_started in [false, true] {
                    assert_eq!(glance_hook(&spec, hp, ai_started), spec);
                }
                assert_eq!(Glance::default().apply(&spec, hp), spec);
            }
        }
        let glance = CompanionRuntime::default().strike_glance(HeroId::Carl, 7);
        assert_eq!(glance.apply(&CARL_JAB, 1), CARL_JAB);
    }

    #[test]
    fn companion_stub_parity_inputs_pass_through() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = FixtureSim::new(&kit, 360, FIXTURES[0]).unwrap();
        let samples = [
            [RawInput::NONE, RawInput::NONE],
            [
                RawInput {
                    move_x: 0.6,
                    attack: true,
                    ..RawInput::NONE
                },
                RawInput {
                    physical: true,
                    jump: true,
                    swap: true,
                    ..RawInput::NONE
                },
            ],
        ];
        for inputs in samples {
            assert_eq!(sim.combat.companion_inputs(&sim.scene, inputs), inputs);
        }
        // Restarting calls the stub reset and leaves the state hash alone.
        let before = sim.combat.state_hash(&sim.scene);
        sim.combat.companion.reset();
        assert_eq!(sim.combat.state_hash(&sim.scene), before);
    }
}
