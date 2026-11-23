# Hazard room first look (NON-FINAL, a bonus of the EVID-ROOM stage)

> **Predates FA-Q38 (i-frame cue) and Donut's hurt frames.** Captured before both heroes flicker during their post-hit invulnerability and before Donut's ANIM-DONUT-02 hurt, knockback and knockdown frames were wired (PR #259; see [f01-donut-hurt](../../f01-donut-hurt/README.md)). Affected here: `hazards-first-look.gif` (Carl is hurt in it). Kept as captured, not regenerated; a regeneration would show the cue.

A first look at `--scene f01_late --f01-room hazards` (the HAZ-A2b room with the HAZ-B stand-in presentation) at gameplay
scale (640 x 360), captured through the new room capture path (`scripts/room_capture.sh`, the `direct` room pilot). It is
the first deliverable HAZ-C reuses. It is **not** HAZ-D's evidence (art review, 2P, human play) and approves nothing:
the art is the HAZ-B stand-ins (`prototype_fixture_unapproved`), the pilot is not a player, and no tuning or game-feel
claim is made. HAZ-D owns the rest of `docs/evidence/f01-hazards/`.

```bash
cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
bash scripts/make_f01_hazards_first_look.sh      # xvfb-run and Pillow
```

| File | Shows |
| --- | --- |
| `hazards-first-look.gif` | Ticks 0 to 414, 6 ticks (100 ms) per frame, game speed. The `direct` pilot walks into the room, is hurt twice (ticks 41 and 100), fights the goblin (jab hits 91 and 130, defeated 130), walks under the first Rot Sticker (provoked 210, blast 234, Carl left with one heart), provokes the second (284), is hurt again and defeated at tick 296, and is back at the entrance door after the respawn (tick about 345, visible at 360). |
| `still-slime-goblin-tick048.png` | Slime and acid trail beside the entrance door, the goblin behind it, Carl's hearts down by one (hurt at tick 41). |
| `still-sticker-ring-tick216.png` | A Rot Sticker on the ceiling with its dotted orange proximity ring and a slime on the floor. |
| `still-sticker-blast-tick240.png` | The sticker's blast: an orange and red fireball with black smoke next to Carl and Donut. |
| `capture_lines.txt` | The `ROOM_CAPTURE ... match=yes` line: the window run and the headless replay end on the same state hash, so the presentation is read-only. |

What is visible (looked at by the author): the slimes read as green blobs with a green acid line on the floor; the stickers
read as dark red pods on the ceiling and the ring shows their reach; the blast reads clearly and is brighter than anything
else in the frame; Carl and Donut stay legible; the HUD hearts fall visibly. The `direct` pilot does not dodge the hazards:
in the 1500-tick headless run it is defeated three times and respawns at the checkpoint each time, so this is a look at the
room, not a clear. A pilot that plays the hazards on purpose is HAZ-C's work (`src/room_pilots.rs` takes new plans).

**NOT RUN:** any human pad session; a 2P capture; a 640 x 400 capture; a real GPU frame time.
