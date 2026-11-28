# F03-CIRCUS-ART-01 — Big Top, Heather and carnival visual implementation keys

**Status: original concept direction only, no approved .aseprite or in-game render.** 2026-11-22. Build alongside [F03-CIRCUS-01](../01-design/floor-03-circus-encounters-and-stagecraft.md), [F03-GRIM-01](../01-design/floor-03-grimaldi-resolution-and-coop.md), [Floor03 master art bible](floor-03-over-city-art-direction.md), [global art direction](art-direction.md) and current [pixel art routing](../07-agents/pixel-art-pipeline.md).

## 1. Why the circus must transform

At first the ruined circus is **ridiculous and dangerous**: saturated torn fabric, tiny aggressive lemurs, overly theatrical circus artillery, red-gold bulbs that fail constantly. After Heather it must feel **abandoned and wounded**; the player has seen what the performers became. Inside the Big Top the same red canvas should become oppressive around Grimaldi's enormous silhouette; after resolving the story, a truly empty/tender space frames Mongo. This is one memorable location with a changing emotional meaning, not five randomly themed rooms.

**Art hierarchy:** gameplay foreground must contain real actor bodies, 1–2 honest cover pieces, attack tell and one interactable prop at a time. All other circus noise belongs to lower-contrast scenic parallax, behind the physical combat plane. The player never loses sight of Carl/Donut under a huge decorative vine. No unreadable blinking strobe when a real hazard is imminent.

## 2. Four authored visual master keys

| Key | Story frame | Native composition | Focus in art review |
| --- | --- | --- | --- |
| F03-CIR-KEY-01 | **The Awning Collapse**, S02 B2 | Carl/Donut at normal 52/40 px scale near an actual low rigging support, huge sagging awning with two distinct silhouettes below, torn tent and crooked tenement in depth | does the player understand what was struck, what is falling, and where to move? |
| F03-CIR-KEY-02 | **Heather in the Rehearsal Pit**, S04 | a tragic unmistakable roller-skating bear, worm-infected claws, smooth low skating rail, bright one-lane warning, unblocked low route, spectators absent | bear performance/tells, humanity and 2P accessibility, no cheap gore joke |
| F03-CIR-KEY-03 | **Grimaldi: Silence in the Big Top**, S05 R3 | an **immense vine/root organism with no human-shaped boss torso needed**, three highly legible snapped wooden poles in background perspective, one exposed root reach point, ward behind | rooted living vine first; distinguish decorative giant branches from real strike volumes; visually reachable communication point |
| F03-CIR-KEY-04 | **The Empty Carnival**, S06 | same tents dark and deflated with warm refuge window and approved pink/red feathered Mongo guarding, distant noninteractive early stair silhouette | an emotional quiet end, approved Mongo growth identity, no new playable slot |

**Important visual correction:** a generated high-resolution exploratory Big Top image in the conversation may depict Grimaldi with an oversized clown-like human face and arms. This is a **mood sketch, not an approved species design**. The book describes a transformed **Pestiferous Vine**. The production master must prioritize creeping rooted vegetal anatomy and preserve the tragic former-ringmaster story by garments/props, not simply draw a person on a giant body. Owner/independent editor chooses final representation after original source review.

## 3. Cues are more important than background detail

### S02: support hit and falling awning
- Idle awning: striped cream/red, visible two central support lines.
- Support damaged: a **single bright diagonal tear**, tension line pulled taut; avoid multi-frame visual clutter.
- Warning: support falls into a known rectangle painted on ground, distinct red from string bulbs. Both heroes see it.
- Impact: physically hit actor launched/flattened according to **actual simulation**, not VFX-derived death. Giant moving cloth front layer briefly clips down to hit plane, then becomes static underlay. Physical exit remains open.
- Aftermath: broken poles create a different silhouette visible at real game speed; rare exceptional skill death effect is conditional on actual gameplay, not automatic.

### S03: lions and tattoo army
- Charger vs heavy-swipe silhouettes must differ **before a single attack**: compressed horizontal shoulders and visible rear-leg brace vs upright tall paw.
- Signet's flat tattoo army is rendered as a **two-dimensional dark graphic plane** against old murals, never as a third independent collision body unless a separately approved actor has authority. It may cross behind heroes and should not obscure red tell wedges.
- A real temporary ink blocking curtain is only included if data/hazard ownership can distinguish it from scenic ink. First playable should work without it.

### S04: Heather
- One charged skating foot really needs to be visible; roller skates are not comic decoration. She enters with a rolling/wobbling mass rather than the same generic four-legged gait as mold lions.
- The parasite-covered claws have their own anticipatory motion; direct claw hitbox stays tied to an explicit attack. **No damage from ambient worm movement.**
- Final injury and mercy should be somber; no ordinary glamorous blood finish frame or mandatory slow-mo after killing a suffering performer.
- Preserve both heroes' high-contrast pose silhouettes in every shot, including a giant bear charge tell.

### S05: Grimaldi
- **Atmospheric storytelling from composition**: crushing red roof, three crooked wooden supports, a small pale spotlight on the contact point, larger audience benches empty. Ringmaster traces are one old hat/banner, not a face pasted onto an enormous superhero with fingers.
- Three first key poses: calm root/coiled loom, **committed directional strike** with clear red border, **listening/root open** with an approachable teal safe contact ring. Optionally a fourth reunion pose where the enormous vine becomes still as Signet stands at the tent entrance.
- Narrative root transparency/communication effect must never hide hazard tell or fake a player-created kill.
- The ward blocks Signet's black planar silhouettes, not Carl/Donut when they have story entry; the visual effect cannot affect real movement if story state says they can enter.
- No Grimaldi HP bar, no 3/3 generic break-the-posts HUD counter, no explosive all-room death effect.

### S06: Mongo
- Use approved **pink/red feathered raptor** identity and canonical growth stage; don't enlarge him to adult 76px just because art for that exists. One guarding key, one wounded/tired key and a quiet recovery gesture.
- Native pixel art must remain readable, not downsampled from a high-resolution illustration. Keep Carl and Donut identities untouched.

## 4. Audio/camera staging (proposal)

- Fairground: bad original out-of-tune calliope, cheap metal cymbals, knife whoosh; combat sound remains dominant.
- Mold plaza: rhythmic parade percussion slowly becomes dissonant when Signet's flat army crosses.
- Heather: music contracts; audible wheels against stone, two different claw tells, post-defeat silence.
- Big Top: enormous but **sparse** chord clusters with wet root/wood resonance; a separate rising attack-specific sound for slam/sweep/pull. When listening point appears, remove percussion and introduce one single new original melodic interval.
- Reunion: restrained quiet, no triumphal boss-death jingle.
- Mongo: soft footsteps and one reassuring original tone before movement returns.

Every first reveal and postboss pause is skippable/retry-safe, does not decide physics and cannot hide a live attack. Camera never zooms out to fit the boss's entire body; reserve negative space behind playable heroes and show one **partial** living creature to imply colossal scale.

## 5. Asset pipeline and proof

**ART-P1** original full-resolution mood key; **ART-P2** 1x silhouette proposal + actual accepted Carl/Donut composites; **ART-P3** 2–3 honest anticipatory/recovery keys, frozen after independent reviewer; **ART-P4** editable native sprite source, tagged animation, pivots, atlas, provenance and hashes; **ART-P5** actual in-engine 640×360/400 still and game-speed VFX/recovery; **ART-P6** second independent visual reviewer + owner game feel later.

The high-resolution generated conversation imagery is reference-only. Files need explicit generation/creation history, creator/tool, license/rights status, no third-party artwork, game-size crops and distinct accepted/rejected status. Do not commit the generated image as a production sprite or replace approved hero sources with AI visual approximations. Only actual repository source/provenance can authorize a new asset.

**Art acceptance:**
- CIR-ART-01: the Big Top reads as a **vine** before it reads as a generic clown boss;
- CIR-ART-02: Heather is visually empathetic and a genuine bear/skater, with two clear attack tells;
- CIR-ART-03: falling awning warning rectangle and true collision obvious at 1x;
- CIR-ART-04: every new attack is legible at 640×360/400 while both heroes remain full scale;
- CIR-ART-05: background ink/worm/ward art never deals hidden damage;
- CIR-ART-06: all source masters original, editable, validated, rights-manifested and independently reviewed;
- CIR-ART-07: performance budgets for giant canvases/crowds parallax, and actual render capture/owner game-feel **NOT RUN**.
