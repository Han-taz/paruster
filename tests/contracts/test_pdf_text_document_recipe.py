from __future__ import annotations

import hashlib
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
FIXTURE_ROOT = ROOT / "crates/kordoc-pdf/tests/fixtures/pdfjs_text_document"


def test_text_document_pdf_recipe_reproduces_pinned_bytes_offline(
    tmp_path: Path,
) -> None:
    recipe = FIXTURE_ROOT / "generate.py"
    assert hashlib.sha256(recipe.read_bytes()).hexdigest() == (
        "25047936b3aa20d8fbb87f503164d627bc75c60862f4dc389b5825a027e4f993"
    )
    copied = tmp_path / recipe.name
    shutil.copyfile(recipe, copied)
    output = tmp_path / "document.pdf"
    subprocess.run(
        [sys.executable, str(copied), "--output", str(output)],
        check=True,
        capture_output=True,
    )
    generated = output.read_bytes()
    assert generated == (FIXTURE_ROOT / "document.pdf").read_bytes()
    assert len(generated) == 2256
    assert hashlib.sha256(generated).hexdigest() == (
        "6420221a8fbbfe4386caa18705fe79b3f450ec5893852297461fa793522c4a6b"
    )


def test_complete_text_document_expected_result_retains_its_pinned_bytes() -> None:
    expected = (FIXTURE_ROOT / "expected.json").read_bytes()
    assert len(expected) == 1764
    assert hashlib.sha256(expected).hexdigest() == (
        "24da296d2ec1b231d55c3171530c79bcb9a5847283428ed8dee469670f331f7e"
    )
