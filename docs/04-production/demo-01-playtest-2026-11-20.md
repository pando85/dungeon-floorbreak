# DEMO-01 — owner playtest results, 2026-11-20

**Evidence class:** first-person owner report through the repository's 13-question [DEMO-01 questionnaire](demo-01-questionnaire.md). This is authoritative **subjective game-feel feedback**, not proof of a specific code defect. The report was taken in chat after the owner played; no recording, log or exact tested commit was supplied.

**Unknown / do not infer:** build SHA, launch flags, display resolution, input device model, duration, difficulty mode, whether preset 4 or CINE-01 was explicitly configured, and number of deaths. Next playtest must capture these fields. Do not claim any owner co-op test.

## Exact structured responses

| Question | Owner response | Meaning for next work |
| --- | --- | --- |
| Q1 Carl movement | 3/5; no detail | Serviceable baseline, tune only with measured benefit |
| Q2 Basic combat | 3/5; "**Se hace monótono y aburrido. Faltan ataques y combos. Las muertes son siempre igual**" | Main issue: too few distinct attack choices, feedback and defeat outcomes |
| Q3A Kick distance | 3/5 | Distance approximately right; do **not** reflexively increase impulse |
| Q3B Goblin-on-goblin impact | **1/5**; no detail | A critical failure of perceivable contact; inspect triggered events versus visual response |
| Q4 Downed enemies | "**Solo patadas**" and "**Esa parte queda rara, el enemigo cae pero no puedes aprovechar la ventaja. Quizás deberíamos permitir pisotones y poder rematarlos. Y donut dar zarpazos**" | Keep kicks useful against downed targets; experiment with contextual stomp/finisher and Donut swat/pounce |
| Q5 Respawn/checkpoint | **Not tested** | Do not treat as approved or rejected |
| Q6A Environmental destruction | 3/5 | Improve causal, interactive payoff |
| Q6B Falling block hurting player | **Yes** | Owner-selected direction; still design fair tells, hazard timing, co-op/friendly interactions |
| Q7A Gore intensity | **1/5** on **too little -> too much** scale | Too tame at perceived play settings, not "bad quality 1/5" |
| Q7B Gore readability | 3/5 | Acceptable, still improvable |
| Q7C Corpse/stain clutter | 3/5 on **too cluttered -> too clean** scale; "**Pero de deberían poder romper más**" | Current amount of residue acceptable; likely asks for more visible physical breakup (context: death effects). Do not infer unlimited persistent corpses |
| Q8A Decapitation | No | Unobserved; distinguish eligibility, staging and visibility |
| Q8B Explosion/dismemberment | No | Unobserved; inspect trigger path and visual capture |
| Q8C Special finishes | **Not rated** | Cannot conclude presentation quality/frequency from this answer |
| Q9A Llama warning / evasion | 3/5; "**Parece insuficiente las opciones de esquiva**" | Explore meaningful evasive choices without a new mandatory dodge button |
| Q9B Llama heavy-vs-normal death | "**No noté diferencia**" | Improve cause-specific read or verify that both were actually seen |
| Q10A Camera zoom | **Not observed** | No human verdict on zoom smoothness |
| Q10B HUD | 3/5; "**A veces he notado movimientos bruscos de cámara**" | Evaluate pan / swap / impact kick / zoom traces separately |
| Q11 Tone / identity | **2/5** | Demo feels insufficiently distinctive, despite approved concept assets |
| Q12A Solo swap | **2/5** | Input, handover and camera feel need a real comparison |
| Q12B AI Donut | **2/5** | Companion feels passive / not useful |
| Q12C Local 2P | **Not tested** | Real two-controller game-feel and device behavior still unvalidated |
| Q13 Cut or tone down | "**Los combates son muy muy básicos**" | Do not expand content before satisfying micro-combat |
| Q13 Amplify first | "**Combos, más ataques, más opciones de esquiva, escenarios más ricos....**" | New microarena must exercise all of these |
| Q13 Additional issue | "**Donut parece que no ataca ni puede hacer nada**" | Distinguish controlled Donut (#123 pending) from follower AI (still no self-initiated attack in #123) |

The original replies were in Spanish. Quoted passages are the owner's own words, not translations or new canon.

## Grounded conclusions

1. **Repetition and lack of consequence are the central product failures.** A 3/5 basic hit plus a 1/5 enemy collision and no recognized special finishes is more informative than any raw number of existing Bevy systems.
2. **Donut matters in both roles.** Playing her actively, swapping to her and watching her AI behavior are separate experiences; Q12 scored the latter two 2/5. The Q10 + mana PR #123 addresses the first but *does not* make the AI initiate attacks.
3. **Destruction needs quality and agency more than additional residue.** The owner finds retained blood/body clutter adequate and wants more bodies visibly broken and richer usable scenery, not indiscriminate gore density.
4. **Visibility is not the same as implementation.** Decapitation, explosion, cinematic finishes and zoom have independent fixtures/conditions. The review did not prove any of them broken and could not rate the unseen effects.
5. **Audience/HUD work is not a prerequisite.** The viewer counters begin story-locked and should not compensate for insufficient base combat.

## Baseline acceptance goals for the next playtest

These are **proposed success targets** for the redesign, not already approved facts: basic combat >=4/5, enemy-to-enemy contacts >=4/5, gore intensity approximately 3/5 on its appropriateness scale, death readability >=4/5, solo swap >=4/5, Donut AI >=4/5, tone >=4/5, HUD >=4/5. Observe one genuine differentiated heavy finish and one skill-gated cinematic moment without scripting an ordinary kill to look extraordinary. Checkpoint and two-pad behavior must receive ratings for the first time. Record build + device and ensure no misleading comparison between pre- and post-biscuit character states.

## Reproduction / test handoff

- Primary current demo: `bash scripts/export_assets.sh`; `cargo run --release --locked --bin floor01_scene -- --demo --scale 3`.
- Run beat-specific probes where necessary (B2 body contact, B3 falling block and edge, B4 llama, B5 blast), and distinguish scripted/autopilot evidence from actual manual play.
- For every missed spectacle: record `CombatEvent` / `EnemyDefeated` (or relevant existing event), actual selection and camera visibility, then an original-pixel clip. For camera jitter, log rest width, pan delta, swap and impact kick on the same tick.
- Before/after A/B testing uses the **same starting room, enemies, input ownership, difficulty, resolution and audio settings**. Compare human impressions and traces, not only a screenshot.
- Reviewer must flag untested issues rather than turning them into green checkboxes.

**Follow-up design:** [COMBAT-FUN-02](../01-design/combat-fun-02-post-demo01-proposals.md). **Implementation work order:** [COMBAT-FUN-02 agent plan](../07-agents/combat-fun-02-workorder.md).
