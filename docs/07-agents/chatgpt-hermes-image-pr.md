# ChatGPT ↔ Hermes: mobile image PR handoff

**Policy adopted 2026-11-17.** This documents the agreed workflow. It does not mean a generated image is automatically accessible to GitHub or Hermes.

## Owner experience

When the owner approves ChatGPT-generated THE CRAWL concept images and asks for a PR, **ChatGPT should prepare a draft PR plus a PR-specific ZIP in the same reply**, without needing a separate request for the ZIP.

1. ChatGPT prepares a text-only image intake spec and PR branch, including exact destination paths, approved design choices, pending image slots and review requirements. If the owner identifies an existing PR, update that PR instead.
2. ChatGPT generates a ZIP in the current runtime and links it together with the verified PR URL. The ZIP contains `README-ES.md`, `hermes-task.md` and `handoff.json` (contract below).
3. The owner sends the ZIP **and the original PNG files** to installed Hermes via Android/Telegram as documents/files, not compressed photos.
4. Hermes commits the original binary files **to the same PR head branch**, updates provenance and image manifests and reports validation/CI results.
5. ChatGPT can review that PR and help resolve remaining checks. The owner retains final art/game-feel and merge decisions.

**Do not** make the owner upload the PNGs manually into GitHub, ask Hermes to open a duplicate PR, or pretend the files arrived because ChatGPT displayed them. ChatGPT's generated-image previews and ChatGPT sandbox links are not guaranteed to be accessible to Hermes. A ZIP that only contains instructions must say so explicitly.

## Responsibilities

- **ChatGPT:** design decisions, check current `main` and `AGENTS.md`, choose correct destinations, create/update the draft PR and return an exact PR-specific ZIP. Never invent image byte sizes, SHA-256, image filenames, dimensions or completed tests.
- **Hermes:** receive actual files, verify originals, copy bytes unchanged, update the existing PR branch and relevant manifests/docs, run validation and check CI, return the same PR URL.
- **Local art agents:** create editable `.ase` / pixel-grid source assets and implement Rust + Bevy gameplay when separately requested. Concept PNGs are not final production assets.
- **Owner:** approve creative direction, deliver original image files, review final output. An art approval is not a general game-distribution license.

## Standard ChatGPT procedure

1. Inspect `AGENTS.md`, `docs/04-production/rights-and-release.md`, the relevant art docs and existing destination README/manifests. Check existing PRs to avoid duplication.
2. Record exact image slots in an English intake specification or PR body. Do not add missing PNGs or fake hash/manifest records.
3. Create or update the PR head branch. **Keep image-dependent PRs draft** until Hermes has committed the originals and the checks have passed. Text-only specs may be ready independently.
4. Create the PR-specific ZIP with values copied from the **real** PR (number/head branch) and actual planned paths. If the PR could not be created, set `pr_number` and `head_branch` to `null`, describe the blocker and do not claim success.
5. Deliver both links (PR and downloadable ZIP) with a short Spanish instruction to send the ZIP and PNG originals to Hermes. Do not claim background execution or automatic upload.
6. On review, verify committed originals by SHA-256, manifest entries, provenance and CI before stating that the PR is ready or requesting merge.

If verifiable original image files are already accessible in the current runtime, the ZIP may include them with checked hashes; otherwise set `images_in_zip: false` and leave the binary work to Hermes.

## ZIP contract (v1)

Contents:
- `README-ES.md`: one-screen Android usage instructions and exact PR target.
- `hermes-task.md`: English commands/criteria (below), including branch/paths.
- `handoff.json`: machine-readable PR/asset mapping using `the-crawl-hermes-pr-handoff-v1`.

Sample shape (replace example values; never copy them verbatim):

~~~json
{
  "schema": "the-crawl-hermes-pr-handoff-v1",
  "repository": "pando85/dungeon-floorbreak",
  "base_branch": "main",
  "pr_number": 123,
  "head_branch": "art/floor01-example",
  "owner_approval": "approved visual direction only",
  "images_in_zip": false,
  "assets": [
    {
      "slot": "floor01-example",
      "target_path": "assets/concept/floor-01/enemies/example.png",
      "source_filename": null,
      "status": "approved",
      "notes": "Example only; replace with the real decision"
    }
  ],
  "spec_paths": []
}
~~~

Status is `exploration` unless owner approval is explicitly recorded. A PR target must exist before sending a numbered handoff; never guess its head branch. `source_filename` can remain null until Hermes receives the actual file.

## Hermes task requirements

- Verify live PR number, repository, head branch and GitHub write permissions. Fetch/check out the **same branch**, never commit directly to `main` or create a second PR.
- Find the actual input binaries on the Hermes host. A message preview/ChatGPT URL alone is insufficient. Ask for missing originals, then stop; no pretend uploads.
- Route concept art to the right `assets/concept/` path; never put concept reference PNGs in `assets/source/` or `generated/art/`. Compare hashes to avoid duplicates and never replace an approved image implicitly.
- Preserve original file bytes (no cropping, re-encoding, resizing or recompression). Record original filename, actual SHA-256, byte count, dimensions, source tool (when known), approval scope and rights note.
- Follow existing directory contracts (`manifest.json`, `README.md`, `SHA256SUMS` and/or `provenance.md`). Don't introduce conflicting metadata schemas.
- Run relevant `scripts/check_docs.py`, `scripts/validate_assets.py` and project CI. Only check off PR items that demonstrably passed; record blockers and send the same PR URL back.
- Follow `AGENTS.md`: English repository contents, 1980–1990s action antihero aesthetic, human final review, rights policy, and no automatic merge.

### Current concept destinations

| Concept | Destination |
| --- | --- |
| Carl | `assets/concept/carl/approved/` |
| Donut | `assets/concept/donut/approved/` |
| Mongo | `assets/concept/mongo/approved/` |
| Floor 01 goblins / Murder Dozer / opening | `assets/concept/floor-01/opening/` |
| Floor 01 enemies and bosses | `assets/concept/floor-01/enemies/` |
| Floor 01 environments | `assets/concept/floor-01/atlas-v1/` |

Mechanics specs and workorders belong to `docs/01-design/`, `docs/02-art/` or `docs/07-agents/`, as appropriate. Verify paths against the current repository at handoff time.

## Required PR checklist

- [ ] All original images received and committed to this PR branch
- [ ] Verified binary SHA-256, byte sizes and dimensions recorded
- [ ] Correct asset paths, approval scope, source and rights provenance recorded
- [ ] Existing manifest / README / checksum conventions preserved
- [ ] Relevant validation and CI passed or real blockers documented
- [ ] Owner reviewed final approved visuals

An image-dependent PR is **not merge-ready** while required images are pending. Do not auto-merge.

## Future chats

This repository file is the durable source of truth. When the owner next requests an image PR, consult this workflow and generate a **fresh PR-specific ZIP** for that PR rather than reusing an old generic install ZIP. Conversation memory is not a substitute for checking current branch, paths and approvals.
