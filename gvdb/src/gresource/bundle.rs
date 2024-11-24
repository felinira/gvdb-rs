mod error;

pub use error::*;

use crate::gresource::xml::PreprocessOptions;
use crate::write::{FileWriter, HashTableBuilder};
use flate2::write::ZlibEncoder;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const FLAG_COMPRESSED: u32 = 1 << 0;

static SKIPPED_FILE_EXTENSIONS_DEFAULT: &[&str] =
    &["meson.build", "gresource.xml", ".gitignore", ".license"];
static COMPRESS_EXTENSIONS_DEFAULT: &[&str] = &[".ui", ".css"];

/// A container for a GResource data object
///
/// Allows to read a file from the filesystem. The file is then preprocessed and compressed.
///
/// ```
/// # use std::path::PathBuf;
/// use gvdb::gresource::{PreprocessOptions, FileData};
///
/// let mut key = "/my/app/id/icons/scalable/actions/send-symbolic.svg".to_string();
/// let mut filename = PathBuf::from("test-data/gresource/icons/scalable/actions/send-symbolic.svg");
///
/// let preprocess_options = PreprocessOptions::empty();
/// let file_data =
///     FileData::from_file(key, &filename, true, &preprocess_options).unwrap();
/// ```
#[derive(Debug)]
pub struct FileData<'a> {
    key: String,
    data: Cow<'a, [u8]>,
    flags: u32,

    /// uncompressed data is zero-terminated
    /// compressed data is not
    size: u32,
}

impl<'a> FileData<'a> {
    /// Create a new `GResourceFileData` from raw bytes
    ///
    /// The `path` parameter is used for error output, and should be set to a valid filesystem path
    /// if possible or `None` if not applicable.
    ///
    /// Preprocessing will be applied based on the `preprocess` parameter.
    /// Will compress the data if `compressed` is set.
    ///
    /// ```
    /// # use std::borrow::Cow;
    /// use std::path::PathBuf;
    /// use gvdb::gresource::{FileData, PreprocessOptions};
    ///
    /// let mut key = "/my/app/id/style.css".to_string();
    /// let mut filename = PathBuf::from("path/to/style.css");
    ///
    /// let preprocess_options = PreprocessOptions::empty();
    /// let data: Vec<u8> = vec![1, 2, 3, 4];
    /// let file_data =
    ///     FileData::new(key, Cow::Owned(data), None, true, &preprocess_options).unwrap();
    /// ```
    pub fn new(
        key: String,
        data: Cow<'a, [u8]>,
        path: Option<PathBuf>,
        compressed: bool,
        preprocess: &PreprocessOptions,
    ) -> BuilderResult<Self> {
        let mut flags = 0;
        let mut data = Self::preprocess(data, preprocess, path.clone())?;
        let size = data.len() as u32;

        if compressed {
            data = Self::compress(data, path)?;
            flags |= FLAG_COMPRESSED;
        } else {
            data.to_mut().push(0);
        }

        Ok(Self {
            key,
            data,
            flags,
            size,
        })
    }

    /// Read the data from a file
    ///
    /// Preprocessing will be applied based on the `preprocess` parameter.
    /// Will compress the data if `compressed` is set.
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// use gvdb::gresource::{FileData, PreprocessOptions};
    ///
    /// let mut key = "/my/app/id/icons/scalable/actions/send-symbolic.svg".to_string();
    /// let mut filename = PathBuf::from("test-data/gresource/icons/scalable/actions/send-symbolic.svg");
    ///
    /// let preprocess_options = PreprocessOptions::empty();
    /// let file_data =
    ///     FileData::from_file(key, &filename, true, &preprocess_options).unwrap();
    /// ```
    pub fn from_file(
        key: String,
        file_path: &Path,
        compressed: bool,
        preprocess: &PreprocessOptions,
    ) -> BuilderResult<Self> {
        let mut open_file = std::fs::File::open(file_path)
            .map_err(BuilderError::from_io_with_filename(Some(file_path)))?;
        let mut data = Vec::new();
        open_file
            .read_to_end(&mut data)
            .map_err(BuilderError::from_io_with_filename(Some(file_path)))?;
        FileData::new(
            key,
            Cow::Owned(data),
            Some(file_path.to_path_buf()),
            compressed,
            preprocess,
        )
    }

    fn xml_stripblanks(data: Cow<'a, [u8]>, path: Option<PathBuf>) -> BuilderResult<Cow<'a, [u8]>> {
        let output = Vec::new();

        let mut reader = quick_xml::Reader::from_str(
            std::str::from_utf8(&data).map_err(|err| BuilderError::Utf8(err, path.clone()))?,
        );
        reader.config_mut().trim_text(true);

        let mut writer = quick_xml::Writer::new(std::io::Cursor::new(output));

        loop {
            match reader
                .read_event()
                .map_err(|err| BuilderError::Xml(err, path.clone()))?
            {
                quick_xml::events::Event::Eof => break,
                event => writer
                    .write_event(event)
                    .map_err(|err| BuilderError::Xml(err, path.clone()))?,
            }
        }

        Ok(Cow::Owned(writer.into_inner().into_inner()))
    }

    fn json_stripblanks(
        data: Cow<'a, [u8]>,
        path: Option<PathBuf>,
    ) -> BuilderResult<Cow<'a, [u8]>> {
        let string =
            std::str::from_utf8(&data).map_err(|err| BuilderError::Utf8(err, path.clone()))?;

        let json: serde_json::Value =
            serde_json::from_str(string).map_err(|err| BuilderError::Json(err, path.clone()))?;

        let mut output = json.to_string().as_bytes().to_vec();
        output.push(b'\n');

        Ok(Cow::Owned(output))
    }

    fn preprocess(
        mut data: Cow<'a, [u8]>,
        options: &PreprocessOptions,
        path: Option<PathBuf>,
    ) -> BuilderResult<Cow<'a, [u8]>> {
        if options.xml_stripblanks {
            data = Self::xml_stripblanks(data, path.clone())?;
        }

        if options.json_stripblanks {
            data = Self::json_stripblanks(data, path)?;
        }

        if options.to_pixdata {
            return Err(BuilderError::Unimplemented(
                "to-pixdata is deprecated since gdk-pixbuf 2.32 and not supported by gvdb-rs"
                    .to_string(),
            ));
        }

        Ok(data)
    }

    fn compress(data: Cow<'a, [u8]>, path: Option<PathBuf>) -> BuilderResult<Cow<'a, [u8]>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder
            .write_all(&data)
            .map_err(BuilderError::from_io_with_filename(path.clone()))?;
        Ok(Cow::Owned(
            encoder
                .finish()
                .map_err(BuilderError::from_io_with_filename(path))?,
        ))
    }

    /// Return the `key` of this `FileData`
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// We define equality as key equality only. The resulting file can only have one file for each key.
impl std::cmp::PartialEq for FileData<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl std::cmp::Eq for FileData<'_> {}

/// We define ordering as key ordering only. The resulting file can only have one file for each key.
impl std::cmp::PartialOrd for FileData<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for FileData<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

/// GResource data value
///
/// This is the format in which all GResource files are stored in the GVDB file.
///
/// The size is the *uncompressed* size and can be used for verification purposes.
/// The flags only indicate whether a file is compressed or not. (Compressed = 1)
#[derive(zvariant::Type, zvariant::Value, zvariant::OwnedValue)]
pub struct Data {
    size: u32,
    flags: u32,
    data: Vec<u8>,
}

/// Create a GResource binary file
///
/// # Example
///
/// Create a GResource XML file with [`XmlManifest`][crate::gresource::XmlManifest] and
/// [`BundleBuilder`]
/// ```
/// use std::borrow::Cow;
/// use std::path::PathBuf;
/// use gvdb::gresource::BundleBuilder;
/// use gvdb::gresource::XmlManifest;
/// use gvdb::read::File;
///
/// const GRESOURCE_XML: &str = "test/data/gresource/test3.gresource.xml";
///
/// fn create_gresource() {
///     let doc = XmlManifest::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
///     let builder = BundleBuilder::from_xml(doc).unwrap();
///     let data = builder.build().unwrap();
///     let root = File::from_bytes(Cow::Owned(data)).unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct BundleBuilder<'a> {
    files: Vec<FileData<'a>>,
}

impl<'a> BundleBuilder<'a> {
    /// Create this builder from a GResource XML file
    pub fn from_xml(xml: super::xml::XmlManifest) -> BuilderResult<Self> {
        let mut files = Vec::new();

        for gresource in &xml.gresources {
            for file in &gresource.files {
                let mut key = gresource.prefix.clone();
                if !key.ends_with('/') {
                    key.push('/');
                }

                if let Some(alias) = &file.alias {
                    key.push_str(alias);
                } else {
                    key.push_str(&file.filename);
                }

                let mut filename = xml.dir.clone();
                filename.push(PathBuf::from(&file.filename));

                let file_data =
                    FileData::from_file(key, &filename, file.compressed, &file.preprocess)?;
                files.push(file_data);
            }
        }

        Ok(Self { files })
    }

    /// Scan a directory and create a GResource file with all the contents of the directory.
    ///
    /// This will ignore any files that end with gresource.xml and meson.build, as
    /// those are most likely not needed inside the GResource.
    ///
    /// This is equivalent to the following XML:
    ///
    /// ```xml
    /// <gresources>
    ///   <gresource prefix="`prefix`">
    ///     <!-- file entries for each file with path beginning from `directory` as root -->
    ///   </gresource>
    /// </gresources>
    /// ```
    ///
    /// ## `prefix`
    ///
    /// The prefix for the gresource section
    ///
    /// ## `directory`
    ///
    /// The root directory of the included files
    ///
    /// ## `strip_blanks`
    ///
    /// Acts as if every xml file uses the option `xml-stripblanks` in the GResource XML and every
    /// JSON file uses `json-stripblanks`.
    ///
    /// JSON files are all files with the extension '.json'.
    /// XML files are all files with the extensions '.xml', '.ui', '.svg'
    ///
    /// ## `compress`
    ///
    /// Compresses all files that end with the preconfigured patterns.
    /// Compressed files are currently: ".ui", ".css"
    pub fn from_directory(
        prefix: &str,
        directory: &Path,
        strip_blanks: bool,
        compress: bool,
    ) -> BuilderResult<Self> {
        let compress_extensions = if compress {
            COMPRESS_EXTENSIONS_DEFAULT
        } else {
            &[]
        };

        Self::from_directory_with_extensions(
            prefix,
            directory,
            strip_blanks,
            compress_extensions,
            SKIPPED_FILE_EXTENSIONS_DEFAULT,
        )
    }

    /// Like `from_directory` but allows you to specify the extensions directories yourself
    ///
    /// ## `compress_extensions`
    ///
    /// All files that end with these strings will get compressed
    ///
    /// ## `skipped_file_extensions`
    ///
    /// Skip all files that end with this string
    pub fn from_directory_with_extensions(
        prefix: &str,
        directory: &Path,
        strip_blanks: bool,
        compress_extensions: &[&str],
        skipped_file_extensions: &[&str],
    ) -> BuilderResult<Self> {
        let mut prefix = prefix.to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }

        let mut files = Vec::new();

        'outer: for res in WalkDir::new(directory).into_iter() {
            let entry = match res {
                Ok(entry) => entry,
                Err(err) => {
                    let path = err.path().map(|p| p.to_path_buf());
                    Err(BuilderError::Io(err.into(), path))?
                }
            };

            if entry.path().is_file() {
                let filename: &str = match entry.file_name().try_into() {
                    Ok(name) => name,
                    Err(err) => return Err(BuilderError::Utf8(err, Some(entry.path().to_owned()))),
                };

                for name in skipped_file_extensions {
                    if filename.ends_with(name) {
                        continue 'outer;
                    }
                }

                let mut compress_this = false;

                for name in compress_extensions {
                    if filename.ends_with(name) {
                        compress_this = true;
                        break;
                    }
                }

                let file_abs_path = entry.path();
                let file_path_relative = match file_abs_path.strip_prefix(directory) {
                    Ok(path) => path,
                    Err(err) => {
                        return Err(BuilderError::StripPrefix(err, file_abs_path.to_owned()))
                    }
                };

                let file_path_str_relative: &str = match file_path_relative.as_os_str().try_into() {
                    Ok(name) => name,
                    Err(err) => {
                        return Err(BuilderError::Utf8(err, Some(file_path_relative.to_owned())))
                    }
                };

                let options = if strip_blanks && file_path_str_relative.ends_with(".json") {
                    PreprocessOptions::json_stripblanks()
                } else if strip_blanks && file_path_str_relative.ends_with(".xml")
                    || file_path_str_relative.ends_with(".ui")
                    || file_path_str_relative.ends_with(".svg")
                {
                    PreprocessOptions::xml_stripblanks()
                } else {
                    PreprocessOptions::empty()
                };

                let key = format!("{}{}", prefix, file_path_str_relative);
                let file_data = FileData::from_file(key, file_abs_path, compress_this, &options)?;
                files.push(file_data);
            }
        }

        // Make sure the files are sorted in a reproducible way to ensure reproducible builds
        files.sort();

        Ok(Self { files })
    }

    /// Create a new Builder from a `Vec<FileData>`.
    ///
    /// This is the most flexible way to create a GResource file, but also the most hands-on.
    pub fn from_file_data(files: Vec<FileData<'a>>) -> Self {
        Self { files }
    }

    /// Build the binary GResource data
    pub fn build(self) -> BuilderResult<Vec<u8>> {
        let builder = FileWriter::new();
        let mut table_builder = HashTableBuilder::new();

        for file_data in self.files.into_iter() {
            let data = Data {
                size: file_data.size,
                flags: file_data.flags,
                data: file_data.data.to_vec(),
            };

            table_builder.insert_value(file_data.key(), zvariant::Value::from(data))?;
        }

        Ok(builder.write_to_vec_with_table(table_builder)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gresource::xml::XmlManifest;
    use crate::read::File;
    use crate::test::{assert_is_file_3, byte_compare_file_3, GRESOURCE_DIR, GRESOURCE_XML};
    use matches::assert_matches;
    use zvariant::Type;

    #[test]
    fn file_data() {
        let doc = XmlManifest::from_file(&GRESOURCE_XML).unwrap();
        let builder = BundleBuilder::from_xml(doc).unwrap();

        for file in &builder.files {
            assert!(file.key().starts_with("/gvdb/rs/test"));

            assert!(
                [
                    "/gvdb/rs/test/online-symbolic.svg",
                    "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
                    "/gvdb/rs/test/json/test.json",
                    "/gvdb/rs/test/test.css"
                ]
                .contains(&file.key()),
                "Unknown file with key: {}",
                file.key()
            );

            // Make sure the Eq implementation works as expected
            for file2 in &builder.files {
                if std::ptr::eq(file as *const FileData, file2 as *const FileData) {
                    assert_eq!(file, file2);
                } else {
                    assert_ne!(file, file2);
                }
            }
        }
    }

    #[test]
    fn from_dir_file_data() {
        for preprocess in [true, false] {
            let builder = BundleBuilder::from_directory(
                "/gvdb/rs/test",
                &GRESOURCE_DIR,
                preprocess,
                preprocess,
            )
            .unwrap();

            for file in builder.files {
                assert!(file.key().starts_with("/gvdb/rs/test"));

                assert!(
                    [
                        "/gvdb/rs/test/icons/scalable/actions/online-symbolic.svg",
                        "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
                        "/gvdb/rs/test/json/test.json",
                        "/gvdb/rs/test/test.css",
                        "/gvdb/rs/test/test3.gresource.xml"
                    ]
                    .contains(&file.key()),
                    "Unknown file with key: {}",
                    file.key()
                );
            }
        }
    }

    #[test]
    fn from_dir_invalid() {
        let res = BundleBuilder::from_directory(
            "/gvdb/rs/test",
            &PathBuf::from("INVALID_DIR"),
            false,
            false,
        );

        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_matches!(err, BuilderError::Io(..));
    }

    #[test]
    fn test_file_3() {
        let doc = XmlManifest::from_file(&GRESOURCE_XML).unwrap();
        let builder = BundleBuilder::from_xml(doc).unwrap();
        let data = builder.build().unwrap();
        let root = File::from_bytes(Cow::Owned(data)).unwrap();

        assert_is_file_3(&root);
        byte_compare_file_3(&root);
    }

    #[test]
    fn to_pixdata() {
        let path = GRESOURCE_DIR.join("json").join("test.json");
        let mut options = PreprocessOptions::empty();
        options.to_pixdata = true;
        let err = FileData::from_file("test.json".to_string(), &path, false, &options).unwrap_err();
        assert_matches!(err, BuilderError::Unimplemented(_));
        assert!(format!("{}", err).contains("to-pixdata is deprecated"));
    }

    #[test]
    fn xml_stripblanks() {
        for path in [Some(PathBuf::from("test")), None] {
            let xml = "<invalid";
            let err = FileData::new(
                "test".to_string(),
                Cow::Borrowed(xml.as_bytes()),
                path,
                false,
                &PreprocessOptions::xml_stripblanks(),
            )
            .unwrap_err();

            assert_matches!(err, BuilderError::Xml(_, _));
            assert!(format!("{}", err).contains("Error processing XML data"));
        }
    }

    #[test]
    fn json_stripblanks() {
        for path in [Some(PathBuf::from("test")), None] {
            let invalid_utf8 = [0xC3, 0x28];
            let err = FileData::new(
                "test".to_string(),
                Cow::Borrowed(&invalid_utf8),
                path.clone(),
                false,
                &PreprocessOptions::json_stripblanks(),
            )
            .unwrap_err();

            assert_matches!(err, BuilderError::Utf8(..));
            assert!(format!("{:?}", err).contains("UTF-8"));

            let invalid_json = r#"{ "test": : }"#.as_bytes();
            let err = FileData::new(
                "test".to_string(),
                Cow::Borrowed(invalid_json),
                path,
                false,
                &PreprocessOptions::json_stripblanks(),
            )
            .unwrap_err();

            assert_matches!(err, BuilderError::Json(..));
            assert!(format!("{:?}", err).contains("expected value at line"));
        }

        let valid_json = r#"{ "test": "test" }"#.as_bytes();
        let data = FileData::new(
            "test".to_string(),
            Cow::Borrowed(valid_json),
            None,
            false,
            &PreprocessOptions::json_stripblanks(),
        )
        .unwrap();

        let json = std::str::from_utf8(&data.data).unwrap();
        assert_eq!(json, "{\"test\":\"test\"}\n\0");
    }

    #[test]
    fn derives_data() {
        let data = Data {
            size: 3,
            flags: 0,
            data: vec![1, 2, 3],
        };

        let sig = Data::signature();
        assert_eq!(sig, "(uuay)");
        let owned = zvariant::OwnedValue::try_from(data).unwrap();
        let data = Data::try_from(owned).unwrap();
        let value: zvariant::Value = data.into();
        let _: Data = value.try_into().unwrap();
    }
}
