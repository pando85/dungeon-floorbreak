# Visual review protocol for AI-authored pixel art

## Review order

1. Compare canonical master sprites against candidate at source resolution, no smoothing.
2. View contact sheets with consistent crop and pivots (idle, run, fall, cast/plant, hurt).
3. Play loop GIF or video at real target display scaling and speed.
4. Overlay the character on neutral background, normal gameplay background, busy explosion and overlapping ally.
5. Inspect hit-frame timing, silhouette, frame pivot alignment, palette and readable damage/cooldown feedback.
6. Record pass/fail with links, 3 actionable changes at most per iteration, and human decision.

## Automated checks

- Atlas files exist and metadata matches declared animations.
- Every frame rectangle lies inside atlas; alpha is respected.
- Pivots vary within explicitly allowed bounds.
- No missing/duplicate frame indexes; no negative frame duration.
- All asset identifiers and sources have license/provenance entries.
- Detect unusually changed silhouette bounding box and color palette drift as **warnings**, not absolute vetoes.

## Human review questions

- Can I identify Carl/Donut-equivalent silhouette in <1 second without reading text?
- Can I see whether an explosion broke wood or only created sparks?
- Is dark scenery atmospheric without hiding projectiles and breakable edges?
- Does the run animation feel energetic instead of sliding/puppeteering?
- Are melee and magical attack active frames obvious?
- Does it still read on 1280×800 handheld scale?

## Evaluation record template

```yaml
asset_id: caster_idle_v3
source_commit: '<sha>'
comparison_reference: 'canonical-v1'
preview: 'artifacts/caster_idle_v3.gif'
passes:
  silhouette: true
  palette: true
  pivot_alignment: false
  readability_in_explosion: true
human_approved: false
issues:
  - 'Ear jumps one pixel during the second idle loop'
next_action: 'Correct pivot, regenerate atlas and repeat comparison'
```
