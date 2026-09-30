"""Stage and smoke-test the private PDF.js worker inside native wheels."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import stat
import struct
import subprocess
import sys
import zipfile
from pathlib import Path, PureWindowsPath

ROOT = Path(__file__).resolve().parents[2]
PACKAGE_WORKER = Path("kordoc") / "_bin"
MAX_RESPONSE_BYTES = 4 * 1024 * 1024
HEADER_BYTES = 10
NOTICE_PREFIX = "kordoc/_licenses/pdfjs-v8/"
NOTICE_MANIFEST = Path("crates/kordoc-pdf/assets/v8-licenses/PROVENANCE.json")
REQUIRED_NOTICE_FILES = {
    ("crates/kordoc-pdf/assets/pdfjs/LICENSE", "pdfjs/LICENSE"),
    ("crates/kordoc-pdf/assets/v8-licenses/rusty_v8/LICENSE", "rusty_v8/LICENSE"),
    ("crates/kordoc-pdf/assets/v8-licenses/v8/LICENSE", "v8/LICENSE"),
    ("crates/kordoc-pdf/assets/v8-licenses/v8/LICENSE.v8", "v8/LICENSE.v8"),
}
EXPECTED_NOTICE_PATHS = frozenset(
    {
        "deps/abseil-cpp/LICENSE",
        "deps/dragonbox/LICENSE-Apache2-LLVM",
        "deps/dragonbox/LICENSE-Boost",
        "deps/fast_float/LICENSE-MIT",
        "deps/fp16/LICENSE",
        "deps/highway/LICENSE",
        "deps/icu/LICENSE",
        "deps/libcxx/LICENSE.TXT",
        "deps/libcxxabi/LICENSE.TXT",
        "deps/libunwind/LICENSE.TXT",
        "deps/llvm-libc/LICENSE.TXT",
        "deps/partition_alloc/LICENSE.chromium",
        "deps/simdutf/LICENSE",
        "pdfjs/LICENSE",
        "rusty_v8/LICENSE",
        "v8/LICENSE",
        "v8/LICENSE.v8",
        "v8/third_party/colorama/LICENSE",
        "v8/third_party/disarm/LICENSE",
        "v8/third_party/fadec/LICENSE",
        "v8/third_party/fp16/LICENSE",
        "v8/third_party/highway/LICENSE",
        "v8/third_party/inspector_protocol/LICENSE",
        "v8/third_party/jsoncpp/LICENSE",
        "v8/third_party/rapidhash-v8/LICENSE",
        "v8/third_party/re2/LICENSE",
        "v8/third_party/siphash/LICENSE",
        "v8/third_party/utf8-decoder/LICENSE",
        "v8/third_party/v8/builtins/LICENSE",
        "v8/third_party/v8/codegen/LICENSE",
        "v8/third_party/valgrind/LICENSE",
        "v8/third_party/vtune/LICENSE",
        "v8/third_party/wasm-api/LICENSE",
    }
)

TARGETS = {
    "x86_64-unknown-linux-gnu": ("elf", 62, "manylinux_2_28_x86_64", "pdfjs-worker"),
    "aarch64-unknown-linux-gnu": ("elf", 183, "manylinux_2_28_aarch64", "pdfjs-worker"),
    "x86_64-pc-windows-msvc": ("pe", 0x8664, "win_amd64", "pdfjs-worker.exe"),
    "aarch64-pc-windows-msvc": ("pe", 0xAA64, "win_arm64", "pdfjs-worker.exe"),
    "x86_64-apple-darwin": ("macho", 0x01000007, "x86_64", "pdfjs-worker"),
    "aarch64-apple-darwin": ("macho", 0x0100000C, "arm64", "pdfjs-worker"),
}


def _target_info(target: str) -> tuple[str, int, str, str]:
    try:
        return TARGETS[target]
    except KeyError as error:
        raise ValueError(f"unsupported worker target: {target}") from error


def _check_architecture(data: bytes, target: str) -> None:
    kind, expected_machine, _, _ = _target_info(target)
    if kind == "elf":
        if (
            len(data) < 20
            or data[:4] != b"\x7fELF"
            or data[4] != 2
            or data[5] not in (1, 2)
        ):
            raise ValueError("worker is not a valid 64-bit ELF executable")
        byte_order = "little" if data[5] == 1 else "big"
        machine = int.from_bytes(data[18:20], byte_order)
    elif kind == "pe":
        if len(data) < 64 or data[:2] != b"MZ":
            raise ValueError("worker is not a valid PE executable")
        pe_offset = int.from_bytes(data[60:64], "little")
        if pe_offset + 6 > len(data) or data[pe_offset : pe_offset + 4] != b"PE\0\0":
            raise ValueError("worker has an invalid PE header")
        machine = int.from_bytes(data[pe_offset + 4 : pe_offset + 6], "little")
    else:
        if len(data) < 8:
            raise ValueError("worker is not a valid 64-bit Mach-O executable")
        if data[:4] == b"\xcf\xfa\xed\xfe":
            byte_order = "little"
        elif data[:4] == b"\xfe\xed\xfa\xcf":
            byte_order = "big"
        else:
            raise ValueError("worker is not a thin 64-bit Mach-O executable")
        machine = int.from_bytes(data[4:8], byte_order)
    if machine != expected_machine:
        raise ValueError(
            f"worker architecture mismatch for {target}: got 0x{machine:x}, "
            f"expected 0x{expected_machine:x}"
        )


def _worker_name(target: str) -> str:
    return _target_info(target)[3]


def _notice_entries(manifest_path: Path, root: Path = ROOT) -> list[dict[str, str]]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if (
        not isinstance(manifest, dict)
        or manifest.get("schema_version") != 1
        or manifest.get("rusty_v8_tag") != "v152.2.0"
        or manifest.get("v8_submodule_commit")
        != "c4ca1eccb90c5464d826b7713dc27178a53b0cfe"
        or not isinstance(manifest.get("files"), list)
    ):
        raise ValueError(
            "V8 license provenance manifest has an unsupported schema or pin"
        )
    entries = manifest["files"]
    normalized: list[dict[str, str]] = []
    seen_sources: set[str] = set()
    seen_wheel_paths: set[str] = set()
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {
            "source",
            "wheel_path",
            "sha256",
            "upstream_url",
        }:
            raise ValueError("V8 license manifest file entry has invalid fields")
        source = entry["source"]
        wheel_path = entry["wheel_path"]
        digest = entry["sha256"]
        upstream_url = entry["upstream_url"]
        if not all(
            isinstance(value, str) and value
            for value in (source, wheel_path, digest, upstream_url)
        ):
            raise ValueError("V8 license manifest contains an empty or non-text field")
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError(f"invalid SHA-256 in license manifest for {source}")
        source_path = Path(source)
        if source_path.is_absolute() or ".." in source_path.parts:
            raise ValueError(f"unsafe license source path: {source}")
        source_resolved = (root / source_path).resolve(strict=True)
        try:
            source_resolved.relative_to(root.resolve())
        except ValueError as error:
            raise ValueError(
                f"license source escapes repository root: {source}"
            ) from error
        wheel = Path(wheel_path)
        windows_wheel = PureWindowsPath(wheel_path)
        if (
            wheel.is_absolute()
            or windows_wheel.is_absolute()
            or windows_wheel.drive
            or ".." in wheel.parts
            or "\\" in wheel_path
            or not wheel.parts
        ):
            raise ValueError(f"unsafe wheel notice path: {wheel_path}")
        if source in seen_sources or wheel_path in seen_wheel_paths:
            raise ValueError("duplicate source or wheel path in license manifest")
        seen_sources.add(source)
        seen_wheel_paths.add(wheel_path)
        _verified_notice_bytes(source_resolved, digest, source)
        normalized.append(
            {
                "source": source,
                "wheel_path": wheel_path,
                "sha256": digest,
                "upstream_url": upstream_url,
            }
        )
    pairs = {(entry["source"], entry["wheel_path"]) for entry in normalized}
    if not REQUIRED_NOTICE_FILES <= pairs:
        raise ValueError("V8 license manifest omits a required PDF.js/V8 notice")
    if seen_wheel_paths != EXPECTED_NOTICE_PATHS:
        missing = EXPECTED_NOTICE_PATHS - seen_wheel_paths
        extra = seen_wheel_paths - EXPECTED_NOTICE_PATHS
        raise ValueError(
            f"V8 license manifest inventory mismatch: missing={sorted(missing)}, "
            f"extra={sorted(extra)}"
        )
    return normalized


def _sha256(data: bytes) -> str:
    import hashlib

    return hashlib.sha256(data).hexdigest()


def _verified_notice_bytes(source: Path, digest: str, label: str) -> bytes:
    data = source.read_bytes()
    if _sha256(data) == digest:
        return data
    # Git may materialize text files with CRLF on Windows. Accept that checkout
    # representation only when converting CRLF back to LF restores the pinned bytes.
    canonical = data.replace(b"\r\n", b"\n")
    if canonical != data and _sha256(canonical) == digest:
        return canonical
    raise ValueError(f"source notice hash does not match manifest: {label}")


def stage_notices(
    manifest_path: Path = ROOT / NOTICE_MANIFEST, root: Path = ROOT
) -> None:
    entries = _notice_entries(manifest_path, root)
    destination_root = root / "python" / "kordoc" / "_licenses" / "pdfjs-v8"
    expected_paths = {entry["wheel_path"] for entry in entries}
    if destination_root.exists():
        existing_paths = {
            path.relative_to(destination_root).as_posix()
            for path in destination_root.rglob("*")
            if path.is_file()
        }
        extras = existing_paths - expected_paths
        if extras:
            raise ValueError(
                f"notice staging directory has unmanifested files: {sorted(extras)}"
            )
    for entry in entries:
        source = root / entry["source"]
        destination = destination_root / Path(entry["wheel_path"])
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(
            _verified_notice_bytes(source, entry["sha256"], entry["source"])
        )
        if _sha256(destination.read_bytes()) != entry["sha256"]:
            raise ValueError(f"staged notice hash mismatch: {entry['wheel_path']}")


def stage(
    target: str,
    binary: Path,
    manifest_path: Path = ROOT / NOTICE_MANIFEST,
) -> None:
    _, _, _, name = _target_info(target)
    source = binary.resolve(strict=True)
    data = source.read_bytes()
    _check_architecture(data, target)
    if os.name != "nt" and not source.stat().st_mode & (
        stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    ):
        raise ValueError(f"worker executable bit is missing: {source}")
    destination = ROOT / "python" / PACKAGE_WORKER / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    if os.name != "nt" and not destination.stat().st_mode & 0o111:
        raise ValueError(f"staged worker lost executable mode: {destination}")
    stage_notices(manifest_path)
    print(f"staged {target} worker at {destination.relative_to(ROOT)}")


def check_wheel(
    target: str,
    wheel: Path,
    manifest_path: Path = ROOT / NOTICE_MANIFEST,
    root: Path = ROOT,
) -> None:
    _, _, platform_tag, expected_name = _target_info(target)
    if "cp310-abi3" not in wheel.name:
        raise ValueError(f"wheel does not advertise the cp310 abi3 tag: {wheel.name}")
    if target.endswith(("-unknown-linux-gnu", "-pc-windows-msvc")):
        if not wheel.name.endswith(f"-{platform_tag}.whl"):
            raise ValueError(
                f"wheel platform tag does not match {target}: {wheel.name}"
            )
    elif f"_{platform_tag}.whl" not in wheel.name:
        raise ValueError(f"wheel platform tag does not match {target}: {wheel.name}")

    expected_path = (PACKAGE_WORKER / expected_name).as_posix()
    with zipfile.ZipFile(wheel) as archive:
        workers = [
            item
            for item in archive.infolist()
            if item.filename.startswith("kordoc/_bin/")
            and "pdfjs-worker" in item.filename
        ]
        if len(workers) != 1 or workers[0].filename != expected_path:
            raise ValueError(
                f"wheel must contain exactly {expected_path}; got "
                f"{[item.filename for item in workers]}"
            )
        info = workers[0]
        worker_data = archive.read(info)
        _check_architecture(worker_data, target)
        if os.name != "nt" and ((info.external_attr >> 16) & 0o111) == 0:
            raise ValueError("wheel worker does not preserve executable permissions")
        notice_entries = _notice_entries(manifest_path, root)
        expected_notices = {
            NOTICE_PREFIX + entry["wheel_path"]: entry["sha256"]
            for entry in notice_entries
        }
        actual_notice_names = [
            item.filename
            for item in archive.infolist()
            if item.filename.startswith(NOTICE_PREFIX) and not item.is_dir()
        ]
        if len(actual_notice_names) != len(set(actual_notice_names)):
            raise ValueError("wheel contains duplicate notice entries")
        if set(actual_notice_names) != set(expected_notices):
            raise ValueError(
                "wheel notice entries do not match the provenance manifest: "
                f"expected {sorted(expected_notices)}, got {sorted(set(actual_notice_names))}"
            )
        for name, expected_hash in expected_notices.items():
            if _sha256(archive.read(name)) != expected_hash:
                raise ValueError(f"wheel notice digest mismatch: {name}")
    print(f"validated {target} worker entry in {wheel.name}")


def _request_frame(pdf: bytes) -> bytes:
    if len(pdf) > 32 * 1024 * 1024:
        raise ValueError("smoke-test fixture exceeds the worker request cap")
    return b"KPDF\x01\x01" + struct.pack(">I", len(pdf)) + pdf


def _decode_response(frame: bytes) -> dict[str, object]:
    if len(frame) < HEADER_BYTES or frame[:4] != b"KPDF":
        raise ValueError("worker response has an invalid frame header")
    if frame[4] != 1 or frame[5] != 2:
        raise ValueError("worker response has an unsupported version or frame kind")
    payload_length = struct.unpack(">I", frame[6:10])[0]
    if payload_length > MAX_RESPONSE_BYTES:
        raise ValueError("worker response exceeds its 4 MiB cap")
    if len(frame) != HEADER_BYTES + payload_length:
        raise ValueError("worker response is truncated or has trailing bytes")
    try:
        payload = json.loads(frame[HEADER_BYTES:].decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("worker response is not valid UTF-8 JSON") from error
    if not isinstance(payload, dict):
        raise TypeError("worker response JSON must be an object")
    return payload


def _expect_probe(response: dict[str, object], expected_text: str) -> None:
    if response.get("status") != "success":
        raise ValueError(f"worker returned a non-success response: {response!r}")
    result = response.get("result")
    if not isinstance(result, dict):
        raise TypeError("worker success response is missing its result")
    if result.get("page_count") != 1 or result.get("page_text") != [expected_text]:
        raise ValueError(f"worker extracted unexpected PDF text: {result!r}")


def smoke(target: str, fixture: Path, expected_text: str) -> None:
    import kordoc

    _, _, _, name = _target_info(target)
    package_root = Path(kordoc.__file__).resolve().parent
    worker = package_root / "_bin" / name
    if not worker.is_file():
        raise ValueError(f"installed package is missing the adjacent worker: {worker}")
    _check_architecture(worker.read_bytes(), target)
    if os.name != "nt" and not os.access(worker, os.X_OK):
        raise ValueError("installed worker is not executable")
    notice_root = package_root / "_licenses" / "pdfjs-v8"
    entries = _notice_entries(ROOT / NOTICE_MANIFEST)
    expected_notice_paths = {entry["wheel_path"] for entry in entries}
    actual_notice_paths = {
        path.relative_to(notice_root).as_posix()
        for path in notice_root.rglob("*")
        if path.is_file()
    }
    if actual_notice_paths != expected_notice_paths:
        raise ValueError("installed package notice files do not match the manifest")
    for entry in entries:
        notice = notice_root / entry["wheel_path"]
        if _sha256(notice.read_bytes()) != entry["sha256"]:
            raise ValueError(f"installed notice digest mismatch: {entry['wheel_path']}")

    completed = subprocess.run(
        [str(worker)],
        input=_request_frame(fixture.read_bytes()),
        capture_output=True,
        timeout=30,
        check=False,
    )
    if completed.returncode != 0:
        raise ValueError(f"worker exited with status {completed.returncode}")
    response = _decode_response(completed.stdout)
    _expect_probe(response, expected_text)
    print(f"installed {target} worker extracted {expected_text!r}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    stage_parser = commands.add_parser("stage")
    stage_parser.add_argument("--target", required=True, choices=TARGETS)
    stage_parser.add_argument("--binary", type=Path, required=True)
    stage_parser.add_argument("--manifest", type=Path, default=ROOT / NOTICE_MANIFEST)
    wheel_parser = commands.add_parser("check-wheel")
    wheel_parser.add_argument("--target", required=True, choices=TARGETS)
    wheel_parser.add_argument("--wheel", type=Path, required=True)
    wheel_parser.add_argument("--manifest", type=Path, default=ROOT / NOTICE_MANIFEST)
    smoke_parser = commands.add_parser("smoke")
    smoke_parser.add_argument("--target", required=True, choices=TARGETS)
    smoke_parser.add_argument("--fixture", type=Path, required=True)
    smoke_parser.add_argument("--expected-text", required=True)
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "stage":
            stage(arguments.target, arguments.binary, arguments.manifest)
        elif arguments.command == "check-wheel":
            check_wheel(arguments.target, arguments.wheel, arguments.manifest)
        else:
            smoke(arguments.target, arguments.fixture, arguments.expected_text)
    except (
        OSError,
        ValueError,
        subprocess.SubprocessError,
        zipfile.BadZipFile,
    ) as error:
        print(f"pdf worker wheel check failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
