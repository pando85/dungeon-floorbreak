# HUD-TRY-02: `TRY n` inside the boss strip

**Status:** implemented, game-feel review pending. Display-only: no state-hash, golden or `MANIFEST_HASH` change.

## Behavior

HUD-TRY-01 (#293) put `TRY n` in the location strip, but the boss strip replaces the location strip in the Juicer's
room (15) and the Ball of Swine's room (18), so the count was invisible there. **HUDTRY-AD2:** the boss strip now draws
`TRY n` itself (existing Jersey font, `amber_dim`, `amber` in high contrast), right-aligned on the boss-name line and
ending at the bar's right edge. The count is its own field (`BossView::attempt`, `Some` only in a room), not parsed
from the location text. **HUDTRY-AD3:** if the name and `TRY n` would overlap at 640x360, `TRY n` is dropped, never the
name; both current names fit with `TRY 9` (unit test). No new art.

## Evidence

Window stills, 640x360, `--scale 1`, `scripts/room_capture.sh` twice per still (byte-identical, window hash = headless
hash, so the presentation is read-only).

| File | Room, pilot, tick | HUD |
| --- | --- | --- |
| `bos_try1_640x360.png` | `--f01-room bos`, `bos_kick`, 238 | `THE BALL OF SWINE` ... `TRY 1` |
| `juicer_try1_640x360.png` | `--f01-room juicer`, `juicer`, 300 | `THE JUICER` ... `TRY 1` (the warning icon sits right of it) |

`bos_try1` is the same tick and pilot as `docs/evidence/bos-b/scene_tell_640x360.png` and the same state hash; the
two differ only inside the boss strip (changed pixels bounded by a 31x10 box at x 411, y 12).

## Stills changed

Any window still that shows the boss strip in a room (rooms 15 and 18) differs from a fresh capture only inside the
strip's name line (the added `TRY n`). The committed earlier stills (`docs/evidence/bos-a`, `bos-b`, `boss-f01`, the
PT-GATE and room evidence of the two boss rooms) are not regenerated and keep showing the strip without `TRY n`; no
script compares them byte for byte. Only the two stills above are new.
