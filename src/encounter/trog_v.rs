//! TROG-V: the troglodyte rules of the encounter validator
//! (`docs/01-design/floor-01-troglodyte-integration.md` section 5.5), run only for a sidecar
//! that holds a troglodyte. A failure names the rule and the ids ([`EncounterError::Trog`]).
//!
//! * **V0** a troglodyte stands on the walking line with a `trigger_x` and no perch, gate or
//!   fallback (its wake rule is the sidecar's `trigger_x`, its leash the tuning's);
//! * **V1** its leash band lies inside the play area and contains its spawn x;
//! * **V2** no static solid taller than 16 px lies inside a leash band;
//! * **V3** no part of `protected_spawn` lies within a leash band plus the forward reach
//!   (the Pygmy's leap and strike, the Basher's smash, the Virtuoso's tongue);
//! * **V4** a partition prop inside a leash band is rejected (one named by no rule is allowed
//!   elsewhere);
//! * **V5** the existing reachability checks still pass with every partition intact (the
//!   validator's own, unchanged).

use super::*;
use crate::trog::TrogTuning;

/// The tallest static solid a leash band may hold.
const MAX_SOLID_IN_LEASH_PX: i32 = 16;

pub(super) struct TrogInput<'a> {
    pub room: &'a Room,
    pub roster: &'a [RosterEntry],
    pub statics: &'a [StaticEntry],
    pub props: &'a [Prop],
    pub protected_spawn: IRect,
}

fn trog_err(rule: &'static str, ids: &[&str], reason: impl Into<String>) -> EncounterError {
    EncounterError::Trog {
        rule,
        ids: ids.iter().map(|s| (*s).to_owned()).collect(),
        reason: reason.into(),
    }
}

/// The forward reach beyond the leash of a species' attack (V3).
fn forward_reach(t: &TrogTuning, species: crate::trog::Species) -> i32 {
    let sp = t.species(species);
    let hit = i32::from(sp.hitbox.x1);
    match species {
        crate::trog::Species::Pygmy => i32::from(sp.leap_max_px) + hit,
        _ => hit,
    }
}

pub(super) fn check(inp: &TrogInput<'_>, errs: &mut Vec<EncounterError>) {
    let t = TrogTuning::active();
    let room = inp.room;
    for e in inp.roster.iter() {
        let Some(species) = e.kind.trog() else {
            continue;
        };
        let id = e.id.as_str();
        // V0
        if e.trigger_x.is_none() {
            errs.push(trog_err(
                "V0",
                &[id],
                "a troglodyte needs a trigger_x (its wake rule)",
            ));
        }
        if e.perch.is_some() || e.gate.is_some() || e.fallback.is_some() || e.mount.is_some() {
            errs.push(trog_err(
                "V0",
                &[id],
                "a troglodyte stands on the walking line: no perch, gate, fallback or mount",
            ));
        }
        let leash = i32::from(t.species(species).leash_px);
        let (l0, l1) = (e.x - leash, e.x + leash);
        // V1
        if l0 < room.play_x.0 || l1 > room.play_x.1 {
            errs.push(trog_err(
                "V1",
                &[id],
                format!(
                    "the leash band {l0}..{l1} leaves the play area {}..{}",
                    room.play_x.0, room.play_x.1
                ),
            ));
        }
        // V2
        for st in inp
            .statics
            .iter()
            .filter(|st| st.kind == StaticKind::Solid && st.rect.h > MAX_SOLID_IN_LEASH_PX)
        {
            if st.rect.x < l1 && st.rect.right() > l0 {
                errs.push(trog_err(
                    "V2",
                    &[id, st.id.as_str()],
                    format!(
                        "a static solid taller than {MAX_SOLID_IN_LEASH_PX} px lies inside the \
                         leash band {l0}..{l1}"
                    ),
                ));
            }
        }
        // V3
        let reach = forward_reach(t, species);
        let p = inp.protected_spawn;
        if p.x < l1 + reach && p.right() > l0 - reach {
            errs.push(trog_err(
                "V3",
                &[id],
                format!(
                    "protected_spawn {}..{} lies within the leash band {l0}..{l1} plus the \
                     forward reach {reach}",
                    p.x,
                    p.right()
                ),
            ));
        }
        // V4
        for prop in inp.props.iter().filter(|p| p.kind == PropKind::Partition) {
            if prop.target.x < l1 && prop.target.right() > l0 {
                errs.push(trog_err(
                    "V4",
                    &[id, prop.id.as_str()],
                    format!("a partition prop lies inside the leash band {l0}..{l1}"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests;
