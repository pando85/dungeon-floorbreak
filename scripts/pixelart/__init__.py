"""Shared pixel-art toolkit (PIPE-01).

One toolkit that takes a new asset from an approved concept to validated sources, evidence and a three-image
review pack, instead of a bespoke script per asset. See ``docs/07-agents/pixel-art-pipeline.md`` and the
"Toolkit" section of ``docs/02-art/asset-pipeline.md``.

Commands (``python3 -m scripts.pixelart <command>`` from the repository root):

- ``packet``      concept crops, palette candidates, scale lineup, frame/pivot template, spec template
- ``validate``    parametrized rules from a per-asset ``pixelart-asset-v1`` config, JSON report + text summary
- ``evidence``    per-tag GIFs at tick timing, contact sheets, lit Floor 1 scene, SHA256SUMS
- ``review-pack`` exactly three images plus the validator report (the only input of a top-tier reviewer)
- ``freeze``      write the frozen-key fixture (SHA-256 per key frame) from a grid
- ``compare``     concept art next to pixel frames

The toolkit only reads sources and writes into an ``--out`` directory. It never changes existing assets,
manifests or the output of the per-asset scripts.
"""
