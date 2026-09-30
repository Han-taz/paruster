#!/usr/bin/env python3
"""Deterministically generate the authored CC0 HWPML H0 inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
FIXTURE_DIR = ROOT / "tests" / "golden" / "document" / "hwpml" / "fixtures"

NORMAL = """<?xml version="1.0" encoding="UTF-8"?>
<HWPML xmlns="http://www.hancom.co.kr/hwpml/2011/hwpml" xmlns:h="http://www.hancom.co.kr/hwpml/2011/hwpml" Version="2.9">
  <h:DOCSUMMARY><h:TITLE>합성 HWPML 제목</h:TITLE><h:AUTHOR>Open Fixture</h:AUTHOR><h:DATE>2026-09-30</h:DATE></h:DOCSUMMARY>
  <h:HEAD><h:MAPPINGTABLE><h:PARASHAPELIST>
    <h:PARASHAPE Id="title" HeadingType="Outline" Level="0"/>
    <h:PARASHAPE Id="deep" HeadingType="Outline" Level="99"/>
  </h:PARASHAPELIST></h:MAPPINGTABLE></h:HEAD>
  <h:BODY>
    <h:SECTION Id="0">
      <h:P ParaShape="title"><h:TEXT><h:CHAR>첫</h:CHAR><h:CHAR> 제목</h:CHAR></h:TEXT></h:P>
      <h:P ParaShape="plain"><h:TEXT><h:CHAR>앞&nbsp;뒤</h:CHAR><h:CHAR> &amp; 끝</h:CHAR>
        <h:AUTONUM>자동번호제외</h:AUTONUM>
        <h:PICTURE><h:CHAR>그림텍스트제외</h:CHAR></h:PICTURE>
        <h:SHAPEOBJECT><h:CHAR>도형텍스트제외</h:CHAR></h:SHAPEOBJECT>
        <h:FOOTNOTE><h:TEXT><h:CHAR>각주내용은문단에포함</h:CHAR></h:TEXT></h:FOOTNOTE>
      </h:TEXT></h:P>
      <h:HEADER><h:P><h:TEXT><h:CHAR>머리말제외</h:CHAR></h:TEXT></h:P></h:HEADER>
      <h:FOOTER><h:P><h:TEXT><h:CHAR>꼬리말제외</h:CHAR></h:TEXT></h:P></h:FOOTER>
    </h:SECTION>
    <h:SECTION Id="1">
      <h:P ParaShape="deep"><h:TEXT><h:CHAR>둘째 구역 제목</h:CHAR></h:TEXT></h:P>
      <h:P><h:TEXT><h:CHAR>둘째 구역 본문</h:CHAR></h:TEXT></h:P>
    </h:SECTION>
  </h:BODY>
</HWPML>
"""

EMPTY_BODY = """<?xml version="1.0" encoding="UTF-8"?>
<HWPML Version="2.9"><DOCSUMMARY><TITLE>빈 본문 문서</TITLE></DOCSUMMARY><BODY/></HWPML>
"""

EMPTY_SECTIONS = """<?xml version="1.0" encoding="UTF-8"?>
<HWPML Version="2.9"><BODY><SECTION Id="0"/><SECTION Id="1"><HEADER><P/></HEADER></SECTION></BODY></HWPML>
"""

NESTED_TABLE = """<?xml version="1.0" encoding="UTF-8"?>
<HWPML Version="2.9"><BODY><SECTION Id="0">
  <P><TEXT><CHAR>표 앞 문단</CHAR></TEXT>
    <TABLE RowCount="2" ColCount="3">
      <ROW>
        <CELL RowAddr="0" ColAddr="0" RowSpan="2" ColSpan="1"><PARALIST><P><TEXT><CHAR>세로 병합</CHAR></TEXT></P></PARALIST></CELL>
        <CELL RowAddr="0" ColAddr="1" RowSpan="1" ColSpan="1"><PARALIST><P><TEXT><CHAR>중첩 셀</CHAR><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"><PARALIST><P><TEXT><CHAR>안쪽 표</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL>
        <CELL RowAddr="0" ColAddr="2" RowSpan="1" ColSpan="1"><PARALIST><P><TEXT><CHAR> </CHAR></TEXT></P></PARALIST></CELL>
      </ROW>
      <ROW>
        <CELL RowAddr="1" ColAddr="1" RowSpan="1" ColSpan="2"><PARALIST><P><TEXT><CHAR>가로 병합</CHAR></TEXT></P></PARALIST></CELL>
      </ROW>
    </TABLE>
  </P>
</SECTION></BODY></HWPML>
"""

MALFORMED_PARTIAL = """<?xml version="1.0" encoding="UTF-8"?>
<HWPML Version="2.9"><BODY><SECTION Id="0"><P><TEXT><CHAR>복구 관찰 문장</CHAR></TEXT></P><P><TEXT><CHAR>미정의 &unknown; 엔티티</CHAR></TEXT></P></SECTION></BODY></HWPML>
"""

MALFORMED_UNCLOSED = """<?xml version="1.0" encoding="UTF-8"?>
<HWPML Version="2.9"><BODY><SECTION Id="0"><P><TEXT><CHAR>복구 관찰 문장</CHAR></TEXT></P><P><TEXT><CHAR>열린 문장</CHAR></TEXT>
"""

DTD_EXTERNAL_ENTITY = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE HWPML [<!ENTITY local SYSTEM "file:///etc/passwd">]>
<HWPML Version="2.9"><BODY><SECTION Id="0"><P><TEXT><CHAR>앞&amp;local;뒤</CHAR></TEXT></P></SECTION></BODY></HWPML>
"""

DTD_EXTERNAL_ENTITY_REFERENCE = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE HWPML [<!ENTITY probe SYSTEM "file:///nonexistent/paruster-hwpml-external-entity-probe.txt">]>
<HWPML Version="2.9"><BODY><SECTION Id="0"><P><TEXT><CHAR>앞&probe;뒤</CHAR></TEXT></P></SECTION></BODY></HWPML>
"""

FIXTURE_DATA: dict[str, bytes] = {
    "normal_metadata_styles.xml": bytes.fromhex("efbbbf") + NORMAL.encode("utf-8"),
    "empty_body.xml": EMPTY_BODY.encode("utf-8"),
    "empty_sections.xml": EMPTY_SECTIONS.encode("utf-8"),
    "nested_table.xml": NESTED_TABLE.encode("utf-8"),
    "malformed_partial.xml": MALFORMED_PARTIAL.encode("utf-8"),
    "malformed_unclosed.xml": MALFORMED_UNCLOSED.encode("utf-8"),
    "dtd_external_entity.xml": DTD_EXTERNAL_ENTITY.encode("utf-8"),
    "dtd_external_entity_reference.xml": DTD_EXTERNAL_ENTITY_REFERENCE.encode("utf-8"),
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=FIXTURE_DIR)
    parser.add_argument(
        "--write",
        action="store_true",
        help="write fixture bytes; default verifies existing bytes without mutation",
    )
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if args.write:
        output.mkdir(parents=True, exist_ok=True)

    expected_names = set(FIXTURE_DATA)
    actual_names = {path.name for path in output.glob("*.xml")} if output.exists() else set()
    if not args.write and actual_names != expected_names:
        raise SystemExit(
            f"fixture names differ: missing={sorted(expected_names - actual_names)}, "
            f"extra={sorted(actual_names - expected_names)}"
        )

    rows = []
    for name, contents in FIXTURE_DATA.items():
        path = output / name
        if args.write:
            path.write_bytes(contents)
        elif not path.is_file() or path.read_bytes() != contents:
            raise SystemExit(f"fixture is not reproducible: {path}")
        rows.append({"path": name, "bytes": len(contents), "sha256": digest(contents)})
    print(json.dumps(rows, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
