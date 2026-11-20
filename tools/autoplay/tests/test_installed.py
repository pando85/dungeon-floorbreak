"""Run from outside the source tree against the installed wheel."""

import os
from pathlib import Path

import pytest

import autoplay


@pytest.mark.skipif(
    os.environ.get("AUTOPLAY_TEST_INSTALLED") != "1",
    reason="set AUTOPLAY_TEST_INSTALLED=1 to require a non-editable installed package",
)
def test_package_comes_from_site_packages_not_the_source_tree():
    location = Path(autoplay.__file__).resolve()
    assert "site-packages" in location.parts, location
    assert not Path.cwd().resolve().is_relative_to(Path(__file__).resolve().parents[1])
