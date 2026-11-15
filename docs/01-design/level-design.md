# Level design, progression, enemy roster and encounter grammar

## Structure

Build **authored** compact rooms and interconnected short floor segments before considering procedural generation. A floor has: introduction, controlled interaction tutorial, optional alternate-route discovery, mixed encounter, checkpoint, short payoff. Each material interaction has an observable response and a route implication.

## Encounter building blocks

- **Wood / breakable stone:** predictable bombs create new passages; tiles convert to edge variants and spawn bounded debris.
- **Metal / reinforcement:** blocks normal explosives; clearly communicated by shape, not only color.
- **Glass / fragile panels:** cheaper to breach but may expose the party to enemies.
- **A single brittle beam:** collapse only a locally authored region when sufficiently damaged.
- **Red/marked explosive props:** chain reaction with capped propagation and readable warning.
- **Anchors and critical geometry:** mission exits, spawn pads and required traversal routes are invulnerable in v1.

## Example encounter: Inspection Corridor

1. Players enter at left; armored enemy guards an elevated exit.
2. The main corridor holds two enemies and wooden cover; a narrow upper route is visible.
3. Carl can spend a bomb to breach a wooden partition and reach the guard's flank.
4. Donut can use a control spell to pause enemy advance while Carl places the charge.
5. A breakable support creates an optional collapse that neutralizes the guard but may block the shortest route temporarily.
6. Debris settles; level validator or gameplay affordance guarantees a way to reach the exit by the main or upper route.

## Enemies (first slice)

- **Chaser:** communicates pathing and reaction to breached terrain; one ground attack.
- **Ranged guard:** forces flanking or use of cover; does not shoot through unbroken solids.
- **Optional elite:** telegraphed wind-up, destructible cover, short recovery.

Do not create 20 enemy types before proving the chaser and ranged guard. Keep enemy AI on a simple state machine (`Patrol`, `Alert`, `Pursue`, `Attack`, `Recover`, `StuckFallback`) and expose state in debug overlays.

## Progression

The initial game should not lock core character synergy behind collectible upgrades. Test one optional pickup (e.g., bomb refill) and one reward for exploring alternate paths. Only after the core loop works: modular equipment, gated abilities, a deliberate economy, and authored boss encounters.

## Route safety rules

- Exit reachable from at least one reachable platform after every *authored* mandatory destruction event.
- Essential jumping surfaces may be protected or recoverable.
- Explosions cannot permanently isolate both active heroes from the goal.
- Enemy nav graph updates after terrain change or uses a bounded recovery policy.
- A dynamic path-connectivity check should flag impossible levels in CI; runtime should support checkpoint reset when unexpected player actions bypass authoring assumptions.

## Boss design concept (not in slice)

Boss has three phases mapped to one material mechanic each. Strongly readable attack tells and arenas that change through controlled authored collapse; avoid requiring full general-purpose structural simulation. Design and scope only after room-level destruction is stable.
