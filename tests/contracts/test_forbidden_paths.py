import subprocess
from pathlib import Path

ROOT = Path(__file__).parents[2]


def test_kordoc_oracle_is_not_tracked() -> None:
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    assert not any(path.startswith(b"kordoc/") for path in tracked.split(b"\0"))
