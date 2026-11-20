import pytest
from gymnasium.utils.env_checker import check_env

from autoplay import AutoplayEnv, scenario_roster


@pytest.mark.parametrize("scenario", [sid for sid, _ in scenario_roster()])
def test_check_env_passes_for_every_roster_scenario(scenario):
    check_env(AutoplayEnv(scenario), skip_render_check=True)
