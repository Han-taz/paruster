use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const ASSET_ROOT: &str = "assets/pdfjs";
const FIXTURE_DIR: &str = "tests/fixtures/pdfjs_probe";
const EXPECTED_FIXTURE_SHA256: &str =
    "b290aaa8fa5b388a5f3893bba6fc9e8d73f5350af681d95269b986f9dcdef451";
const EXPECTED_SELECTED_FILE_COUNT: usize = 188;

#[test]
fn checked_in_pdfjs_assets_and_probe_fixture_are_reproducible() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let asset_root = crate_root.join(ASSET_ROOT);
    let manifest: Value = serde_json::from_slice(
        &fs::read(asset_root.join("PROVENANCE.json")).expect("read asset provenance"),
    )
    .expect("parse asset provenance JSON");

    assert_eq!(manifest["package"], "pdfjs-dist");
    assert_eq!(manifest["version"], "4.10.38");
    assert_eq!(
        manifest["registry_integrity"], manifest["verified_download_integrity"],
        "the recorded tarball integrity must match the verified download"
    );

    let selected = manifest["selected_files"]
        .as_array()
        .expect("selected_files must be an array");
    assert_eq!(selected.len(), EXPECTED_SELECTED_FILE_COUNT);
    let mut expected_paths: HashSet<PathBuf> = HashSet::new();
    for entry in selected {
        let relative = entry["path"].as_str().expect("file path string");
        let relative_path = Path::new(relative);
        assert!(
            relative_path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
            "manifest path must stay beneath asset root: {relative}"
        );
        assert!(
            expected_paths.insert(relative_path.to_path_buf()),
            "duplicate manifest path: {relative}"
        );

        let path = asset_root.join(relative_path);
        let bytes =
            fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert_eq!(
            entry["bytes"].as_u64(),
            Some(bytes.len() as u64),
            "byte count mismatch for {relative}"
        );
        assert_eq!(
            entry["sha256"].as_str(),
            Some(hex(&Sha256::digest(&bytes)).as_str()),
            "SHA-256 mismatch for {relative}"
        );
        assert!(
            entry["license"]
                .as_str()
                .is_some_and(|license| !license.is_empty())
        );
    }

    let mut actual_paths = HashSet::new();
    collect_files(&asset_root, &asset_root, &mut actual_paths);
    actual_paths.remove(Path::new("PROVENANCE.json"));
    assert_eq!(
        actual_paths, expected_paths,
        "asset tree has missing or unmanifested files"
    );

    for license in [
        "LICENSE",
        "cmaps/LICENSE",
        "standard_fonts/LICENSE_FOXIT",
        "standard_fonts/LICENSE_LIBERATION",
    ] {
        assert!(
            asset_root.join(license).is_file(),
            "missing required license: {license}"
        );
    }

    let main = fs::read_to_string(asset_root.join("legacy/build/pdf.mjs"))
        .expect("read PDF.js main module");
    let worker = fs::read_to_string(asset_root.join("legacy/build/pdf.worker.mjs"))
        .expect("read PDF.js worker module");
    for (name, source) in [("pdf.mjs", &main), ("pdf.worker.mjs", &worker)] {
        assert!(
            !has_static_import(source),
            "{name} must not have static imports for fully offline embedding"
        );
    }

    let fixture_dir = crate_root.join(FIXTURE_DIR);
    let fixture =
        fs::read(fixture_dir.join("one_page_helvetica.pdf")).expect("read authored probe PDF");
    assert_eq!(hex(&Sha256::digest(&fixture)), EXPECTED_FIXTURE_SHA256);

    let first_generation = generate_probe_pdf();
    let second_generation = generate_probe_pdf();
    assert_eq!(
        first_generation, second_generation,
        "probe recipe must be deterministic"
    );
    assert_eq!(
        first_generation, fixture,
        "checked-in probe must match the reproducible recipe"
    );
}

fn collect_files(root: &Path, directory: &Path, paths: &mut HashSet<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read asset directory") {
        let path = entry.expect("read asset entry").path();
        if path.is_dir() {
            collect_files(root, &path, paths);
        } else {
            paths.insert(
                path.strip_prefix(root)
                    .expect("file below asset root")
                    .to_path_buf(),
            );
        }
    }
}

fn has_static_import(source: &str) -> bool {
    ["import ", "import{", "import\n", "import\t"]
        .iter()
        .any(|needle| source.contains(needle))
}

fn generate_probe_pdf() -> Vec<u8> {
    let stream = b"BT /F1 12 Tf 72 720 Td (V8 PDF.js probe) Tj ET\n";
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut stream_object = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
    stream_object.extend_from_slice(stream);
    stream_object.extend_from_slice(b"endstream");
    objects.push(stream_object);

    let mut pdf = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}
