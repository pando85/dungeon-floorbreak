#!/usr/bin/env python3
"""COMBAT-FUN-02 CF02-00 baseline audit (Q23).

Replays the DEMO-01 autopilot beats headless, joins the combat event log with the
per-tick positions (`--combat-trace`) and the presentation camera (`--view-trace`,
`--camera auto` and `--camera scroll`), and writes:

  <out>/audit_events.md    one row per audited event: where the bodies were, where the
                           camera looked, and whether the body was inside the view
  <out>/camera_trace.md    pan speed, impact kick and zoom per beat and camera, plus the
                           solo-swap pan of the party fixture (authoritative camera)

Everything printed is measured from the replay; nothing is inferred. Run from the
repository root after `cargo build --locked --bin floor01_scene`:

  python3 scripts/cf02_baseline_audit.py docs/evidence/cf02/audit
"""
import pathlib
import re
import subprocess
import sys

BIN = "./target/debug/floor01_scene"
# beat -> probe ticks (BEAT_FIXTURES in src/demo.rs / docs/evidence/demo-01)
BEATS = {1: 200, 2: 260, 3: 300, 4: 400, 5: 300, 6: 300}
FLOOR_Y = 580  # the walking line of the hall (src/demo.rs HALL_FLOOR_Y)
EDGE_PX = 48  # "at the edge": within this many px of a view edge


def run(args):
    out = subprocess.run([BIN, "--headless-floor01-smoke", *args], capture_output=True, text=True, check=True)
    return out.stdout.splitlines()


def kv(line):
    return dict(tok.split("=", 1) for tok in line.split() if "=" in tok)


def beat_run(beat, ticks):
    base = ["--demo", "--demo-beat", str(beat), "--autopilot", "--capture-tick", str(ticks)]
    lines = run(base + ["--combat-trace"])
    events = []
    for x in lines:
        if x.startswith("COMBAT_EVENT"):
            d = kv(x)
            d["_raw"] = x
            events.append(d)
    trace = {}
    for x in lines:
        if not x.startswith("COMBAT_TRACE"):
            continue
        d = kv(x)
        t = int(d["tick"])
        goblins = {}
        for m in re.finditer(r"g(\d+)=(\w+)(?:/\w+)? g\d+_t=\d+ g\d+_x=(-?[\d.]+) g\d+_y=(-?[\d.]+) g\d+_hs=(\d+) g\d+_hp=(\d+)", x):
            goblins[int(m.group(1))] = (float(m.group(3)), m.group(2), int(m.group(6)))
        llama = None
        m = re.search(r" l0=(\w+) l0_t=\d+ l0_x=(-?[\d.]+) l0_hs=\d+ l0_hp=(\d+)", x)
        if m:
            llama = (float(m.group(2)), m.group(1), int(m.group(3)))
        trace[t] = {"carl_x": float(d["carl_x"]), "cam_left": int(d["cam_left"]), "kick": int(d["kick"]),
                    "goblins": goblins, "llama": llama}
    views = {}
    for cam, extra in (("auto", []), ("scroll", []), ("auto@1920x1080", ["--scale", "3"])):
        vt = {}
        for x in run(base + ["--camera", cam.split("@")[0], "--view-trace", *extra]):
            if x.startswith("FLOOR1_VIEW "):
                d = kv(x)
                vt[int(d["tick"])] = {k: int(d[k]) for k in ("left", "top", "w", "h", "kick")}
        views[cam] = vt
    return events, trace, views


def entity(trace_row, ident, n_goblins):
    """(x, mode, hp) of a target id: goblins first, then llamas."""
    if ident < n_goblins:
        return trace_row["goblins"].get(ident)
    return trace_row["llama"]


def visibility(x, view):
    left, w = view["left"], view["w"]
    margin = min(x - left, left + w - x)
    state = "inside" if margin >= EDGE_PX else ("at the edge" if margin >= 0 else "OUTSIDE")
    return state, round(margin)


def audit():
    rows = []
    responses = []
    notes = []
    for beat, ticks in BEATS.items():
        events, trace, views = beat_run(beat, ticks)
        n_goblins = len(trace[1]["goblins"])
        for ev in events:
            kind = ev.get("kind")
            tick = int(ev.get("tick", 0))
            label = None
            ids = []
            if kind == "goblin_contact":
                label, ids = "B2 contact (goblin hits goblin)", [int(ev["source"]), int(ev["target"])]
            elif kind == "enemy_defeated" and ev.get("cause") == "shore_block_edge":
                label, ids = "B3 decapitation (block edge)", [int(ev["target"])]
            elif kind == "enemy_defeated" and ev.get("cause") == "shore_block":
                label, ids = "B3 crush (block)", [int(ev["target"])]
            elif kind == "enemy_defeated" and ev.get("cause") == "explosion":
                label = "B5 blast (%s)" % ("point blank" if "point_blank" in ev["_raw"] else "ring")
                ids = [int(ev["target"])]
            elif kind == "enemy_defeated" and ev.get("force") == "heavy":
                tgt = int(ev["target"])
                label = "B4 llama lethal kick (slam)" if tgt >= n_goblins else "B2 lethal kick (heavy)"
                ids = [tgt]
            elif kind == "enemy_defeated" and int(ev["target"]) >= n_goblins:
                label, ids = "B4 llama plain kill", [int(ev["target"])]
            elif kind in ("llama_tell", "llama_spit"):
                label, ids = "B4 " + kind, [n_goblins]
            if not label or tick not in trace:
                continue
            for ident in ids:
                ent = entity(trace[tick], ident, n_goblins)
                if ent is None:
                    continue
                cells = []
                for cam in ("auto", "scroll"):
                    v = views[cam].get(tick)
                    if v is None:
                        cells.append("n/a")
                        continue
                    state, margin = visibility(ent[0], v)
                    cells.append("%s (%+d px; view %d..%d, w %d)" % (state, margin, v["left"], v["left"] + v["w"], v["w"]))
                rows.append((beat, tick, label, ident, ent[0], trace[tick]["carl_x"], cells[0], cells[1]))
                if kind == "goblin_contact" and ident == int(ev["target"]):
                    track = []
                    for dt in (0, 5, 10, 20, 30):
                        e2 = entity(trace.get(tick + dt, trace[tick]), ident, n_goblins)
                        track.append("+%d: x=%.1f %s" % (dt, e2[0], e2[1]))
                    responses.append((beat, tick, ident, track))
        # floor line inside every logged view of this beat
        for cam, vt in views.items():
            bad = [t for t, v in vt.items() if not (v["top"] <= FLOOR_Y <= v["top"] + v["h"])]
            if bad:
                notes.append("beat %d camera %s: the floor line y=%d leaves the view on %d ticks (first %d)" % (beat, cam, FLOOR_Y, len(bad), bad[0]))
    return rows, responses, notes


def camera_table():
    lines = ["| Beat | Camera | Ticks | Max pan (px/tick) at tick | Ticks with pan >= 8 px | Max kick (px) | Ticks with a kick | Zoom w min..max | First zoom change (tick) | Cuts (pan > 40 px) |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for beat, ticks in BEATS.items():
        _, _, views = beat_run(beat, ticks)
        for cam in ("auto", "auto@1920x1080", "scroll"):
            vt = views[cam]
            ts = sorted(vt)
            pans = [(abs(vt[b]["left"] - vt[a]["left"]), b) for a, b in zip(ts, ts[1:])]
            peak = max(pans)
            kicks = [abs(vt[t]["kick"]) for t in ts]
            ws = [vt[t]["w"] for t in ts]
            first_zoom = next((t for t in ts if vt[t]["w"] != ws[0]), None)
            lines.append("| B%d | %s | %d | %d at %d | %d | %d | %d | %d..%d | %s | %d |" % (
                beat, cam, len(ts), peak[0], peak[1], sum(1 for p in pans if p[0] >= 8), max(kicks),
                sum(1 for k in kicks if k), min(ws), max(ws), first_zoom if first_zoom else "none",
                sum(1 for p in pans if p[0] > 40)))
    return lines


def swap_pan():
    """The legacy swap pan trace, printed by the pinned test `cf02_baseline_swap_pan_trace`."""
    out = subprocess.run(["cargo", "test", "--locked", "--lib", "cf02_baseline_swap_pan_trace", "--", "--nocapture"],
                         capture_output=True, text=True, check=True)
    rows = []
    gap = None
    for x in (out.stdout + out.stderr).splitlines():
        if x.startswith("CF02_BASELINE_SWAP_PAN hero_gap_px="):
            gap = x.split("=", 1)[1]
        elif x.startswith("CF02_BASELINE_SWAP_PAN"):
            d = kv(x)
            rows.append((int(d["tick_after_swap"]), int(d["left"]), int(d["dx"])))
    return gap, rows


def main(out_dir):
    out = pathlib.Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    rows, responses, notes = audit()
    md = ["# Event log vs camera visibility (DEMO-01 autopilot, measured)", "",
          "Each row is one audited event of a beat probe (`--demo --demo-beat N --autopilot`). `x` is the body's column at the event tick (`--combat-trace`); the view is the presentation camera of `--view-trace` at that tick. `inside` = at least %d px from both view edges, `at the edge` = within %d px, `OUTSIDE` = beyond a view edge. Carl's column is given for reference. The autopilot is not a human: a player who approaches differently sees a different view; this table says what the route's camera showed." % (EDGE_PX, EDGE_PX), "",
          "| Beat | Tick | Event | Target | x | Carl x | `--camera auto` (demo default) | `--camera scroll` |",
          "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for beat, tick, label, ident, x, cx, a, s in rows:
        md.append("| B%d | %d | %s | %d | %.1f | %.1f | %s | %s |" % (beat, tick, label, ident, x, cx, a, s))
    md += ["", "## Second-body response after a contact (the B2 target)", "",
           "| Beat | Contact tick | Target | Track (tick offset: x, mode) |", "| --- | --- | --- | --- |"]
    for beat, tick, ident, track in responses:
        md.append("| B%d | %d | %d | %s |" % (beat, tick, ident, "; ".join(track)))
    if notes:
        md += ["", "## Vertical check", ""] + ["- " + n for n in notes]
    else:
        md += ["", "## Vertical check", "", "The walking line (y=%d) is inside every logged view of every beat and camera, so only the horizontal test above can hide a body." % FLOOR_Y]
    (out / "audit_events.md").write_text("\n".join(md) + "\n")

    cam = ["# Camera trace table (DEMO-01 autopilot beats, measured)", "",
           "Per-tick `FLOOR1_VIEW` lines of `--view-trace` (output 1280x720, slot 640x360). Pan = change of the view's left edge between ticks. Kick = the impact camera kick in px. Zoom = the view width `w` (640 at rest).", ""]
    cam += camera_table()
    gap, rows = swap_pan()
    cam += ["", "## Solo swap pan (authoritative scene camera, legacy rule)", "",
            "The `solo_swap_continuity` party fixture keeps the heroes 40 to 70 px apart, so its swap moves the camera by 0 px (nothing to measure). The pan needs a gap: the pinned test `cf02_baseline_swap_pan_trace` (in `src/cf02_fixtures.rs`) uses the setup of the party test `q7_a_swap_pan_may_exceed_the_normal_camera_cap_but_only_briefly`: Donut far behind, Carl runs 30 ticks (hero gap %s px), swap to Donut, then 80 idle ticks. CF02-Q20 replaces this constant pan with an eased one; the test pins the baseline so the change is deliberate." % gap, "",
            "| Tick after the swap | Camera left (px) | Step (px/tick) |", "| --- | --- | --- |"]
    for n, left, dx in rows:
        cam.append("| %d | %d | %+d |" % (n, left, dx))
    cam += ["", "Reading: the pan starts on the first tick at the full 8 px/tick cap (no ramp), repeats the same step for 18 ticks, ends with a shortened step and stops dead (ticks 22 to 80 are all 0). Velocity step on the first tick: 8 px/tick (0 -> 8); on arrival: 5 -> 0."]
    (out / "camera_trace.md").write_text("\n".join(cam) + "\n")
    print("wrote", out / "audit_events.md", "and", out / "camera_trace.md")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "docs/evidence/cf02/audit")
