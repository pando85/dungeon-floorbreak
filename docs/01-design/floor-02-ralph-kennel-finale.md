# F02-RALPH-01 — The Tiny Terror: kennel, boss, and Mongo finale

**Status:** detailed design proposal, not implemented gameplay or final art. This refines S08–S09 of the [Floor 02 bible](floor-02-last-lesson-design-bible.md). The player should **laugh when Ralph appears, and respect him ten seconds later**. The name, small Frenzied Gerbil species and after-boss Mongo reward are inspired by the novel; our move set, arena and optional dingo interaction are gameplay adaptations. No recognizable third-party images, music or dialogue.

## 1. The boss-reversal contract

Floor 01 ends with a 128px rolling Ball of Swine. Mid-Floor 02 has huge rooted Krakaren. Rage dwarfs both and cannot be defeated through HP. A classic escalation design would now make the final boss even bigger. **Do the opposite:** the announcement implies a terrifying champion, the steel cage opens, and a small angry gerbil runs out.

Tiny MUST NOT mean unfair. The visual read carries the scale of the attack: floor scratches, foreground dust, sparks on cage rails, a spotlight tracking the animal, and an unmistakable directional windup. It should stay funny even if the player never reads a word.

## 2. Stage S08 kennel — make the choice emerge from physical combat

**Provisional width 1800–2100px**, three connected small arenas with unique silhouettes, one clear rightward route; no new dog-taming inventory, no stationary museum room. A working black-metal gig-stage above a functional kobold kennel, with wholly invented distressed iconography and original guitar/percussion texture rather than copied music. Background silhouettes suggest more cages than gameplay actually simulates.

**Three portions:**
- Entrance: one mounted kobold with a readable spear-charge and one unmounted keeper. Teaches a visible mounted silhouette and dismount after a real hit/launch, not a cinematic auto-knockoff.
- Kennel latch: 1–2 active enemies plus an obvious cage mechanism. A normal R1/RB interact from a safe envelope can unlock a **single room-local ally flag**, enabling up to two bounded neutral/helpful dingoes. Optional. It should not trigger when the player merely walks past.
- Ring entrance: a suspended speaker that can fall after an actual rider/body impact; one big payoff and a short boss checkpoint. All cages remain optional.

**Player routes:**
1. **Direct**: fight through both encounters, do not touch cages or speaker.
2. **Physical**: Carl punts mounted rider off dingo; Donut finishes or repositions; launched rider hits an actual speaker anchor -> speaker drops on enemy (not simply on everyone because the cutscene fired).
3. **Optional kindness**: open the kennel first, dogs change local target/flee policy and become a bounded positional advantage. No required animal injury; freeing dogs is not a damage exploit.

**Mount/dismount policy:** kobold rider and danger dingo are separate authoritative entities with a short attachment state, explicit collision ownership and dismount event. Detached mount cannot suddenly disappear, continue carrying an invisible rider or take credit for rider's attacks. A dismounted rider is a normal vulnerable foe. Never alter protagonists' controller bindings to implement enemy mounts.

## 3. Stage S09 Ralph ring geometry

Provisional **1344–1600px**, shorter and more focused than the giant Ball of Swine arena. Keep nearly all fighting on an uninterrupted ground line. Two **low 32–48px** step-up shelves may allow different Donut firing angles and are never permanent safe zones. A hanging loudspeaker and one solid low wall/rail make committed charges predictable and offer knock-on collisions. Both remain optional.

**Hero visibility:** native Carl 52 opaque pixels and Donut 40 remain unchanged. Ralph starts with a **proposed 16–24px recognizably gerbil-shaped source silhouette**, not a large fantasy beast. Actual physical hurtbox is close to his visible physical body; the authored attack *tell* can be visually 3x broader (dust streak, scrape line) without enlarging an invisible damaging region. If Carl's kick collision cannot hit a small floor-level target with existing mechanics, change **Ralph's arena/objective posture or explicit boss collision policy** rather than globally modifying melee hitboxes without baseline approval. Donut's physical pounce and earned Magic Missile also need real contact/LOS proof.

**Entrance keyframes:** theatrical ring lights -> huge gate shaking -> smaller hatch slides open -> one lonely tiny shadow -> immediate single floor scratch -> player controls return. On retry: skip or shorten entrance drastically, no forced 15-second comedy rerun.

## 4. Boss design: committed speed, never random speed

**Core emotional rhythm:** confusion -> dodging the tiny terror -> "I can make him crash into that" -> satisfying honest physical kill.

| State | Read | Player answer | Failure mode disallowed |
| --- | --- | --- | --- |
| CageOpen | tiny dot + spotlight, no damage | read and position | immediate unavoidable contact |
| ThreatDisplay | forepaws planted, clear left/right scratch, no attack | set up spacing, prepare kick/pounce | hidden direction change |
| ChargeCommitted | fast low straight-line charge, direction frozen | jump/avoid/brief blink if earned | homing through player after commitment |
| WallBonk | Ralph actually hits a solid rail at speed; brief stagger | physical or magic punish | fake wall impact with no collision |
| TwoBeatFeint | 1 short feint, 1 new visible committed line; distinct cues | read second line then punish | random zigzag without warning |
| Regroup | Ralph visibly stops near an anchor | select flank/position | invulnerability between every attack |
| OptionalDingoMoment | if kennel flag; a dog interrupts him briefly and he re-emerges | bonus punish; player still finishes | dog automatically wins boss |
| Defeated | hit from either character, real causal event | earned ending | forcing Carl to last-hit |

**Phases are tempo, not three HP bars.** Phase A introduces straight rush and wall impact; phase B adds the two-beat feint and reposition; phase C creates a short intense final mixed sequence. Resist adding enemies during boss phase, hazards everywhere, or a random teleport "because he's tiny." One visible primary attack commitment at a time, adequate punish windows from the *accepted Floor 01 feel baseline*. Do not make players wait through 5 identical charge cycles if attacks are landed cleanly.

**Optional dingo scene:** if kennel flag exists and at least one helper is alive, a helper may swallow/interrupt Ralph, then immediately spit him back alive and vulnerable for a short window, preserving original dark absurdity. **If no dog**, Ralph instead bonks the steel rail and yields an equivalent recoverable window. The helper's contribution is bounded and never grants a finishing blow, 2x viewers or required survival advantage.

## 5. Carl, Donut, companion and real co-op

- **Carl**: floor-level kicks and appropriately timed charged strikes give strong physical response; a committed sprinting gerbil can be sent flying into an authored railing only if real body impulse/collision proves it. Earned dynamite is viable but must have fair self-damage/telegraph; avoid "one bomb instantly wins".
- **Donut**: physical pounce can punish recovery and use her superior mobility. Magic Missile can connect if real line of sight/hurtbox allows; do not secretly auto-aim at a huge invisible proxy. Her optional blink helps reposition but is not a requirement.
- **Solo Carl / solo Donut**: the inactive partner does not steal every punish window, misread tiny targets or simply stand in boss charge. Need separated scorecard counts for attacks, evades and genuine hits.
- **2P**: neither player is parked on a roof; both have at least one consistent close-range opening on every phase. Either may receive the finishing blow and attributable viewers.
- **Camera**: no dramatic zoom out to show the entire arena: it would erase Ralph. Show arena architecture with a **short controlled establishing view**, then normal legible gameplay coverage with attack warning tracking; camera movement never changes authoritative collision.

## 6. Quiet epilogue: Mongo, not a third combat feature

After Ralph's actual defeat and safe checkpoint, hard-cut the crowd noise to an almost empty gentle room; little Mongo is nervous, then responds to Donut. Two small original animation keys (wary -> bond), one story-grant event and the way down. The pet should be recognizable using existing approved Mongo references, but its native source art must be produced with established provenance and independent art review.

Do NOT add: Mongo ride controls, third player, new companion attack policy, race/class selection, pet inventory, mandatory bond minigame or a second boss after the emotional comedown. Book-event timing must be checked before labeling individual text as canon.

## 7. Acceptance tests and evidence

| ID | Proof |
| --- | --- |
| RLP-01 | kennel clear with all cages closed; no prop required |
| RLP-02 | rider dismount: entity ownership and authoritative contact correct; retry resets |
| RLP-03 | optional freed-dingo effect changes finite target policy, not globals |
| RLP-04 | falling speaker is causally triggered by physical collision and lands exactly once |
| RLP-05 | game-speed legibility of small Ralph and two-beat direction at 640x360 and 640x400 |
| RLP-06 | Carl direct-only normal attacks can hit and defeat Ralph in all phases |
| RLP-07 | Donut direct-only physical/magic attacks can hit and defeat Ralph in all phases |
| RLP-08 | large visual tell does NOT enlarge damaging collision or invisible boss hurtbox |
| RLP-09 | no optional dog -> predictable bonk vulnerability, complete boss path |
| RLP-10 | optional dog -> short interrupt, Ralph survives, human hero finishes |
| RLP-11 | same seed retry produces same boss states and authoritative events, strict replay |
| RLP-12 | solo companion avoids charge and contributes real offensive events |
| RLP-13 | story ledger does not spawn Mongo until Ralph defeated and epilogue entered |
| RLP-14 | Mongo cannot be controlled/used to fight; Floor 3 classes not granted |
| RLP-15 | two real pads, owner game feel and Deck/real-hardware perf: **NOT RUN** |

## 8. Build and art order

Pure mounted/dismount state, then S08 direct+optional routes, then **Ralph collision/readability prototype before detailed art**. A poor tiny hitbox is the highest risk and must be tested in Bevy while the body is an original greybox placeholder. Next original keys for giant empty ring -> tiny Ralph -> two-beat attack -> small Mongo, then native masters, in-engine capture, independent top-tier review and owner fun test.

**No late surprise overengineering.** The boss is a short kinetic duel, not a new generic class of minuscule AI agents. The original physical joke and direct fighter agency matter more than an elaborate status-effect system.
