//! COMPANION-AI-01 replay fixtures (CF02-00: an empty registry).
//!
//! The registry has the shape of [`crate::donut_fixtures::DONUT_FIXTURES`] and
//! is chained into [`PartyFixture::all`]. The `companion_*` and `cf02_assist_*`
//! fixtures land here with COMPANION-AI-01; `--list-fixtures companion_` and the
//! CI loop over it never need another edit.

use crate::party_scene::PartyFixture;

pub const COMPANION_FIXTURES: [&PartyFixture; 0] = [];
