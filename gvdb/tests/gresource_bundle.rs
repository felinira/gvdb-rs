#![cfg(feature = "gresource")]

use gvdb::gresource::*;
use gvdb::read::File;
use matches::assert_matches;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub(crate) static TEST_FILE_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("test-data"));
pub(crate) static GRESOURCE_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| TEST_FILE_DIR.join("gresource"));

#[test]
fn test_file_from_dir() {
    let builder =
        BundleBuilder::from_directory("/gvdb/rs/test", &GRESOURCE_DIR, true, true).unwrap();
    let data = builder.build().unwrap();
    let root = File::from_bytes(Cow::Owned(data)).unwrap();

    let table = root.hash_table().unwrap();
    let mut names = table.keys().collect::<Result<Vec<_>, _>>().unwrap();

    names.sort();
    let reference_names = vec![
        "/",
        "/gvdb/",
        "/gvdb/rs/",
        "/gvdb/rs/test/",
        "/gvdb/rs/test/icons/",
        "/gvdb/rs/test/icons/scalable/",
        "/gvdb/rs/test/icons/scalable/actions/",
        "/gvdb/rs/test/icons/scalable/actions/online-symbolic.svg",
        "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
        "/gvdb/rs/test/json/",
        "/gvdb/rs/test/json/test.json",
        "/gvdb/rs/test/test.css",
    ];
    assert_eq!(names, reference_names);

    let svg2 = zvariant::Structure::try_from(
        table
            .get_value("/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg")
            .unwrap(),
    )
    .unwrap()
    .into_fields();
    let svg2_size = u32::try_from(&svg2[0]).unwrap();
    let svg2_flags = u32::try_from(&svg2[1]).unwrap();
    let svg2_data = <Vec<u8>>::try_from(svg2[2].try_clone().unwrap()).unwrap();

    assert_eq!(svg2_size, 339);
    assert_eq!(svg2_flags, 0);

    // Check for null byte
    assert_eq!(svg2_data[svg2_data.len() - 1], 0);
    assert_eq!(svg2_size as usize, svg2_data.len() - 1);
}

#[test]
/// Make sure from_dir reproducibly creates an identical file
fn test_from_dir_reproducible_build() {
    let mut last_data = None;

    use rand::prelude::*;
    fn copy_random_order(from: &Path, to: &Path) {
        let mut rng = rand::thread_rng();
        let mut files: Vec<std::fs::DirEntry> = std::fs::read_dir(from)
            .unwrap()
            .map(|d| d.unwrap())
            .collect();
        files.shuffle(&mut rng);

        for entry in files.iter() {
            let destination = to.join(entry.file_name());
            println!("copy file: {:?} to: {:?}", entry, destination);
            let file_type = entry.file_type().unwrap();
            if file_type.is_file() {
                std::fs::copy(entry.path(), &destination).unwrap();
            } else if file_type.is_dir() {
                std::fs::create_dir(&destination).unwrap();
                copy_random_order(&entry.path(), &destination);
            }
        }
    }

    for _ in 0..10 {
        // Create a new directory with inodes in random order
        let test_dir = tempfile::tempdir().unwrap();

        // Randomize order of root files and copy to test dir
        copy_random_order(&GRESOURCE_DIR, test_dir.path());

        let builder =
            BundleBuilder::from_directory("/gvdb/rs/test", test_dir.path(), true, true).unwrap();
        let data = builder.build().unwrap();

        if let Some(last_data) = last_data {
            assert_eq!(last_data, data);
        }

        last_data = Some(data);
    }
}

#[test]
#[cfg(unix)]
fn test_from_dir_invalid() {
    use std::os::unix::ffi::OsStrExt;
    let invalid_utf8 = OsStr::from_bytes(&[0xC3, 0x28]);
    let mut dir: PathBuf = ["test-data", "temp2"].iter().collect();
    dir.push(invalid_utf8);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::File::create(dir.join("test.xml")).unwrap();
    let res = BundleBuilder::from_directory("test", dir.parent().unwrap(), false, false);
    let _ = std::fs::remove_file(dir.join("test.xml"));
    let _ = std::fs::remove_dir(&dir);
    std::fs::remove_dir(dir.parent().unwrap()).unwrap();

    let err = res.unwrap_err();
    println!("{}", err);
    assert_matches!(err, BuilderError::Utf8(_, _));
    assert!(format!("{}", err).contains("UTF-8"));
}

#[test]
fn test_invalid_utf8_json() {
    use std::os::unix::ffi::OsStrExt;
    let invalid_utf8 = OsStr::from_bytes(&[0xC3, 0x28]);
    let dir: PathBuf = ["test-data", "temp3"].iter().collect();
    std::fs::create_dir_all(&dir).unwrap();
    let mut file = std::fs::File::create(dir.join("test.json")).unwrap();
    let _ = file.write(invalid_utf8.as_bytes());

    let res = BundleBuilder::from_directory("test", &dir, true, true);
    let _ = std::fs::remove_file(dir.join("test.json"));
    let _ = std::fs::remove_dir(&dir);

    let err = res.unwrap_err();
    println!("{}", err);
    assert_matches!(err, BuilderError::Utf8(..));
    assert!(format!("{}", err).contains("UTF-8"));
}

#[test]
fn test_from_file_data() {
    let path = GRESOURCE_DIR.join("json").join("test.json");
    let file_data = FileData::from_file(
        "test.json".to_string(),
        &path,
        false,
        &PreprocessOptions::empty(),
    )
    .unwrap();
    println!("{:?}", file_data);

    let builder = BundleBuilder::from_file_data(vec![file_data]);
    println!("{:?}", builder);
    let _ = builder.build().unwrap();
}

#[test]
#[cfg(unix)]
fn invalid_utf8_filename() {
    use std::os::unix::ffi::OsStrExt;
    let temp_path: PathBuf = ["test-data", "temp"].iter().collect();
    let mut invalid_path = temp_path.clone();

    invalid_path.push(OsStr::from_bytes(&[0xC3, 0x28]));
    std::fs::create_dir_all(PathBuf::from(&temp_path)).unwrap();
    let _ = std::fs::File::create(&invalid_path).unwrap();

    let res = BundleBuilder::from_directory("test", &temp_path, false, false);

    let _ = std::fs::remove_file(invalid_path);
    std::fs::remove_dir(temp_path).unwrap();

    let err = res.unwrap_err();
    assert_matches!(err, BuilderError::Utf8(_, _));
    assert!(err.to_string().contains("UTF-8"));

    assert_matches!(err, BuilderError::Utf8(_, _));
    assert!(err.to_string().contains("UTF-8"));
}
