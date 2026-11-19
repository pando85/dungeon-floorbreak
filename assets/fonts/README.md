# HUD fonts (HUD-HYBRID-A-01)

Both fonts are licensed under the **SIL Open Font License 1.1** and declare **no Reserved Font Name** (checked in each `OFL.txt` on 2026-11-19: the copyright line carries no "with Reserved Font Name" clause). They are used unmodified, as TTF files loaded at runtime by the HUD adapter. The licence text is committed next to each font and must stay with it.

| Font | Files | Role | Version (name table) | Licence | Primary source |
| --- | --- | --- | --- | --- | --- |
| Jersey 10 (Soft Type Project, Sarah Cadigan-Fried) | `jersey10/Jersey10-Regular.ttf`, `jersey10/OFL.txt` | display: names, location, notice headline, boss name | Version 1.001 | OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/jersey10 (upstream https://github.com/scfried/soft-type-jersey) |
| Silkscreen Regular and Bold (Jason Kottke, Silkscreen Project) | `silkscreen/Silkscreen-Regular.ttf`, `silkscreen/Silkscreen-Bold.ttf`, `silkscreen/OFL.txt` | labels: chips, tags, `SYSTEM NOTICE`, PREVIEW, `COMPANION` | Version 1.001 | OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/silkscreen (upstream https://github.com/googlefonts/silkscreen) |

Files were downloaded on 2026-11-19 from `https://raw.githubusercontent.com/google/fonts/main/ofl/<family>/<file>` (google/fonts directory last changed for Jersey 10 at commit `6e4b84c976ca`, 2026-03-03). SHA-256, also enforced by [SHA256SUMS](SHA256SUMS):

| File | SHA-256 |
| --- | --- |
| `jersey10/Jersey10-Regular.ttf` | `db9cbd091617048a145d249daa2b815fe7083be6ab66ac26626e21a4e01c3e82` |
| `jersey10/OFL.txt` | `436d5836064cec796c7ddde6fb4faeabdb6f2f17443a24a30d0c38b75d13f922` |
| `silkscreen/Silkscreen-Regular.ttf` | `c845473330b94c2079ce9af01c51ac8ba2d99c24f4d14c039843bbb8e642ebd8` |
| `silkscreen/Silkscreen-Bold.ttf` | `768476aa712d4f5c3e18d3bce80f980a8bd3f72b7094d22ec5e768df3acfed61` |
| `silkscreen/OFL.txt` | `86c5e9c9382cdcc5948704fdfe60f2aa164a719746931219a42736ecd9cefbd3` |

Pixel-grid sizes (see [the native HUD spec](../../docs/02-art/hybrid-hud-a-native-spec.md), section 4): Jersey 10 at `FontSize::Px(1400 / 75)` for 10 px caps, Silkscreen at `FontSize::Px(8.0)` for 5 px caps. The OFL permits use, embedding and redistribution with the font; the fonts are not sold on their own and are not renamed.
