# Linux, Steam Deck and Steam launch constraints

## Shipping sequence

1. **Linux native x86_64 dev builds** from a reproducible environment. No Steam integration needed to validate the game loop.
2. **Steam Deck handheld test**: gamepad, 1280×800/16:10 UI, pixel scaling, stable frame time, UI texts and sleep/resume.
3. **Windows builds** once Linux gameplay is stable, ensure controller/game saves are platform-neutral.
4. **Steamworks integration and store submission** when rights and game scope are viable: depots/builds, achievements (optional), cloud saves (optional), crash and support procedures.

## Controller-first target

A game must be playable from launch to exit using controller only. No keyboard prompt, external launcher or forced mouse click. Steam Input integration can aid remapping, but internal normalized actions remain platform-independent. Local multiplayer should treat device-slot assignment as a first-class test.

## Remote co-op

Steam Remote Play Together may later allow a local co-op game to be streamed to a remote guest; it is **not** authoritative online multiplayer. It relies on host streaming, supported Steam platform configuration and connection quality. Native netcode remains outside MVP scope.

## Pixel and display concerns

Virtual game canvas candidates 640×360 and 640×400; use safe area and UI sizing per aspect ratio, not stretching. Verify alternate fullscreen/borderless modes. Controller glyphs shouldn't assume Xbox face-button labels universally.

## Performance metrics

Aim: 60 fps on Steam Deck-class hardware with meaningful blast scenarios. Set concrete frame-time and VRAM/memory budgets only after a measured baseline. Prioritize fixed-step determinism and terrain collider rebuild spikes. Keep optional VFX scalability setting for particle/smoke density.

## Compliance and public rights

Publishing a public repo does **not** authorize shipping a game based on copyrighted franchise characters. Before a Steam release: secure rights to derivative IP or replace with original characters/lore; verify distribution rights for art/music/fonts; follow current Steamworks documentation for AI content disclosures and submissions. Do not claim a Steam Deck Verified badge before formal review.

## Source links

- https://partner.steamgames.com/doc/
- https://partner.steamgames.com/doc/steamdeck
- https://partner.steamgames.com/doc/features/remoteplay
- https://partner.steamgames.com/doc/sdk/uploading

Review the official sources again before actual launch: platform policies and SDK integration may change.
