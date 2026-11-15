# ADR-0004: Public concept repository; no rights assumed

- **Status:** Accepted safety constraint
- **Date:** 2026-11-15

## Context

The concept names characters from a copyrighted book series, and a Steam launch is desired. Public visibility can mislead collaborators into believing the derivative game is authorized or assets are reusable. The project has **no evidenced IP license**.

## Decision

Publish only original design/engineering documentation, simple generic code/data fixtures and independently licensed assets, with disclaimers. Keep franchise names in speculative fan-concept **documentation** only and use generic runtime IDs. Do not publish or distribute franchise-specific game builds without rights. Fan-concept character artwork (concept and production) is published under the owner-reported `ART-PUBLICATION-ALL-V1` scope (2026-11-15, amended after this decision); that scope is user-reported, not verified consent, and excludes game, Steam, trailer, merchandise and standalone asset-pack distribution. Require original-IP conversion or license before release.

## Consequences

This public repo can support engine development and planning, but is not a grant of adaptation or commercial rights. Current docs are not openly licensed by default; decide explicit contribution and asset licenses before inviting outside contributors. See [rights](../04-production/rights-and-release.md).
