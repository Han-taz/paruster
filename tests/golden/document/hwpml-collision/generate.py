#!/usr/bin/env python3
"""Generate the authored CC0 HWPML nested-cell collision fixtures."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
FIXTURE_DIR = Path(__file__).resolve().parent / "fixtures"

UNMATCHED = """<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<HWPML Version=\"2.9\"><BODY><SECTION Id=\"0\"><TABLE RowCount=\"1\" ColCount=\"1\"><ROW>
  <CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>owner</CHAR><TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>inner</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL>
  <CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>collision</CHAR></TEXT></P></PARALIST></CELL>
</ROW></TABLE></SECTION></BODY></HWPML>
"""

DECOY = """<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<HWPML Version=\"2.9\"><BODY><SECTION Id=\"0\"><TABLE RowCount=\"1\" ColCount=\"3\"><ROW>
  <CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>owner</CHAR><TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>inner</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL>
  <CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>collision</CHAR></TEXT></P></PARALIST></CELL>
  <CELL RowAddr=\"0\" ColAddr=\"1\"><PARALIST><P><TEXT><CHAR>owner</CHAR></TEXT></P><P><TEXT><CHAR>inner</CHAR></TEXT></P></PARALIST></CELL>
  <CELL RowAddr=\"0\" ColAddr=\"2\"><PARALIST><P><TEXT><CHAR> </CHAR></TEXT></P></PARALIST></CELL>
</ROW></TABLE></SECTION></BODY></HWPML>
"""

BLANK_COLLIDER = """<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<HWPML Version=\"2.9\"><BODY><SECTION Id=\"0\"><TABLE RowCount=\"1\" ColCount=\"1\"><ROW>
  <CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>owner</CHAR><TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>inner</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL>
  <CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR> </CHAR></TEXT></P></PARALIST></CELL>
</ROW></TABLE></SECTION></BODY></HWPML>
"""

FIXTURE_DATA = {
    "collision_unmatched.xml": UNMATCHED.encode("utf-8"),
    "collision_repeated_text_decoy.xml": DECOY.encode("utf-8"),
    "collision_blank_cell.xml": BLANK_COLLIDER.encode("utf-8"),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=FIXTURE_DIR)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if args.write:
        output.mkdir(parents=True, exist_ok=True)

    expected = set(FIXTURE_DATA)
    actual = {path.name for path in output.glob("*.xml")} if output.exists() else set()
    if not args.write and actual != expected:
        raise SystemExit(
            f"fixture names differ: missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )

    rows = []
    for name, contents in FIXTURE_DATA.items():
        path = output / name
        if args.write:
            path.write_bytes(contents)
        elif not path.is_file() or path.read_bytes() != contents:
            raise SystemExit(f"fixture is not reproducible: {path}")
        rows.append({"path": name, "bytes": len(contents), "sha256": hashlib.sha256(contents).hexdigest()})
    print(json.dumps(rows, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
