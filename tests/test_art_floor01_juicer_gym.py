"""Protect the approved Juicer Gym concept source bytes in their own art PR.

Concept sheets are *not* native gameplay textures; hashes prevent accidental
crops, recompression or asset swaps between visual review and production.
"""
import hashlib
import io
import json
from pathlib import Path
import unittest

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
DIR = ROOT / "assets/concept/floor-01/juicer-gym"
MANIFEST = DIR / "manifest.json"
EXPECTED = {
    "floor01-juicer-gym-approved-mood-reference.png",
    "floor01-juicer-gym-standalone-revision-candidate.png",
}


class JuicerGymConceptImageTest(unittest.TestCase):
    def test_original_gym_image_bytes_are_pinned(self):
        with MANIFEST.open(encoding="utf-8") as file:
            manifest = json.load(file)

        self.assertEqual(manifest["schema"], "the-crawl-concept-image-manifest-v1")
        entries = manifest["assets"]
        self.assertEqual(len(entries), 2)
        self.assertEqual({Path(e["path"]).name for e in entries}, EXPECTED)
        self.assertEqual({p.name for p in DIR.glob("*.png")}, EXPECTED)

        for e in entries:
            with self.subTest(filename=e["path"]):
                name = Path(e["path"])
                self.assertEqual(name.parent, DIR.relative_to(ROOT))
                blob = (ROOT / name).read_bytes()
                self.assertEqual(len(blob), e["bytes"])
                self.assertEqual(hashlib.sha256(blob).hexdigest(), e["sha256"])
                with Image.open(io.BytesIO(blob)) as im:
                    self.assertEqual(im.format, "PNG")
                    self.assertEqual(im.size, (e["width"], e["height"]))
                    im.verify()


if __name__ == "__main__":
    unittest.main()
