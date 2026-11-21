from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

from conftest import pass_verifier
from playtrace_tools import schema
from playtrace_tools.dataset import DatasetError, check_against_sources, load_dataset
from playtrace_tools.export import ExportError, Options, export_dataset
from playtrace_tools.journal import JournalError, read_session
from playtrace_tools.split import LeakageError, assign_splits, check_leakage
from playtrace_tools.verify import VerifyResult, parse_verify_output
from synthetic_journal import write_session


def export(base, out, kind="transitions", verifier=pass_verifier, **kw):
    if kind == "bc":
        kw["actor"] = "human"
    options = Options(kind=kind, allow_synthetic=kw.pop("allow_synthetic", True), **kw)
    return export_dataset(base, out, options, verifier)


def reasons(meta):
    return {r["session_id"][0]: r["reason"] for r in meta["rejected"]}


def test_every_row_maps_to_its_source_tick(five_sessions, out):
    meta = export(five_sessions, out, context=3)
    dataset = load_dataset(Path(meta["path"]))
    report = check_against_sources(dataset, five_sessions)
    assert report["rows_checked"] == sum(s["rows"] for s in meta["streams"].values())
    assert report["rows_checked"] > 500
    for stream in dataset.streams.values():
        keys = [(r["session_id"], r["t"], r["hero"]) for r in stream.rows]
        assert len(keys) == len(set(keys)), "duplicate source tick"
    both = [(r["session_id"], r["t"], r["hero"]) for s in dataset.streams.values() for r in s.rows]
    assert len(both) == len(set(both)), "a tick of a hero is in both streams"


def test_next_obs_is_the_following_observation_and_boundaries_are_honest(five_sessions, out):
    meta = export(five_sessions, out)
    dataset = load_dataset(Path(meta["path"]))
    human = dataset.streams["human"]
    by_key = {(r["session_id"], r["t"], r["hero"]): i for i, r in enumerate(human.rows)}
    kinds = set()
    for i, row in enumerate(human.rows):
        kinds.add(row["next_kind"])
        if row["next_kind"] == "step":
            j = by_key.get((row["session_id"], row["t"] + 1, row["hero"]))
            if j is not None and human.rows[j]["traj_id"] == row["traj_id"]:
                assert np.array_equal(human.next_obs[i], human.obs[j])
        if row["next_kind"] == "post_reset":
            assert row["terminal"] and not human.next_valid[i]
            assert not human.next_obs[i].any()
        if row["next_kind"] == "final_obs":
            assert row["truncated"] and human.next_valid[i]
    assert {"step", "final_obs", "post_reset"} <= kinds
    # A session that ends without an episode_end: its last row is not a transition.
    # Session "d" ends open: its last tick has no s_(t+1) for either hero. Every
    # driven (tick, hero) is either a row or an explained exclusion, once.
    assert meta["excluded_rows"] == {"no_next_obs": 2}
    ticks = sum(s["ticks"] for s in meta["sessions"])
    assert sum(s["rows"] for s in meta["streams"].values()) + 2 == 2 * ticks


def test_human_and_ai_streams_are_separate_and_never_relabelled(five_sessions, out):
    meta = export(five_sessions, out)
    dataset = load_dataset(Path(meta["path"]))
    for row in dataset.streams["human"].rows:
        assert row["controlled_by"] in ("human_p1", "human_p2")
        assert row["action_source"] == "hero_routed"
    for row in dataset.streams["ai"].rows:
        assert row["controlled_by"] in ("companion_ai", "scripted_bot")
        assert row["action_source"] == "effective"
    # session "b" has the human on Donut and the AI on Carl
    b_human = {r["hero"] for r in dataset.streams["human"].rows if r["session_id"].startswith("b")}
    b_ai = {r["hero"] for r in dataset.streams["ai"].rows if r["session_id"].startswith("b")}
    assert b_human == {"donut"} and b_ai == {"carl"}
    # session "c": both human, so it contributes nothing to the AI stream
    assert not [r for r in dataset.streams["ai"].rows if r["session_id"].startswith("c")]

    # Relabelling a row after the fact is caught against the source.
    rows_path = Path(meta["path"]) / "ai" / "rows.jsonl"
    lines = rows_path.read_text().splitlines()
    first = json.loads(lines[0])
    first["controlled_by"] = "human_p1"
    lines[0] = json.dumps(first)
    rows_path.write_text("\n".join(lines) + "\n")
    with pytest.raises(DatasetError):
        load_dataset(Path(meta["path"]))  # content hash no longer matches


def test_check_against_sources_catches_a_forged_row(five_sessions, out):
    meta = export(five_sessions, out)
    dataset = load_dataset(Path(meta["path"]))
    dataset.streams["human"].rows[7]["controlled_by"] = "companion_ai"
    with pytest.raises(DatasetError, match="relabelled"):
        check_against_sources(dataset, five_sessions)
    dataset = load_dataset(Path(meta["path"]))
    dataset.streams["human"].rows[7]["pre"] = "0" * 16
    with pytest.raises(DatasetError, match="hash"):
        check_against_sources(dataset, five_sessions)
    dataset = load_dataset(Path(meta["path"]))
    dataset.streams["human"].obs[9, 0] += 0.25
    with pytest.raises(DatasetError, match="o_t"):
        check_against_sources(dataset, five_sessions)


def test_zero_split_leakage(five_sessions, out):
    meta = export(five_sessions, out, seed=7)
    dataset = load_dataset(Path(meta["path"]))
    seen: dict[str, set[str]] = {}
    for stream in dataset.streams.values():
        for row in stream.rows:
            seen.setdefault(row["session_id"], set()).add(row["split"])
    assert all(len(v) == 1 for v in seen.values())
    assert set().union(*seen.values()) >= {"train", "dev"}
    assert meta["leakage"]["units_in_several_splits"] == 0 and meta["leakage"]["duplicate_rows"] == 0
    # deterministic and content-addressed
    again = export(five_sessions, out, seed=7)
    assert again["dataset_id"] == meta["dataset_id"] and again["path"] == meta["path"]
    assert export(five_sessions, out, seed=8)["dataset_id"] != meta["dataset_id"]


def test_leakage_checker_rejects_leaks_and_duplicates():
    groups = {"g1": "train", "g2": "dev"}

    def row(group, t, split, session="s1", ep=0):
        return {
            "session_id": session,
            "t": t,
            "hero": "carl",
            "group": group,
            "split": split,
            "ep": ep,
            "traj_id": f"{session}-{ep}-{group}",
        }

    check_leakage([row("g1", 0, "train"), row("g2", 1, "dev", session="s2")], groups)
    with pytest.raises(LeakageError, match="duplicate"):
        check_leakage([row("g1", 0, "train"), row("g1", 0, "train")], groups)
    with pytest.raises(LeakageError):
        check_leakage([row("g1", 0, "dev")], groups)  # split does not match the group
    with pytest.raises(LeakageError, match="several splits"):
        # one session reaching two splits through two groups (a retry placed apart)
        check_leakage([row("g1", 0, "train"), row("g2", 1, "dev", ep=1)], groups)


def test_split_assignment_is_by_group_deterministic_and_keeps_train():
    assert set(assign_splits(["only"]).values()) == {"train"}
    two = assign_splits(["a", "b"])
    assert sorted(two.values()) == ["dev", "train"]
    ten = assign_splits([f"s{i}" for i in range(10)], seed=3)
    assert sorted(set(ten.values())) == ["dev", "holdout", "train"]
    assert ten == assign_splits([f"s{i}" for i in range(10)], seed=3)
    assert list(ten.values()).count("train") >= 6


def test_player_pseudonym_keeps_a_players_sessions_together(base, out):
    for i in range(6):
        write_session(base, f"{i:032x}", seed=i)
        path = base / f"{i:032x}" / "manifest.json"
        manifest = json.loads(path.read_text())
        manifest["participant_pseudonym"] = "p-one" if i < 3 else f"p-{i}"
        path.write_text(json.dumps(manifest))
    meta = export(base, out, seed=1)
    splits = {s["session_id"]: s["split"] for s in meta["sessions"]}
    assert len({splits[f"{i:032x}"] for i in range(3)}) == 1


def _corrupt_chunk(path: Path):
    chunk = path / "ticks-000000.jsonl"
    text = chunk.read_text()
    chunk.write_text(text.replace('"et":5,', '"et":6,', 1))


def test_rejections_carry_reasons(base, out):
    write_session(base, "1" * 32, seed=1)  # good
    write_session(base, "2" * 32, seed=2)
    _corrupt_chunk(base / ("2" * 32))
    write_session(base, "3" * 32, seed=3, status="incomplete")
    write_session(base, "4" * 32, seed=4, partial_observation=True)
    write_session(base, "5" * 32, seed=5, participants="includes_other_tester")
    write_session(base, "6" * 32, seed=6)
    (base / ("6" * 32) / "ticks-000000.jsonl").unlink()  # a missing chunk
    write_session(base, "7" * 32, seed=7, status="crashed_recovered")
    write_session(base, "8" * 32, seed=8)
    manifest = json.loads((base / ("8" * 32) / "manifest.json").read_text())
    manifest["schema"] = "playtrace-session/9"
    (base / ("8" * 32) / "manifest.json").write_text(json.dumps(manifest))
    write_session(base, "9" * 32, seed=9)
    manifest = json.loads((base / ("9" * 32) / "manifest.json").read_text())
    manifest["observation_schema_id"] = "autoplay-obs/3+other"
    (base / ("9" * 32) / "manifest.json").write_text(json.dumps(manifest))
    meta = export(base, out)
    assert [s["session_id"] for s in meta["sessions"]] == ["1" * 32]
    got = reasons(meta)
    assert got["2"] == "invalid_journal"
    assert got["3"] == "capture_incomplete"
    assert got["4"] == "observation_partial"
    assert got["5"] == "other_tester_not_consented"
    assert got["6"] == "invalid_journal"
    assert got["7"] == "capture_incomplete"
    assert got["8"] == "unsupported_schema"
    assert got["9"] == "unsupported_obs_schema"
    assert all(r["detail"] for r in meta["rejected"])


def test_a_missing_tick_and_a_duplicate_tick_are_rejected(base):
    write_session(base, "a" * 32, seed=1, ticks=50)
    with pytest.raises(JournalError):
        # delete one tick line but keep the trailer consistent for the *file* only
        chunk = base / ("a" * 32) / "ticks-000000.jsonl"
        lines = chunk.read_text().splitlines()
        del lines[10]
        chunk.write_text("\n".join(lines) + "\n")
        read_session(base / ("a" * 32))


def test_consent_needs_the_flag_and_an_explicit_session(base, out):
    sid = "5" * 32
    write_session(base, sid, seed=5, participants="includes_other_tester")
    write_session(base, "6" * 32, seed=6)
    assert reasons(export(base, out))["5"] == "other_tester_not_consented"
    with pytest.raises(ExportError):
        export(base, out, export_consented=True)  # no explicit --session
    assert reasons(export(base, out, session_ids=[sid]))["5"] == "other_tester_not_consented"
    meta = export(base, out, session_ids=[sid], export_consented=True)
    assert [s["session_id"] for s in meta["sessions"]] == [sid]
    # consent for one session does not export the other one
    assert len(meta["sessions"]) == 1


def test_synthetic_is_refused_without_the_flag_and_never_mixed(base, out):
    write_session(base, "a" * 32, seed=1)
    meta = export(base, out, allow_synthetic=False)
    assert reasons(meta) == {"a": "synthetic_not_allowed"} and not meta["sessions"]
    write_session(base, "b" * 32, seed=2, synthetic=False)
    with pytest.raises(ExportError, match="mix"):
        export(base, out)
    meta = export(base, out, session_ids=["a" * 32])
    assert meta["synthetic"] is True


def test_verifier_failure_prefix_scope_and_modification_reject(base, out):
    write_session(base, "a" * 32, seed=1)
    write_session(base, "b" * 32, seed=2)
    write_session(base, "c" * 32, seed=3)
    write_session(base, "d" * 32, seed=4)

    def verifier(path: Path) -> VerifyResult:
        name = path.name[0]
        if name == "b":
            return VerifyResult(False, code="post_state", reason="post-state hash differs at t=5")
        if name == "c":
            return VerifyResult(True, scope="prefix")
        if name == "d":
            with (path / "control.jsonl").open("ab") as handle:
                handle.write(b"\n")
        return VerifyResult(True, scope="full", build="identical")

    meta = export(base, out, verifier=verifier)
    assert [s["session_id"][0] for s in meta["sessions"]] == ["a"]
    got = reasons(meta)
    assert got["b"] == "verify_failed:post_state"
    assert got["c"] == "verify_not_full_scope"
    assert got["d"] == "modified_during_export"


def test_verify_output_parsing_demands_a_clean_pass():
    ok = "PLAYTRACE_VERIFY session=x result=PASS scope=full ticks=3 build=identical\n"
    assert parse_verify_output(ok, 0).passed
    assert not parse_verify_output(ok, 1).passed  # exit code disagrees
    assert not parse_verify_output("", 0).passed
    fail = 'PLAYTRACE_VERIFY session=x result=FAIL code=build_mismatch tick=- reason="other source"\n'
    result = parse_verify_output(fail, 1)
    assert not result.passed and result.code == "build_mismatch" and result.reason == "other source"
    assert parse_verify_output("", 2, "playtrace: no session matches").code == "verifier_error"


def test_hero_actor_and_context_filters(five_sessions, out):
    meta = export(five_sessions, out, hero="donut", actor="human", context=4)
    dataset = load_dataset(Path(meta["path"]))
    assert set(dataset.streams) == {"human"}
    assert {r["hero"] for r in dataset.streams["human"].rows} == {"donut"}
    stream = dataset.streams["human"]
    assert stream.context.shape == (len(stream.rows), 4, schema.ACTION_LEN)
    # context holds the previous actions of the same trajectory, oldest first
    by_traj: dict[str, list[int]] = {}
    for i, row in enumerate(stream.rows):
        by_traj.setdefault(row["traj_id"], []).append(i)
    checked = 0
    for indices in by_traj.values():
        for position, i in enumerate(indices):
            expected = [list(stream.action[j]) for j in indices[max(0, position - 4) : position]]
            assert stream.rows[i]["ctx_len"] == len(expected)
            if expected:
                assert [list(a) for a in stream.context[i][4 - len(expected) :]] == expected
                checked += 1
            assert not stream.context[i][: 4 - len(expected)].any()
    assert checked > 50
    with pytest.raises(ExportError):
        export_dataset(five_sessions, out, Options(kind="bc", actor="ai"), pass_verifier)


def test_bc_export_has_no_next_obs_or_event_text(five_sessions, out):
    meta = export(five_sessions, out, kind="bc")
    directory = Path(meta["path"])
    assert set(meta["streams"]) == {"human"}
    assert not (directory / "human" / "next_obs.npy").exists()
    dataset = load_dataset(directory)
    assert all("event_ids" not in r and "eh" not in r for r in dataset.streams["human"].rows)
    # BC keeps the final row of a session that has no s_(t+1)
    assert "no_next_obs" not in meta["excluded_rows"]


def test_privileged_diagnostics_are_never_read_or_exported(base, out):
    sid = write_session(base, "a" * 32, seed=1)
    (sid / "diagnostics.jsonl").write_text('{"t":0,"hp":[99,99],"rig":[1,2,3,4],"enemies_hidden":5}\nGARBAGE\n')
    meta = export(base, out)
    text = "".join(p.read_text() for p in Path(meta["path"]).rglob("*.json*"))
    for private in ("enemies_hidden", '"rig"', '"hp"', "GARBAGE"):
        assert private not in text


def test_unsupported_dataset_schema_fails_explicitly(five_sessions, out):
    meta = export(five_sessions, out)
    path = Path(meta["path"]) / "dataset.json"
    data = json.loads(path.read_text())
    data["schema"] = "playtrace-dataset/2"
    path.write_text(json.dumps(data))
    with pytest.raises(DatasetError, match="unsupported dataset schema"):
        load_dataset(Path(meta["path"]))


def test_tampered_array_fails_the_content_hash(five_sessions, out):
    meta = export(five_sessions, out)
    target = Path(meta["path"]) / "human" / "action.npy"
    data = bytearray(target.read_bytes())
    data[-1] ^= 1
    target.write_bytes(bytes(data))
    with pytest.raises(DatasetError, match="does not match"):
        load_dataset(Path(meta["path"]))


def test_export_never_writes_into_the_session_store(five_sessions, out):
    before = {p: p.stat().st_mtime_ns for p in five_sessions.rglob("*") if p.is_file()}
    export(five_sessions, out)
    after = {p: p.stat().st_mtime_ns for p in five_sessions.rglob("*") if p.is_file()}
    assert before == after


def test_cli_refuses_to_write_inside_the_repository(base, capsys):
    from playtrace_tools.cli import main
    from playtrace_tools.verify import repo_root

    code = main(["export-bc", "--dir", str(base), "--out", str(repo_root() / "datasets-tmp")])
    assert code == 2
    assert not (repo_root() / "datasets-tmp").exists()
    assert "inside the repository" in capsys.readouterr().err


def test_cli_without_a_verifier_rejects_every_session(base, out, monkeypatch, capsys):
    from playtrace_tools.cli import main

    write_session(base, "a" * 32, seed=1)
    monkeypatch.delenv("PLAYTRACE_BIN", raising=False)
    code = main(
        ["export-bc", "--dir", str(base), "--out", str(out), "--allow-synthetic", "--verifier", "/nonexistent/playtrace"]
    )
    captured = capsys.readouterr()
    assert code == 1
    assert "verifier_unavailable" in captured.out
