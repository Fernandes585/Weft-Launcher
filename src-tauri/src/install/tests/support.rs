use std::fs::File as StdFile;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const EXECUTABLE_BYTES: usize = 2 * 1024 * 1024;
pub const BIN_BYTES: usize = 300 * 1024;

pub fn pseudo_random_bytes(len: usize, seed: u32) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        out.push((state >> 24) as u8);
    }
    out
}

pub struct Fixture {
    pub exe: Vec<u8>,
    pub license: Vec<u8>,
    pub bin: Vec<u8>,
    pub mod_zip: Vec<u8>,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            exe: pseudo_random_bytes(EXECUTABLE_BYTES, 0x1234_5678),
            license: b"MoltenVK is distributed under the Apache License 2.0.\n".to_vec(),
            bin: pseudo_random_bytes(BIN_BYTES, 0x9abc_def0),
            mod_zip: pseudo_random_bytes(2048, 0x0bad_f00d),
        }
    }

    /// Writes a `.tar.gz` shaped like the real macOS/Linux asset:
    /// a top-level `Weft/` folder with nested directories.
    pub fn write_tar_gz(&self, dir: &Path) -> PathBuf {
        let path = dir.join("Weft-test-macos-arm64.tar.gz");
        let file = StdFile::create(&path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);

        append_dir(&mut builder, "Weft/");
        append_file(&mut builder, "Weft/weft", &self.exe, 0o755);
        append_file(&mut builder, "Weft/MoltenVK-LICENSE", &self.license, 0o644);
        append_file(&mut builder, "Weft/assets/a/b/c.bin", &self.bin, 0o644);
        append_file(&mut builder, "Weft/run/mods/base.zip", &self.mod_zip, 0o644);

        builder.into_inner().unwrap().finish().unwrap();
        path
    }
}

pub fn append_dir<W: Write>(builder: &mut tar::Builder<W>, path: &str) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_mode(0o755);
    header.set_size(0);
    builder.append_data(&mut header, path, std::io::empty()).unwrap();
}

pub fn append_file<W: Write>(builder: &mut tar::Builder<W>, path: &str, data: &[u8], mode: u32) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(mode);
    header.set_size(data.len() as u64);
    builder.append_data(&mut header, path, data).unwrap();
}

/// Writes the name bytes directly so `..` reaches the archive untouched.
pub fn append_raw_path<W: Write>(builder: &mut tar::Builder<W>, path: &[u8], data: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(data.len() as u64);
    {
        let name = &mut header.as_gnu_mut().unwrap().name;
        name[..path.len()].copy_from_slice(path);
        name[path.len()] = 0;
    }
    header.set_cksum();
    builder.append(&header, data).unwrap();
}

pub fn append_symlink<W: Write>(builder: &mut tar::Builder<W>, path: &str, target: &str) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_mode(0o777);
    header.set_size(0);
    builder.append_link(&mut header, path, target).unwrap();
}

/// Builds an in-memory `.zip` with the given entries, in order.
pub fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    use zip::write::FileOptions;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = FileOptions::default().unix_permissions(0o644);
    for (name, data) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

pub enum Entry<'a> {
    Dir(&'a str),
    File(&'a str, &'a [u8], u32),
    /// Name bytes are written verbatim (bypasses tar-rs path preparation).
    Raw(&'a [u8], &'a [u8]),
    #[allow(dead_code)] // only constructed by the unix-only link tests
    Symlink(&'a str, &'a str),
}

/// Writes a `.tar.gz` containing exactly `entries`, in order.
pub fn write_tar_gz_entries(dir: &Path, name: &str, entries: &[Entry]) -> PathBuf {
    let path = dir.join(name);
    let file = StdFile::create(&path).unwrap();
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);

    for entry in entries {
        match entry {
            Entry::Dir(path) => append_dir(&mut builder, path),
            Entry::File(path, data, mode) => append_file(&mut builder, path, data, *mode),
            Entry::Raw(path, data) => append_raw_path(&mut builder, path, data),
            Entry::Symlink(path, target) => append_symlink(&mut builder, path, target),
        }
    }

    builder.into_inner().unwrap().finish().unwrap();
    path
}
