from __future__ import annotations

import hashlib
import json
import os
import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

import pdf_worker_wheel


class ArchitectureTests(unittest.TestCase):
    def test_elf_architecture_is_checked(self) -> None:
        image = bytearray(20)
        image[:4] = b"\x7fELF"
        image[4:6] = b"\x02\x01"
        image[18:20] = (183).to_bytes(2, "little")
        pdf_worker_wheel._check_architecture(bytes(image), "aarch64-unknown-linux-gnu")
        with self.assertRaisesRegex(ValueError, "architecture mismatch"):
            pdf_worker_wheel._check_architecture(
                bytes(image), "x86_64-unknown-linux-gnu"
            )

    def test_elf_rejects_32_bit_image(self) -> None:
        image = bytearray(20)
        image[:6] = b"\x7fELF\x01\x01"
        image[18:20] = (62).to_bytes(2, "little")
        with self.assertRaisesRegex(ValueError, "64-bit ELF"):
            pdf_worker_wheel._check_architecture(
                bytes(image), "x86_64-unknown-linux-gnu"
            )

    def test_pe_machine_is_checked(self) -> None:
        image = bytearray(70)
        image[:2] = b"MZ"
        image[60:64] = (64).to_bytes(4, "little")
        image[64:68] = b"PE\0\0"
        image[68:70] = (0xAA64).to_bytes(2, "little")
        pdf_worker_wheel._check_architecture(bytes(image), "aarch64-pc-windows-msvc")
        with self.assertRaisesRegex(ValueError, "architecture mismatch"):
            pdf_worker_wheel._check_architecture(bytes(image), "x86_64-pc-windows-msvc")

    def test_macho_cputype_is_checked(self) -> None:
        image = b"\xcf\xfa\xed\xfe" + (0x0100000C).to_bytes(4, "little")
        pdf_worker_wheel._check_architecture(bytes(image), "aarch64-apple-darwin")
        with self.assertRaisesRegex(ValueError, "architecture mismatch"):
            pdf_worker_wheel._check_architecture(bytes(image), "x86_64-apple-darwin")


class ProtocolSmokeTests(unittest.TestCase):
    def test_unicode_smoke_output_is_utf8_under_isolated_mode(self) -> None:
        helper = pdf_worker_wheel.ROOT / "tests/support/pdf_worker_wheel.py"
        script = (
            "import runpy,sys; "
            "helper=runpy.run_path(sys.argv[1]); "
            "print(helper['_smoke_success_message'](sys.argv[2],sys.argv[3]))"
        )
        completed = subprocess.run(
            [
                sys.executable,
                "-I",
                "-X",
                "utf8",
                "-c",
                script,
                str(helper),
                "aarch64-pc-windows-msvc",
                "한글🧪",
            ],
            capture_output=True,
            check=True,
        )
        self.assertEqual(
            completed.stdout,
            (
                "installed aarch64-pc-windows-msvc worker extracted '한글🧪'"
                + os.linesep
            ).encode("utf-8"),
        )

    def test_success_frame_uses_rust_snake_case_probe_fields(self) -> None:
        payload = json.dumps(
            {"status": "success", "result": {"page_count": 1, "page_text": ["한글🧪"]}},
            ensure_ascii=False,
        ).encode("utf-8")
        frame = b"KPDF\x01\x02" + struct.pack(">I", len(payload)) + payload
        response = pdf_worker_wheel._decode_response(frame)
        pdf_worker_wheel._expect_probe(response, "한글🧪")

    def test_bad_frame_length_and_oversized_payload_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "truncated or has trailing"):
            pdf_worker_wheel._decode_response(b"KPDF\x01\x02\x00\x00\x00\x02{}x")
        oversized = b"KPDF\x01\x02" + struct.pack(">I", 4 * 1024 * 1024 + 1)
        with self.assertRaisesRegex(ValueError, "4 MiB cap"):
            pdf_worker_wheel._decode_response(oversized)


class TextDocumentSmokeTests(unittest.TestCase):
    def _document(self) -> dict[str, object]:
        return {
            "page_count": 1,
            "metadata": {
                "title": "  한글🧪  ",
                "author": None,
                "creator": "",
                "subject": None,
                "keywords": "alpha; beta",
                "creation_date": "D:20250930123456Z",
                "modified_date": None,
            },
            "pages": [
                {
                    "page_number": 1,
                    "view_box": [100.0, 200.0, 500.0, 700.0],
                    "rotation": 90,
                    "items": [
                        {
                            "text": "한글🧪",
                            "width": 24.0,
                            "height": 12.0,
                            "transform": [12.0, 0.0, 0.0, 12.0, 150.0, 650.0],
                            "font_name": "g_d0_f1",
                        }
                    ],
                }
            ],
        }

    def test_new_request_has_explicit_kind_three_and_preserves_old_bytes(self) -> None:
        pdf = b"%PDF-1.7\n"
        suffix = struct.pack(">I", len(pdf)) + pdf
        self.assertEqual(pdf_worker_wheel._request_frame(pdf), b"KPDF\x01\x01" + suffix)
        self.assertEqual(
            pdf_worker_wheel._text_document_request_frame(pdf), b"KPDF\x01\x03" + suffix
        )

    def test_new_response_requires_kind_four_and_exact_metadata_and_geometry(
        self,
    ) -> None:
        expected = self._document()
        response = {"status": "success", "result": expected}
        payload = json.dumps(response, ensure_ascii=False).encode("utf-8")
        frame = b"KPDF\x01\x04" + struct.pack(">I", len(payload)) + payload
        actual = pdf_worker_wheel._decode_text_document_response(frame)
        pdf_worker_wheel._expect_text_document(actual, expected)
        with self.assertRaisesRegex(ValueError, "frame kind"):
            pdf_worker_wheel._decode_response(frame)
        with self.assertRaisesRegex(ValueError, "frame kind"):
            pdf_worker_wheel._decode_text_document_response(
                frame[:5] + b"\x02" + frame[6:]
            )
        with self.assertRaisesRegex(ValueError, "truncated or has trailing"):
            pdf_worker_wheel._decode_text_document_response(frame + b"x")
        with self.assertRaisesRegex(ValueError, "4 MiB cap"):
            pdf_worker_wheel._decode_text_document_response(
                b"KPDF\x01\x04"
                + struct.pack(">I", pdf_worker_wheel.MAX_RESPONSE_BYTES + 1)
            )

    def test_exact_comparison_rejects_extra_fields_changed_values_and_boolean_counts(
        self,
    ) -> None:
        expected = self._document()
        for changed in (
            {**expected, "page_count": True},
            {**expected, "page_count": 2},
            {**expected, "unexpected": "ignored"},
            {**expected, "metadata": {"title": "한글🧪"}},
        ):
            with (
                self.subTest(changed=changed),
                self.assertRaisesRegex(ValueError, "unexpected"),
            ):
                pdf_worker_wheel._expect_text_document(
                    {"status": "success", "result": changed}, expected
                )

    def test_comparison_rejects_nonfinite_values_and_failure_envelopes(self) -> None:
        expected = self._document()
        for changed in (
            {"status": "failure", "error": {"code": "PARSE_ERROR"}},
            {"status": "success", "result": {**expected, "page_count": float("nan")}},
            {"status": "success", "result": expected, "unexpected": 1},
        ):
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                pdf_worker_wheel._expect_text_document(changed, expected)


class NoticeBundleTests(unittest.TestCase):
    def _manifest(self, root: Path) -> tuple[Path, dict[str, bytes]]:
        required_sources = {
            "pdfjs/cmaps/LICENSE": "crates/kordoc-pdf/assets/pdfjs/cmaps/LICENSE",
            "pdfjs/standard_fonts/LICENSE_FOXIT": "crates/kordoc-pdf/assets/pdfjs/standard_fonts/LICENSE_FOXIT",
            "pdfjs/standard_fonts/LICENSE_LIBERATION": "crates/kordoc-pdf/assets/pdfjs/standard_fonts/LICENSE_LIBERATION",
            "pdfjs/LICENSE": "crates/kordoc-pdf/assets/pdfjs/LICENSE",
            "rusty_v8/LICENSE": "crates/kordoc-pdf/assets/v8-licenses/rusty_v8/LICENSE",
            "v8/LICENSE": "crates/kordoc-pdf/assets/v8-licenses/v8/LICENSE",
            "v8/LICENSE.v8": "crates/kordoc-pdf/assets/v8-licenses/v8/LICENSE.v8",
        }
        entries = []
        expected: dict[str, bytes] = {}
        for index, wheel_path in enumerate(
            sorted(pdf_worker_wheel.EXPECTED_NOTICE_PATHS)
        ):
            source = required_sources.get(
                wheel_path,
                f"crates/kordoc-pdf/assets/v8-licenses/test-notices/license-{index}.txt",
            )
            data = f"notice fixture {wheel_path}\n".encode()
            source_path = root / source
            source_path.parent.mkdir(parents=True, exist_ok=True)
            source_path.write_bytes(data)
            expected[wheel_path] = data
            entries.append(
                {
                    "source": source,
                    "wheel_path": wheel_path,
                    "sha256": hashlib.sha256(data).hexdigest(),
                    "upstream_url": "https://example.invalid/pinned-notice",
                }
            )
        manifest_path = root / "crates/kordoc-pdf/assets/v8-licenses/PROVENANCE.json"
        manifest_path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "rusty_v8_tag": "v152.2.0",
                    "v8_submodule_commit": "c4ca1eccb90c5464d826b7713dc27178a53b0cfe",
                    "files": entries,
                }
            ),
            encoding="utf-8",
        )
        return manifest_path, expected

    def test_checked_in_manifest_has_independently_pinned_inventory(self) -> None:
        manifest = pdf_worker_wheel._notice_entries(
            pdf_worker_wheel.ROOT / pdf_worker_wheel.NOTICE_MANIFEST
        )
        self.assertEqual(
            {entry["wheel_path"] for entry in manifest},
            pdf_worker_wheel.EXPECTED_NOTICE_PATHS,
        )

    def test_pinned_sources_and_probe_pdfs_disable_git_text_conversion(self) -> None:
        manifest = pdf_worker_wheel._notice_entries(
            pdf_worker_wheel.ROOT / pdf_worker_wheel.NOTICE_MANIFEST
        )
        paths = sorted(
            {entry["source"] for entry in manifest}
            | {
                "crates/kordoc-pdf/tests/fixtures/pdfjs_probe/one_page_helvetica.pdf",
                "crates/kordoc-pdf/tests/fixtures/unicode_probe/unicode_probe.pdf",
                "tests/golden/fixtures/minimal.pdf",
            }
        )
        result = subprocess.run(
            ["git", "check-attr", "--stdin", "text"],
            cwd=pdf_worker_wheel.ROOT,
            input=("\n".join(paths) + "\n").encode(),
            capture_output=True,
            check=True,
        )
        lines = result.stdout.decode("utf-8").splitlines()
        self.assertEqual(len(lines), len(paths))
        for path, line in zip(paths, lines):
            self.assertEqual(line, f"{path}: text: unset")

    def test_manifest_rejects_each_missing_resource_notice(self) -> None:
        for path in (
            "pdfjs/cmaps/LICENSE",
            "pdfjs/standard_fonts/LICENSE_FOXIT",
            "pdfjs/standard_fonts/LICENSE_LIBERATION",
        ):
            with self.subTest(path=path), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                manifest, _ = self._manifest(root)
                data = json.loads(manifest.read_text(encoding="utf-8"))
                data["files"] = [
                    entry for entry in data["files"] if entry["wheel_path"] != path
                ]
                manifest.write_text(json.dumps(data), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "omits a required"):
                    pdf_worker_wheel._notice_entries(manifest, root)

    def test_manifest_rejects_missing_inventory_entry(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, _ = self._manifest(root)
            data = json.loads(manifest.read_text(encoding="utf-8"))
            data["files"].pop()
            manifest.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "inventory mismatch"):
                pdf_worker_wheel._notice_entries(manifest, root)

    def test_manifest_stages_only_verified_notices(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, expected = self._manifest(root)
            pdf_worker_wheel.stage_notices(manifest, root)
            destination = root / "python/kordoc/_licenses/pdfjs-v8"
            actual = {
                path.relative_to(destination).as_posix(): path.read_bytes()
                for path in destination.rglob("*")
                if path.is_file()
            }
            self.assertEqual(actual, expected)

    def test_manifest_normalizes_windows_checkout_line_endings(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, expected = self._manifest(root)
            manifest_data = json.loads(manifest.read_text(encoding="utf-8"))
            entry = next(
                item
                for item in manifest_data["files"]
                if item["source"].endswith("test-notices/license-0.txt")
            )
            source = root / entry["source"]
            original = source.read_bytes()
            source.write_bytes(original.replace(b"\n", b"\r\n"))

            pdf_worker_wheel.stage_notices(manifest, root)

            destination = (
                root / "python/kordoc/_licenses/pdfjs-v8" / entry["wheel_path"]
            )
            self.assertEqual(destination.read_bytes(), expected[entry["wheel_path"]])

    def test_manifest_rejects_tampering_with_crlf_checkout(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, _ = self._manifest(root)
            source = root / "crates/kordoc-pdf/assets/pdfjs/LICENSE"
            source.write_bytes(
                source.read_bytes().replace(b"\n", b"\r\n") + b"tampered"
            )
            with self.assertRaisesRegex(ValueError, "source notice hash"):
                pdf_worker_wheel.stage_notices(manifest, root)

    def test_manifest_rejects_modified_source_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, _ = self._manifest(root)
            source = root / "crates/kordoc-pdf/assets/pdfjs/LICENSE"
            source.write_bytes(b"tampered license\n")
            with self.assertRaisesRegex(ValueError, "source notice hash"):
                pdf_worker_wheel.stage_notices(manifest, root)

    def test_manifest_rejects_traversal_and_duplicate_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, _ = self._manifest(root)
            data = json.loads(manifest.read_text(encoding="utf-8"))
            data["files"][0]["wheel_path"] = "../outside"
            manifest.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "unsafe wheel notice path"):
                pdf_worker_wheel._notice_entries(manifest, root)

            data = json.loads(manifest.read_text(encoding="utf-8"))
            data["files"][0]["wheel_path"] = data["files"][1]["wheel_path"]
            manifest.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate source or wheel path"):
                pdf_worker_wheel._notice_entries(manifest, root)

    def test_wheel_notice_members_and_digests_match_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, expected = self._manifest(root)
            wheel = root / "kordoc-0.1.0-cp310-abi3-macosx_11_0_arm64.whl"
            worker = b"\xcf\xfa\xed\xfe" + (0x0100000C).to_bytes(4, "little")
            with zipfile.ZipFile(wheel, "w") as archive:
                info = zipfile.ZipInfo("kordoc/_bin/pdfjs-worker")
                info.external_attr = 0o100755 << 16
                archive.writestr(info, worker)
                for path, data in expected.items():
                    archive.writestr(f"kordoc/_licenses/pdfjs-v8/{path}", data)
            pdf_worker_wheel.check_wheel("aarch64-apple-darwin", wheel, manifest, root)

    def test_wheel_rejects_extra_or_modified_notice(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, expected = self._manifest(root)
            wheel = root / "kordoc-0.1.0-cp310-abi3-macosx_11_0_arm64.whl"
            worker = b"\xcf\xfa\xed\xfe" + (0x0100000C).to_bytes(4, "little")
            with zipfile.ZipFile(wheel, "w") as archive:
                info = zipfile.ZipInfo("kordoc/_bin/pdfjs-worker")
                info.external_attr = 0o100755 << 16
                archive.writestr(info, worker)
                for path, data in expected.items():
                    archive.writestr(f"kordoc/_licenses/pdfjs-v8/{path}", data)
                archive.writestr("kordoc/_licenses/pdfjs-v8/extra.txt", b"extra")
            with self.assertRaisesRegex(ValueError, "notice entries do not match"):
                pdf_worker_wheel.check_wheel(
                    "aarch64-apple-darwin", wheel, manifest, root
                )

    def test_wheel_rejects_notice_digest_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, expected = self._manifest(root)
            wheel = root / "kordoc-0.1.0-cp310-abi3-macosx_11_0_arm64.whl"
            worker = b"\xcf\xfa\xed\xfe" + (0x0100000C).to_bytes(4, "little")
            with zipfile.ZipFile(wheel, "w") as archive:
                info = zipfile.ZipInfo("kordoc/_bin/pdfjs-worker")
                info.external_attr = 0o100755 << 16
                archive.writestr(info, worker)
                for path, data in expected.items():
                    if path == "pdfjs/LICENSE":
                        data = b"modified license\n"
                    archive.writestr(f"kordoc/_licenses/pdfjs-v8/{path}", data)
            with self.assertRaisesRegex(ValueError, "notice digest mismatch"):
                pdf_worker_wheel.check_wheel(
                    "aarch64-apple-darwin", wheel, manifest, root
                )

    def test_wheel_rejects_duplicate_notice_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, expected = self._manifest(root)
            wheel = root / "kordoc-0.1.0-cp310-abi3-macosx_11_0_arm64.whl"
            worker = b"\xcf\xfa\xed\xfe" + (0x0100000C).to_bytes(4, "little")
            with zipfile.ZipFile(wheel, "w") as archive:
                info = zipfile.ZipInfo("kordoc/_bin/pdfjs-worker")
                info.external_attr = 0o100755 << 16
                archive.writestr(info, worker)
                for path, data in expected.items():
                    archive.writestr(f"kordoc/_licenses/pdfjs-v8/{path}", data)
                archive.writestr(
                    "kordoc/_licenses/pdfjs-v8/pdfjs/LICENSE", b"duplicate"
                )
            with self.assertRaisesRegex(ValueError, "duplicate notice entries"):
                pdf_worker_wheel.check_wheel(
                    "aarch64-apple-darwin", wheel, manifest, root
                )


if __name__ == "__main__":
    unittest.main()
