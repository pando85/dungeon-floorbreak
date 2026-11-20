//! A deterministic fake two-slot party producer. It proves independent input
//! slots, exact action edges, reset/retry and corruption detection without the
//! game, and is the fixture PT-A's tests (and later PT-D smoke tests) share.
//! It is clearly synthetic: never a human demonstration.

use super::codec::{ActionBits, ControlSource, EventRecord, TickRecord};
use crate::input::RawInput;

/// A tiny splitmix64 (no RNG crate).
#[derive(Debug, Clone)]
pub struct FakeParty {
    seed: u64,
    state: u64,
    /// Next global tick.
    pub t: u64,
    /// Ticks since the last retry (`sim.world.tick` analogue).
    pub episode_t: u64,
    pub episode: u32,
    /// A retry (restart) every this many ticks; 0 disables retries.
    pub retry_every: u64,
    event_id: u64,
    hash: u64,
    held: [RawInput; 2],
}

impl FakeParty {
    pub fn new(seed: u64, retry_every: u64) -> Self {
        Self {
            seed,
            state: seed,
            t: 0,
            episode_t: 0,
            episode: 0,
            retry_every,
            event_id: 0,
            hash: 0xcbf2_9ce4_8422_2325,
            held: [RawInput::NONE; 2],
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn slot_input(&mut self) -> RawInput {
        let r = self.next_u64();
        // Analog axis with an arbitrary f32 bit pattern, held buttons that
        // persist across ticks (no synthetic repeat edges).
        let axis = ((r & 0xffff) as f32 / 65_535.0) * 2.0 - 1.0;
        RawInput {
            move_x: if r & 0x10000 != 0 { axis } else { 0.0 },
            jump: r & 0x20000 != 0,
            attack: r & 0x40000 != 0,
            physical: r & 0x80000 != 0,
            ability: r & 0x100000 != 0,
            pause: false,
            swap: false,
            down: r & 0x200000 != 0,
        }
    }

    /// The next pair of raw slot inputs (slot 0 sets `swap` every 90th call),
    /// for the live-path tests that feed a real simulation.
    pub fn next_slots(&mut self) -> [RawInput; 2] {
        self.t += 1;
        let mut slots = [self.slot_input(), self.slot_input()];
        slots[0].swap = self.t.is_multiple_of(90);
        slots
    }

    /// Whether the next [`Self::tick`] begins after a retry.
    pub fn retry_due(&self) -> bool {
        self.retry_every > 0 && self.t > 0 && self.t.is_multiple_of(self.retry_every)
    }

    /// Produce the next tick. Slots are independent; the routed per-hero
    /// inputs equal the slots (two-player). The call is the whole "game": the
    /// result never depends on whether a recorder is attached.
    pub fn tick(&mut self) -> TickRecord {
        if self.retry_due() {
            self.episode += 1;
            self.episode_t = 0;
            self.held = [RawInput::NONE; 2];
        }
        let pre = self.hash;
        let slots = [self.slot_input(), self.slot_input()];
        self.held = slots;
        self.episode_t += 1;
        let mut ev = Vec::new();
        let r = self.next_u64();
        if r.is_multiple_of(7) {
            ev.push(EventRecord {
                i: self.event_id,
                k: "Fake".to_owned(),
                d: format!("Fake {{ tick: {} }}", self.episode_t),
            });
            self.event_id += 1;
        }
        let bits = [
            ActionBits::from_raw(slots[0]),
            ActionBits::from_raw(slots[1]),
        ];
        for word in [
            bits[0].0,
            u32::from(bits[0].1),
            bits[1].0,
            u32::from(bits[1].1),
        ] {
            self.hash = (self.hash ^ u64::from(word)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.hash = (self.hash ^ self.episode_t).wrapping_mul(0x0000_0100_0000_01b3);
        let record = TickRecord {
            t: self.t,
            ep: self.episode,
            et: self.episode_t,
            stage: 0,
            pre,
            slots: bits,
            hero: bits,
            src: [ControlSource::HumanP1, ControlSource::HumanP2],
            ev,
            post: self.hash,
            eh: self.event_id,
            ph: 0,
            obs: None,
            eff: None,
        };
        self.t += 1;
        record
    }

    /// Logical outcome so far, to compare runs with and without a recorder.
    pub fn digest(&self) -> (u64, u64, u32) {
        (self.hash, self.event_id, self.episode)
    }
}
