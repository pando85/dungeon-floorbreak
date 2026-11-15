# Dungeon Floorbreak — Design & AI Development Repository

> **Status: early technical spike. No playable game or finished game assets are included.**
>
> **Public, unlicensed fan-project concept.** Dungeon Crawler Carl and the characters Carl, Princess Donut, and Mongo are referenced for noncommercial design exploration. This project is **not affiliated with, endorsed by, or licensed by** their rightsholders. **Do not distribute a game, character artwork, dialogue, or other derivative content without the necessary rights.** An original-IP path is defined in [rights and release](docs/04-production/rights-and-release.md).

A Linux-first, controller-first, side-scrolling action platformer design: expressive, relatively detailed **dark-fantasy pixel art**, explosive comedy, character synergy, and **selective, gameplay-significant level destruction**. Inspirations include Broforce (pacing and destructibility) and Mewgenics (expressive modular animation), not replication of either game's assets or systems.

## Decisions at a glance

| Subject | Direction | Status |
| --- | --- | --- |
| Development method | AI agents implement **all** code/art with human direction and playtesting | **Decided** |
| Primary platform | Native Linux desktop; Steam Deck controller support | **Decided** |
| Distribution | Steam later; Windows build after Linux | **Target, not shipping commitment** |
| Game engine | Rust + Bevy 0.20.x | **Provisional / benchmark against Godot** |
| Art | Detailed, dark, expressive pixel art with selective modular animation | **Decided direction** |
| Terrain | Tile/material-driven selective destruction; no pixel-fluid simulation | **Provisional** |
| Players | Solo + local two-player cooperative, one camera; remote via Steam Remote Play Together later | **Design target** |
| Characters | Carl and Donut from initial slice; Mongo joins later | **Fictional design, rights required to release** |
| Level tools | LDtk; exported JSON; content validation | **Provisional** |

## Start here (for humans and LLMs)

1. [AGENTS.md](AGENTS.md) — mandatory entry point for all local agent sessions.
2. [Product vision and principles](docs/01-design/vision.md).
3. [Core loop, combat and cooperative play](docs/01-design/gameplay.md).
4. [Character ability sheets](docs/01-design/characters.md).
5. [Art direction](docs/02-art/art-direction.md) and [animation pipeline](docs/02-art/animation.md).
6. [Architecture](docs/03-tech/architecture.md) and [destruction specification](docs/03-tech/destruction.md).
7. [Vertical-slice definition](docs/04-production/vertical-slice.md), [90-day plan](docs/04-production/roadmap.md), and [backlog](docs/04-production/backlog.md).
8. [Decisions](docs/05-decisions/) and [source register](docs/06-research/sources.md).

## What this repository is not

- There is no claim that AI-generated art is automatically production quality; it requires visual review and provenance.
- There is **no guarantee** of Steam approval, Steam Deck Verified status, 60 fps, or legal permission to ship franchise material.
- There is no promise of online rollback/netcode in v1.
- Specifications and numerical targets are *initial hypotheses*, not validated results.

## Current next action

Implement two short technical spikes before full production: (1) controller + pixel-camera + two-character movement and (2) destructible arena + collision update. Compare the time/quality of Bevy against a small Godot reference if needed. **Do not generate a full campaign or lots of character art yet.**

## Local checks

```sh
python3 scripts/check_docs.py
```

## Publishing this repository

This repository is publicly hosted at [pando85/dungeon-floorbreak](https://github.com/pando85/dungeon-floorbreak). To fork or re-publish elsewhere using [GitHub CLI](https://cli.github.com/):

```sh
cd dungeon-floorbreak
bash scripts/publish_public.sh
```

That script creates **`pando85/dungeon-floorbreak` as a public GitHub repository** and pushes the documentation. Inspect files and confirm there are no secrets before executing. Adjust `REPO_NAME` in the script if needed. The original public repository already exists; do not run the publishing script against the existing remote.

## Licensing and rights

No software or content license has been granted for the repository itself at this stage (all rights reserved by default). Independent contributions and assets require explicit provenance and licensing decisions. See [rights and release](docs/04-production/rights-and-release.md) and [NOTICE](NOTICE.md).
