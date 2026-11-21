#!/usr/bin/env python3
"""Session helpers of `ci_floor01_combat.sh replay` (PTR-R1).

Only ever used on the synthetic session the script records itself; it never
reads or writes a human recording.

  last-post DIR            print the `post` hash of the last TickRecord
  is-png FILE              exit 1 unless FILE starts with the PNG signature
  cross-build DIR          rewrite the manifest's source fingerprint (a
                           recording by another build)
  tamper-post DIR TICK     flip one bit of one tick's `post` hash and reseal the
                           chunk (trailer fnv, manifest checksum), so only the
                           replay can notice
"""

import glob
import json
import sys

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3


def fnv(lines):
    h = FNV_OFFSET
    for line in lines:
        for byte in line.encode() + b"\n":
            h = ((h ^ byte) * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return h


def chunk_files(directory):
    return sorted(glob.glob(directory + "/ticks-*.jsonl"))


def last_post(directory):
    last = None
    for path in chunk_files(directory):
        for line in open(path):
            if line.strip() and '"end":true' not in line:
                last = json.loads(line)
    print(last["post"])


def is_png(path):
    if open(path, "rb").read(8) != b"\x89PNG\r\n\x1a\n":
        sys.exit(f"{path} is not a PNG")


def cross_build(directory):
    path = directory + "/manifest.json"
    manifest = json.load(open(path))
    manifest["source_fingerprint"] = "0123456789abcdef"
    json.dump(manifest, open(path, "w"), indent=2)


def tamper_post(directory, tick):
    for index, path in enumerate(chunk_files(directory)):
        rows = [line.rstrip("\n") for line in open(path)]
        ticks, trailer = rows[:-1], json.loads(rows[-1])
        for at, row in enumerate(ticks):
            record = json.loads(row)
            if record["t"] != tick:
                continue
            record["post"] = "%016x" % (int(record["post"], 16) ^ 1)
            ticks[at] = json.dumps(record, separators=(",", ":"))
            trailer["fnv"] = "%016x" % fnv(ticks)
            body = "".join(line + "\n" for line in ticks)
            body += json.dumps(trailer, separators=(",", ":")) + "\n"
            open(path, "w").write(body)
            manifest_path = directory + "/manifest.json"
            manifest = json.load(open(manifest_path))
            manifest["chunk_checksums"][index]["fnv"] = trailer["fnv"]
            json.dump(manifest, open(manifest_path, "w"), indent=2)
            return
    sys.exit(f"no tick {tick}")


def main(argv):
    commands = {
        "last-post": (last_post, 1),
        "is-png": (is_png, 1),
        "cross-build": (cross_build, 1),
        "tamper-post": (lambda d, t: tamper_post(d, int(t)), 2),
    }
    if len(argv) < 2 or argv[1] not in commands:
        sys.exit(__doc__)
    function, arity = commands[argv[1]]
    if len(argv) != 2 + arity:
        sys.exit(__doc__)
    function(*argv[2:])


if __name__ == "__main__":
    main(sys.argv)
