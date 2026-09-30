from __future__ import annotations

import hashlib
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
FIXTURE_ROOT = ROOT / "crates/kordoc-pdf/tests/fixtures/pdfjs_resource_probe"


def test_pdf_resource_probe_recipe_reproduces_pinned_bytes_offline(
    tmp_path: Path,
) -> None:
    recipe = FIXTURE_ROOT / "generate_resource_probe.py"
    assert hashlib.sha256(recipe.read_bytes()).hexdigest() == (
        "74a276bab8781abfec8707cf8d7cb68c82a6a3a67cbeaee9c8ad786d0c646151"
    )
    copied = tmp_path / recipe.name
    shutil.copyfile(recipe, copied)
    subprocess.run([sys.executable, str(copied)], check=True, capture_output=True)
    generated = (tmp_path / "resource_probe.pdf").read_bytes()
    assert generated == (FIXTURE_ROOT / "resource_probe.pdf").read_bytes()
    assert len(generated) == 1644
    assert hashlib.sha256(generated).hexdigest() == (
        "62601a0563488196397e88773eadfd7e2259c56fa33777a797ce958045a72679"
    )
