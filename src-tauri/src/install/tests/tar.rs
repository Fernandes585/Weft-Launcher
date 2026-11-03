use super::super::{extract_tar_gz_blocking, normalize_install_structure};
use super::support::{self, Entry, Fixture};

#[test]
fn extracts_macos_tar_gz_fixture() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new();
    let archive = fixture.write_tar_gz(tmp.path());
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    extract_tar_gz_blocking(&archive, &dest).unwrap();

    let weft = dest.join("Weft");
    assert_eq!(std::fs::read(weft.join("weft")).unwrap(), fixture.exe);
    assert_eq!(
        std::fs::read(weft.join("MoltenVK-LICENSE")).unwrap(),
        fixture.license
    );
    assert_eq!(
        std::fs::read(weft.join("assets/a/b/c.bin")).unwrap(),
        fixture.bin
    );
    assert_eq!(
        std::fs::read(weft.join("run/mods/base.zip")).unwrap(),
        fixture.mod_zip
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(weft.join("weft"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }
}

#[tokio::test]
async fn normalize_lifts_top_level_weft_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new();
    let archive = fixture.write_tar_gz(tmp.path());
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    extract_tar_gz_blocking(&archive, &dest).unwrap();
    normalize_install_structure(&dest, "v-test").await.unwrap();

    // Case-insensitive filesystems resolve `Weft` to the lifted `weft` file.
    assert!(!dest.join("Weft").is_dir());
    assert_eq!(std::fs::read(dest.join("weft")).unwrap(), fixture.exe);
    assert_eq!(
        std::fs::read(dest.join("assets/a/b/c.bin")).unwrap(),
        fixture.bin
    );
}

#[test]
fn rejects_parent_dir_entry_without_writing_outside() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = support::write_tar_gz_entries(
        tmp.path(),
        "evil.tar.gz",
        &[
            Entry::File("Weft/ok.txt", b"ok", 0o644),
            Entry::Raw(b"../evil.txt", b"pwned"),
        ],
    );
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    let err = extract_tar_gz_blocking(&archive, &dest).unwrap_err();
    assert!(
        err.to_string().contains("entrada insegura"),
        "unexpected error: {err}"
    );
    assert!(!tmp.path().join("evil.txt").exists());
}

#[test]
fn ignores_macos_metadata_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = support::write_tar_gz_entries(
        tmp.path(),
        "junk.tar.gz",
        &[
            Entry::Dir("Weft/"),
            Entry::File("Weft/weft", b"binary", 0o644),
            Entry::File("Weft/._weft", b"resource-fork", 0o644),
            Entry::Dir("__MACOSX/Weft/"),
            Entry::File("__MACOSX/Weft/._weft", b"resource-fork", 0o644),
        ],
    );
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    extract_tar_gz_blocking(&archive, &dest).unwrap();

    assert!(dest.join("Weft/weft").is_file());
    assert!(!dest.join("Weft/._weft").exists());
    assert!(!dest.join("__MACOSX").exists());
    let top_level: Vec<_> = std::fs::read_dir(&dest)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(top_level, vec![std::ffi::OsString::from("Weft")]);
}
