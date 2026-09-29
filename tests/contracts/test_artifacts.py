from __future__ import annotations

import io
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

import pytest

ROOT = Path(__file__).parents[2]
CHECKER = ROOT / "scripts" / "check_artifacts.py"


def _oracle_member(suffix: str) -> str:
    return f"{'kor' + 'doc'}/{suffix}"


def _wheel(path: Path, extra: tuple[str, ...] = (), *, omit: str | None = None) -> None:
    members = {
        "kordoc/__init__.py": b"",
        "kordoc/_native.abi3.so": b"native extension placeholder",
        "kordoc-1.0.dist-info/METADATA": b"Metadata-Version: 2.1\nName: kordoc\nVersion: 1.0\n",
        "kordoc-1.0.dist-info/licenses/LICENSE": b"license",
    }
    for required in ("metadata", "native", "license"):
        if omit == required:
            members.pop(
                {
                    "metadata": "kordoc-1.0.dist-info/METADATA",
                    "native": "kordoc/_native.abi3.so",
                    "license": "kordoc-1.0.dist-info/licenses/LICENSE",
                }[required]
            )
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in members.items():
            archive.writestr(name, content)
        for name in extra:
            archive.writestr(name, b"" if name.endswith("/") else b"bad member")


def _sdist(path: Path, extra: tuple[str, ...] = (), *, omit: str | None = None) -> None:
    members = {
        "kordoc-1.0/LICENSE": b"license",
        "kordoc-1.0/pyproject.toml": b"[build-system]\n",
        "kordoc-1.0/python/kordoc/__init__.py": b"",
    }
    if omit is not None:
        members.pop(f"kordoc-1.0/{omit}")
    with tarfile.open(path, "w:gz") as archive:
        for name, content in members.items():
            info = tarfile.TarInfo(name)
            info.size = len(content)
            archive.addfile(info, io.BytesIO(content))
        for name in extra:
            info = tarfile.TarInfo(name)
            if name.endswith("/"):
                info.type = tarfile.DIRTYPE
                archive.addfile(info)
            else:
                info.size = 3
                archive.addfile(info, io.BytesIO(b"bad"))


def _run(*artifacts: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(CHECKER), *(str(path) for path in artifacts)],
        check=False,
        capture_output=True,
        text=True,
    )


def test_accepts_valid_wheel_and_sdist_with_python_kordoc_members(
    tmp_path: Path,
) -> None:
    wheel = tmp_path / "kordoc-1.0-py3-none-any.whl"
    sdist = tmp_path / "kordoc-1.0.tar.gz"
    _wheel(wheel)
    _sdist(sdist)

    result = _run(wheel, sdist)

    assert result.returncode == 0, result.stderr


@pytest.mark.parametrize(
    "member",
    [
        "/absolute/file",
        "../outside/file",
        "C:\\outside\\file",
        "..\\outside\\file",
        "src/node_modules/pkg/index.js",
        "src/parser.ts",
        _oracle_member("src/index.js"),
        _oracle_member("package.json"),
        _oracle_member("package-lock.json"),
    ],
)
@pytest.mark.parametrize("kind", ["wheel", "sdist"])
def test_rejects_unsafe_or_oracle_members_in_both_archives(
    tmp_path: Path, member: str, kind: str
) -> None:
    artifact = tmp_path / (
        "kordoc-1.0-py3-none-any.whl" if kind == "wheel" else "kordoc-1.0.tar.gz"
    )
    if kind == "wheel":
        _wheel(artifact, (member,))
    else:
        archive_name = (
            member
            if member.startswith("/") or (len(member) > 1 and member[1] == ":")
            else f"kordoc-1.0/{member}"
        )
        _sdist(artifact, (archive_name,))

    result = _run(artifact)

    assert result.returncode != 0
    if "\\" in member:
        reason = (
            "parent traversal" if member.startswith("..") else "absolute or invalid"
        )
        assert reason in result.stderr
    else:
        assert member in result.stderr


@pytest.mark.parametrize("missing", ["metadata", "native", "license"])
def test_rejects_wheels_missing_required_payload(tmp_path: Path, missing: str) -> None:
    wheel = tmp_path / "kordoc-1.0-py3-none-any.whl"
    _wheel(wheel, omit=missing)

    result = _run(wheel)

    assert result.returncode != 0
    assert missing in result.stderr.lower()


@pytest.mark.parametrize("missing", ["LICENSE", "pyproject.toml"])
def test_rejects_sdists_missing_required_payload(tmp_path: Path, missing: str) -> None:
    sdist = tmp_path / "kordoc-1.0.tar.gz"
    _sdist(sdist, omit=missing)

    result = _run(sdist)

    assert result.returncode != 0
    assert missing in result.stderr


@pytest.mark.parametrize(
    ("kind", "missing", "directory_member", "diagnostic"),
    [
        ("wheel", "metadata", "kordoc-1.0.dist-info/METADATA/", "METADATA"),
        ("wheel", "native", "kordoc/_native.abi3.so/", "native extension"),
        (
            "wheel",
            "license",
            "kordoc-1.0.dist-info/licenses/LICENSE/",
            "license",
        ),
        ("sdist", "LICENSE", "kordoc-1.0/LICENSE/", "LICENSE"),
        (
            "sdist",
            "pyproject.toml",
            "kordoc-1.0/pyproject.toml/",
            "pyproject.toml",
        ),
    ],
)
def test_required_payload_directories_do_not_satisfy_file_requirements(
    tmp_path: Path, kind: str, missing: str, directory_member: str, diagnostic: str
) -> None:
    if kind == "wheel":
        artifact = tmp_path / "kordoc-1.0-py3-none-any.whl"
        _wheel(artifact, (directory_member,), omit=missing)
    else:
        artifact = tmp_path / "kordoc-1.0.tar.gz"
        _sdist(artifact, (directory_member,), omit=missing)

    result = _run(artifact)

    assert result.returncode != 0
    assert diagnostic in result.stderr


def test_rejects_tar_link_that_escapes_archive_root(tmp_path: Path) -> None:
    sdist = tmp_path / "kordoc-1.0.tar.gz"
    _sdist(sdist)
    with tarfile.open(sdist, "w:gz") as archive:
        for name, content in (
            ("kordoc-1.0/LICENSE", b"license"),
            ("kordoc-1.0/pyproject.toml", b"[build-system]\n"),
            ("kordoc-1.0/python/kordoc/__init__.py", b""),
        ):
            info = tarfile.TarInfo(name)
            info.size = len(content)
            archive.addfile(info, io.BytesIO(content))
        link = tarfile.TarInfo("kordoc-1.0/python/kordoc/escape")
        link.type = tarfile.SYMTYPE
        link.linkname = "../../../../outside"
        archive.addfile(link)

    result = _run(sdist)

    assert result.returncode != 0
    assert "outside" in result.stderr
