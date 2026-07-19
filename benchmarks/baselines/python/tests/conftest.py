from __future__ import annotations

from pathlib import Path

import pytest


@pytest.fixture(scope="session")
def repository_root() -> Path:
    current = Path(__file__).resolve()
    for candidate in current.parents:
        if (candidate / ".git").is_dir():
            return candidate
    pytest.fail("tests are not inside the repository")
