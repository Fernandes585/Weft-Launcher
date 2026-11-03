use super::super::extract_zip_blocking;
use super::support;

#[test]
fn extracts_zip_files_and_subfolders() {
    let tmp = tempfile::tempdir().unwrap();
    let zip = support::zip_bytes(&[
        ("Weft/weft.exe", b"binary"),
        ("Weft/assets/a/b/c.bin", b"nested"),
    ]);
    let archive = tmp.path().join("Weft-windows-x64.zip");
    std::fs::write(&archive, zip).unwrap();
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    extract_zip_blocking(&archive, &dest).unwrap();

    assert_eq!(
        std::fs::read(dest.join("Weft/weft.exe")).unwrap(),
        b"binary"
    );
    assert_eq!(
        std::fs::read(dest.join("Weft/assets/a/b/c.bin")).unwrap(),
        b"nested"
    );
}

#[test]
fn rejects_zip_slip_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let zip = support::zip_bytes(&[("../evil.txt", b"pwned")]);
    let archive = tmp.path().join("evil.zip");
    std::fs::write(&archive, zip).unwrap();
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    let err = extract_zip_blocking(&archive, &dest).unwrap_err();
    assert!(
        err.to_string().contains("entrada insegura"),
        "unexpected error: {err}"
    );
    assert!(!tmp.path().join("evil.txt").exists());
}
