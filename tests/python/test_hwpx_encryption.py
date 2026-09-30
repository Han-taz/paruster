from __future__ import annotations

import json
import zipfile
from pathlib import Path

import kordoc
import pytest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "golden" / "document" / "hwpx" / "fixtures"
PASSWORD = "fixture-password"
WRONG_PASSWORD = "never-echo-this-password"
ENCRYPTED_FIXTURES = ("encrypted_sha1.hwpx", "encrypted_sha256.hwpx")


def _fixture_bytes(name: str) -> bytes:
    return (FIXTURES / name).read_bytes()


def _parse_with(api: str, data: bytes, password: str | None = None):
    options = None if password is None else {"password": password}
    if api == "parse_hwpx":
        return kordoc.parse_hwpx(data, options=options)
    if api == "parse":
        return kordoc.parse(data, options=options)
    raise AssertionError(f"unknown parse API: {api}")


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
@pytest.mark.parametrize("api", ("parse_hwpx", "parse"))
def test_parse_apis_require_a_password_for_encrypted_sections(
    fixture_name: str, api: str
) -> None:
    data = _fixture_bytes(fixture_name)

    with pytest.raises(kordoc.EncryptedError) as caught:
        _parse_with(api, data)

    assert caught.value.code == "ENCRYPTED"


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
def test_try_parse_reports_missing_password_as_hwpx_encrypted_failure(
    fixture_name: str,
) -> None:
    result = kordoc.try_parse(_fixture_bytes(fixture_name))

    assert result.success is False
    assert result.file_type == "hwpx"
    assert result.code == "ENCRYPTED"
    assert result.document is None


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
def test_validator_requires_a_password_for_encrypted_sections(
    fixture_name: str,
) -> None:
    with pytest.raises(kordoc.EncryptedError) as caught:
        kordoc.validate_hwpx(_fixture_bytes(fixture_name))

    assert caught.value.code == "ENCRYPTED"


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
@pytest.mark.parametrize("api", ("parse_hwpx", "parse"))
def test_parse_apis_reject_wrong_password_without_echoing_it(
    fixture_name: str, api: str
) -> None:
    with pytest.raises(kordoc.EncryptedError) as caught:
        _parse_with(api, _fixture_bytes(fixture_name), WRONG_PASSWORD)

    assert caught.value.code == "ENCRYPTED"
    assert WRONG_PASSWORD not in str(caught.value)
    assert WRONG_PASSWORD not in caught.value.message


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
def test_try_parse_wrong_password_is_typed_and_does_not_echo_secret(
    fixture_name: str,
) -> None:
    result = kordoc.try_parse(
        _fixture_bytes(fixture_name), options={"password": WRONG_PASSWORD}
    )

    assert result.success is False
    assert result.file_type == "hwpx"
    assert result.code == "ENCRYPTED"
    assert result.error is not None
    assert WRONG_PASSWORD not in result.error
    assert WRONG_PASSWORD not in json.dumps(result.to_dict())


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
def test_validator_wrong_password_is_typed_and_does_not_echo_secret(
    fixture_name: str,
) -> None:
    with pytest.raises(kordoc.EncryptedError) as caught:
        kordoc.validate_hwpx(_fixture_bytes(fixture_name), password=WRONG_PASSWORD)

    assert caught.value.code == "ENCRYPTED"
    assert WRONG_PASSWORD not in str(caught.value)
    assert WRONG_PASSWORD not in caught.value.message


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
@pytest.mark.parametrize("api", ("parse_hwpx", "parse", "try_parse"))
def test_correct_password_matches_unencrypted_minimal_wire(
    fixture_name: str, api: str
) -> None:
    encrypted = _fixture_bytes(fixture_name)
    minimal = _fixture_bytes("minimal.hwpx")
    baseline = kordoc.parse_hwpx(minimal).to_dict()
    if api == "try_parse":
        result = kordoc.try_parse(encrypted, options={"password": PASSWORD})
    else:
        result = _parse_with(api, encrypted, PASSWORD)

    assert result.success is True
    assert result.file_type == "hwpx"
    assert result.to_dict() == baseline


@pytest.mark.parametrize("fixture_name", ENCRYPTED_FIXTURES)
def test_correct_password_validates_and_reports_files_only_entry_count(
    fixture_name: str,
) -> None:
    result = kordoc.validate_hwpx(_fixture_bytes(fixture_name), password=PASSWORD)
    with zipfile.ZipFile(FIXTURES / fixture_name) as archive:
        files_only = sum(not member.is_dir() for member in archive.infolist())

    assert result.ok is True
    assert result.issues == ()
    assert result.entry_count == files_only


@pytest.mark.parametrize(
    ("api", "password", "error_type"),
    [
        ("parse_hwpx", 7, TypeError),
        ("parse", 7, TypeError),
        ("validate_hwpx", 7, TypeError),
        ("parse_hwpx", "p" * 65_537, ValueError),
        ("parse", "p" * 65_537, ValueError),
        ("validate_hwpx", "p" * 65_537, ValueError),
    ],
)
def test_encryption_apis_validate_password_type_and_length(
    api: str, password: object, error_type: type[Exception]
) -> None:
    data = _fixture_bytes("minimal.hwpx")
    with pytest.raises(error_type):
        if api == "validate_hwpx":
            kordoc.validate_hwpx(data, password=password)  # type: ignore[arg-type]
        else:
            _parse_with(api, data, password)  # type: ignore[arg-type]
