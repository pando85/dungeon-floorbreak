# DEMO-01 playtest questionnaire

**Read this before you play, answer it right after.** It takes about 5 minutes. Play the [DEMO-01 route](demo-01-plan.md#b-the-demo-route) once from start to end (about 3 to 5 minutes), at 1920x1080 if you can (`--demo --scale 3`): the camera only zooms there or at 4K.

**How to answer.** Scales are 1 to 5. On a **"too little / right / too much"** scale, 3 means "keep it". Write `skip` for anything the build did not show (the question says when). One word or a short line is enough for the free-text questions. Copy the answer block at the end and fill it in.

**What to watch for while playing** (no need to take notes): how a basic goblin kill feels; how far the kick sends a goblin and what happens when it hits another one; whether you want to hit goblins that are lying down; the rotten shore and the block; how much gore stays on screen; the decapitation and the explosion; the llama's spit and how it dies; the camera when the llama appears; the HUD; Donut as a companion and the swap to her (L1/LB, Q on the keyboard); the overall tone.

## Questions

1. **Movement.** Running, jumping and landing with Carl. 1 sluggish or floaty, 5 tight and responsive.
2. **Basic kill.** Jab, then haymaker on a goblin (hit-stop, stagger, the goblin's reaction). 1 weightless, 5 heavy and worth repeating.
3. **Kick.**
   a. Distance a kicked goblin flies: 1 far too short, 3 right, 5 far too long.
   b. A kicked goblin knocking another one down (contact push): 1 unreadable or limp, 5 clear and fun.
4. **Downed goblins.** Right now a lying goblin can be hit (a jab stands it up, a kick launches it again). Keep it? `yes` / `no` / `kick only`.
5. **Respawn and restart.** Getting up at the last checkpoint after a defeat, Select tap to respawn, Select hold to restart the demo. 1 annoying or confusing, 5 quick and clear.
6. **Rotten shore and block.**
   a. Setting it up and dropping the block on goblins: 1 fiddly or unclear, 5 satisfying.
   b. Should the falling block also hurt Carl if he stands under it? `yes` / `no`.
7. **Gore at preset 4.**
   a. Intensity: 1 too tame, 3 right, 5 too much.
   b. Readability (you can tell what killed whom): 1 a mess, 5 always clear.
   c. Clutter after a fight (at most 8 bodies stay, the oldest fade; stains never fade): 1 too cluttered, 3 right, 5 too clean.
8. **Decapitation and explosion.** Did you see each? (`decap yes/no`, `explosion yes/no`.) How special did they feel in this run: 1 too rare or I missed them, 3 right, 5 too common and cheapened. `skip` if neither appeared.
9. **Bad Llama.**
   a. The spit tell is readable and the spit is fair to dodge: 1 no, 5 yes.
   b. A lethal kick throws it on its back (slam), a punch kill drops it plainly. Does the difference read and feel earned? `yes` / `no` / `didn't notice`. `skip` both if the llama was not in the build.
10. **Camera and HUD.**
    a. Zooming out when the llama takes aim and back in afterwards (the eased transitions): 1 rough or distracting, 5 smooth and helpful. `skip` at 1280x800 or if the camera never zoomed.
    b. HUD readability (health, notices like `LAUNCHED!` and `FLATTENED!`): 1 hard to read or in the way, 5 clear at a glance. `skip` if there was no HUD.
11. **Tone.** Black humour and 1980s to 1990s action-movie attitude (a battered everyman, dirty impacts, the mess as the joke): 1 absent, 5 nailed. Then one line: what is missing or wrong?
12. **Donut and the party.**
    a. Solo swap to Donut and back (L1/LB, Q on the keyboard): 1 clunky or confusing, 5 instant and useful. `skip` if you never swapped.
    b. Donut as companion (she follows, dodges, can be hurt, and her defeat ends the attempt; she has no attacks yet): 1 a liability, 5 a partner worth protecting.
    c. Two players (a second pad joins with Start): 1 broken or no fun, 5 fun. `skip` if you played solo.
13. **Priorities.** One line each:
    a. Cut or tone down first:
    b. Amplify first:
    c. Anything broken or confusing (optional):

## Answer block

```text
DEMO-01 answers (date, build or commit, output size):
Q1:
Q2:
Q3a:     Q3b:
Q4:
Q5:
Q6a:     Q6b:
Q7a:     Q7b:     Q7c:
Q8:  decap yes/no, explosion yes/no, scale:
Q9a:     Q9b:
Q10a:    Q10b:
Q11:     line:
Q12a:    Q12b:    Q12c:
Q13a:
Q13b:
Q13c:
```

The answers feed the game-feel reviews still open on DEATH-A (kick distance, contact push, downed hits, Select restart, block damage), DEATH-C and DEATH-E1 (gore intensity, corpse cap, stain persistence), DEATH-D (rarity), LLAMA-02-DEATH (slam rule), the camera lab, the HUD and the party (WORK-005, WORK-006: swap, Donut damage, 2P). The orchestrator records them in the [backlog](backlog.md) DEMO-01 row and turns each low score into a follow-up item.
