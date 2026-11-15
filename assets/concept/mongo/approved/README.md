# Mongo — approved source image register (upload pending)

**Selected art approved:** 2026-11-15. The original three PNGs below are **not yet present in Git**. Upload the exact source bytes under this directory and include `SHA256SUMS` before marking this PR as a full art delivery.

| Destination filename | Conversation source | Resolution | Size (bytes) | SHA-256 |
| --- | --- | --- | ---: | --- |
| `Mongo_Exploration_Approved_v1.png` | `a_detailed_concept_art_character_design_sheet_il.png` | 1672×941 | 3088804 | `2913475b7b6c6f0b1d66d28d21b03c0c3817570998329165a67397d0706c28c4` |
| `Mongo_Growth_Approved_v1.png` | `a_detailed_illustrated_character_design_sheet.png` | 1672×941 | 3250628 | `eb3e832861ba63e7621ed0b6f177377faa62988b57b5a21235b196bc65a06c94` |
| `Mongo_Adult_Approved_v1.png` | `a_highly_detailed_polished_game_character_design.png` | 1536×1024 | 3352900 | `4d686989291fc3ca9b668f48aae1a6b289c79cd1915685bedb8e587f797e8278` |

The owner chose the **first four-way exploration board**, then the **growth** board and finally the **adult model/acting** board. The intervening near-duplicate exploration is not the source master.

To finish: upload all three PNGs **unmodified** to `assets/concept/mongo/approved/` on branch `art/mongo-character-bible-v1`; upload `SHA256SUMS` using the exact three hashes and filenames above. Run `sha256sum -c assets/concept/mongo/approved/SHA256SUMS`, `python3 scripts/check_docs.py` and inspect the PR's documentation CI. Do not upload chat thumbnails, recompressed JPGs, crops or replacements. The checksum file is deliberately omitted until the images arrive because CI rejects missing referenced assets.

These are approved **visual concept references only**: no real pixel-art sprites, canonical book facts, gameplay readiness, broad IP license or game distribution rights are claimed.

See [character bible](../../../../docs/02-art/mongo-character-bible.md), [provenance](provenance.md), and [rights policy](../../../../docs/04-production/rights-and-release.md).
