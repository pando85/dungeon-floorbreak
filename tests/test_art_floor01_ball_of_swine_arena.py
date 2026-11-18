"""Pin the four original Ball of Swine standalone arena concept PNGs.

These are art-direction references, not production sprite sources. The exact
original PNG bytes must remain unchanged across future revisions.
"""

import hashlib
import io
import json
from pathlib import Path
import unittest

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
CONCEPT_DIR = ROOT / "assets/concept/floor-01/ball-of-swine-arena"
MANIFEST = CONCEPT_DIR / "manifest.json"
EXPECTED_SHA256 = {
    "01-arena-overview.png":
        "e3877c1afeea57567f9618881b5a6293fabbdb7aed6527a046e001c477c453b1",
    "02-first-impact.png":
        "67b48302b6430a9e000956c4f1f300f7f6e5786a30f1c455e87f12fa2c2b1866",
    "03-space-usage-exploration.png":
        "18dab811c314c31271fc333515627f5fae9b7ad23e9b566cba64f2221d436865",
    "04-gameplay-layout-preferred.png":
        "746e3c0d96b3560a5cbd1fe88c141d3a4872203189b8aeb5cd22250088a74492",
}


class BallOfSwineArenaConceptTest(unittest.TestCase):
    def test_original_images_match_pinned_manifest_and_bytes(self):
        with MANIFEST.open(encoding="utf-8") as file:
            manifest = json.load(file)

        self.assertEqual(
            manifest["schema"], "the-crawl-concept-image-manifest-v1"
        )
        entries = manifest["assets"]
        self.assertEqual(len(entries), len(EXPECTED_SHA256))
        self.assertEqual(
            {Path(entry["path"]).name for entry in entries},
            set(EXPECTED_SHA256),
        )
        self.assertEqual(
            {file.name for file in CONCEPT_DIR.glob("*.png")},
            set(EXPECTED_SHA256),
        )

        for entry in entries:
            with self.subTest(filename=entry["path"]):
                relative = Path(entry["path"])
                self.assertEqual(relative.parent, CONCEPT_DIR.relative_to(ROOT))
                self.assertEqual(
                    entry["sha256"], EXPECTED_SHA256[relative.name]
                )
                blob = (ROOT / relative).read_bytes()
                self.assertEqual(len(blob), entry["bytes"])
                self.assertEqual(
                    hashlib.sha256(blob).hexdigest(),
                    EXPECTED_SHA256[relative.name],
                )
                with Image.open(io.BytesIO(blob)) as image:
                    self.assertEqual(image.format, "PNG")
                    self.assertEqual(
                        image.size, (entry["width"], entry["height"])
                    )
                    image.verify()


if __name__ == "__main__":
    unittest.main()
