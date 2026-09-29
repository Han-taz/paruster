"""Inspect built wheels and sdists without extracting their contents."""

from __future__ import annotations

import argparse
import stat
import sys
import tarfile
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath, PureWindowsPath

MAX_ARCHIVE_BYTES = 1024 * 1024 * 1024
MAX_ARCHIVE_MEMBERS = 50_000
MAX_MEMBER_NAME_BYTES = 4096


@dataclass(frozen=True)
class ArchiveMember:
    name: str
    is_file: bool
    link_target: str | None = None


def _unsafe_path(name: str) -> str | None:
    normalized = name.replace("\\", "/")
    windows = PureWindowsPath(name)
    path = PurePosixPath(normalized)
    if (
        len(name.encode("utf-8", errors="surrogatepass")) > MAX_MEMBER_NAME_BYTES
        or "\0" in name
        or path.is_absolute()
        or windows.is_absolute()
        or windows.drive
    ):
        return "absolute or invalid archive path"
    if any(part == ".." for part in path.parts):
        return "parent traversal in archive path"
    return None


def _member_violation(member: ArchiveMember) -> str | None:
    unsafe = _unsafe_path(member.name)
    if unsafe:
        return f"{unsafe}: {member.name!r}"
    if member.link_target is not None:
        target_problem = _unsafe_path(member.link_target)
        if target_problem:
            return (
                f"unsafe archive link target {member.link_target!r} for {member.name!r}"
            )

    parts = PurePosixPath(member.name.replace("\\", "/")).parts
    lowered = tuple(part.casefold() for part in parts)
    if "node_modules" in lowered:
        return f"node_modules member is forbidden: {member.name!r}"
    if member.name.casefold().endswith(".ts"):
        return f"TypeScript member is forbidden: {member.name!r}"
    for index in range(len(lowered) - 1):
        if lowered[index : index + 2] == ("kordoc", "src"):
            return f"oracle source layout is forbidden: {member.name!r}"
    if len(lowered) >= 2 and lowered[-2:] in (
        ("kordoc", "package.json"),
        ("kordoc", "package-lock.json"),
    ):
        return f"oracle package manifest is forbidden: {member.name!r}"
    return None


def _read_zip(path: Path) -> list[ArchiveMember]:
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        if len(infos) > MAX_ARCHIVE_MEMBERS:
            raise ValueError(f"archive exceeds {MAX_ARCHIVE_MEMBERS} members")
        members = []
        for item in infos:
            file_type = stat.S_IFMT(item.external_attr >> 16)
            members.append(
                ArchiveMember(
                    item.filename,
                    file_type in (0, stat.S_IFREG) and not item.is_dir(),
                )
            )
        return members


def _read_tar(path: Path) -> list[ArchiveMember]:
    members: list[ArchiveMember] = []
    with tarfile.open(path, mode="r:*") as archive:
        for item in archive:
            if len(members) >= MAX_ARCHIVE_MEMBERS:
                raise ValueError(f"archive exceeds {MAX_ARCHIVE_MEMBERS} members")
            target = item.linkname if item.issym() or item.islnk() else None
            members.append(ArchiveMember(item.name, item.isfile(), target))
    return members


def _validate_required(members: list[ArchiveMember], kind: str) -> list[str]:
    names = [member.name.replace("\\", "/") for member in members if member.is_file]
    lower_names = [name.casefold() for name in names]
    problems: list[str] = []
    if kind == "wheel":
        if not any(name.endswith(".dist-info/metadata") for name in lower_names):
            problems.append("wheel is missing .dist-info/METADATA")
        if not any(Path(name).name.casefold().startswith("license") for name in names):
            problems.append("wheel is missing packaged license")
        native_suffixes = (".so", ".pyd", ".dylib", ".dll")
        if not any(
            PurePosixPath(name).parts
            and PurePosixPath(name).parts[0].casefold() == "kordoc"
            and PurePosixPath(name).name.casefold().startswith("_native.")
            and PurePosixPath(name).name.casefold().endswith(native_suffixes)
            for name in names
        ):
            problems.append("wheel is missing kordoc native extension")
    else:
        if not any(Path(name).name.casefold() == "license" for name in names):
            problems.append("sdist is missing LICENSE")
        if not any(Path(name).name.casefold() == "pyproject.toml" for name in names):
            problems.append("sdist is missing pyproject.toml")
    return problems


def inspect_artifact(path: Path) -> list[str]:
    """Return validation errors for a wheel or source distribution."""
    try:
        if path.stat().st_size > MAX_ARCHIVE_BYTES:
            return [f"archive exceeds {MAX_ARCHIVE_BYTES} byte limit"]
        if path.name.endswith(".whl"):
            kind = "wheel"
            members = _read_zip(path)
        elif path.name.endswith((".tar.gz", ".tgz", ".tar")):
            kind = "sdist"
            members = _read_tar(path)
        else:
            return ["unsupported artifact type (expected .whl, .tar.gz, .tgz, or .tar)"]
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        return [f"cannot read archive: {error}"]

    problems = [problem for member in members if (problem := _member_violation(member))]
    problems.extend(_validate_required(members, kind))
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", nargs="+", type=Path)
    args = parser.parse_args(argv)

    errors = 0
    for artifact in args.artifacts:
        problems = inspect_artifact(artifact)
        for problem in problems:
            print(f"{artifact}: {problem}", file=sys.stderr)
            errors += 1
    if errors:
        return 1
    print(f"validated {len(args.artifacts)} artifact(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
