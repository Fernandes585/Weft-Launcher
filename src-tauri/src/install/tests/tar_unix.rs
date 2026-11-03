use super::super::extract_tar_gz_blocking;
use super::support::{self, Entry};

#[test]
fn keeps_exec_bit_and_relative_symlink_inside_dest() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let archive = support::write_tar_gz_entries(
        tmp.path(),
        "links.tar.gz",
        &[
            Entry::Dir("Weft/"),
            Entry::Dir("Weft/bin/"),
            Entry::File("Weft/weft", b"binary", 0o755),
            Entry::Symlink("Weft/bin/weft-link", "../weft"),
        ],
    );
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    extract_tar_gz_blocking(&archive, &dest).unwrap();

    let exe = dest.join("Weft/weft");
    let mode = std::fs::metadata(&exe).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);

    let link = dest.join("Weft/bin/weft-link");
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        std::path::PathBuf::from("../weft")
    );
    assert_eq!(std::fs::read(&link).unwrap(), b"binary");
}

#[test]
fn rejects_symlink_that_escapes_dest() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = support::write_tar_gz_entries(
        tmp.path(),
        "escape.tar.gz",
        &[
            Entry::Dir("Weft/"),
            Entry::Symlink("Weft/escape", "../../outside"),
        ],
    );
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    let err = extract_tar_gz_blocking(&archive, &dest).unwrap_err();
    assert!(
        err.to_string().contains("entrada insegura"),
        "unexpected error: {err}"
    );
    assert!(!tmp.path().join("outside").exists());
}
