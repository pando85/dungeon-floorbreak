"""Validate the owner-approved Floor 01 concept PNGs against their manifest.

Unlike production sprite exports, these are fixed original concept images.
Keep their exact bytes and approved identity rather than rasterizing or
recompressing them into runtime art.
"""

import hashlib
import io
import json
from pathlib import Path
import unittest

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
CONCEPT_DIR = ROOT / "assets/concept/floor-01/enemies"
MANIFEST = CONCEPT_DIR / "manifest.json"
EXPECTED_NAMES = {
    "bad-llama-abc-exploration.png",
    "bad-llama-c-plus-a-approved.png",
    "scatterers-rot-sticker-slime-bestiary.png",
    "floor01-bestiary-exploration-sheet.png",
    "floor01-enemy-lineup-approved.png",
    "floor01-hoarder-individual-study.png",
    "floor01-troglodyte-family-approved.png",
    "floor01-ball-of-swine-approved.png",
}


class Floor01EnemyConceptsTest(unittest.TestCase):
    def test_approved_reference_images_match_manifest(self):
        with MANIFEST.open(encoding="utf-8") as stream:
            manifest = json.load(stream)

        self.assertEqual(manifest["schema"], "the-crawl-concept-image-manifest-v1")
        entries = manifest["assets"]
        self.assertEqual(len(entries), len(EXPECTED_NAMES))
        self.assertEqual(
            {Path(entry["path"]).name for entry in entries}, EXPECTED_NAMES
        )
        self.assertEqual(
            {file.name for file in CONCEPT_DIR.glob("*.png")}, EXPECTED_NAMES
        )

        for entry in entries:
            with self.subTest(filename=entry["path"]):
                filename = Path(entry["path"])
                self.assertEqual(filename.parent, CONCEPT_DIR.relative_to(ROOT))
                full_path = ROOT / filename
                self.assertTrue(full_path.is_file())
                data = full_path.read_bytes()

                self.assertEqual(len(data), entry["bytes"])
                self.assertEqual(hashlib.sha256(data).hexdigest(), entry["sha256"])

                with Image.open(io.BytesIO(data)) as image:
                    self.assertEqual(image.format, "PNG")
                    self.assertEqual(
                        image.size, (entry["width"], entry["height"])
                    )
                    image.verify()


if __name__ == "__main__":
    unittest.main()
