import kordoc


def test_package_imports_native_version() -> None:
    assert kordoc.__version__ == "0.1.0"
    assert kordoc.native_version() == "0.1.0"
