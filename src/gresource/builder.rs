use crate::gresource::error::{GResourceBuilderError, GResourceBuilderResult};
use crate::gresource::xml::PreprocessOptions;
use crate::write::{GvdbFileWriter, GvdbHashTableBuilder};
use flate2::write::ZlibEncoder;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use xml::{EmitterConfig, EventReader, EventWriter, ParserConfig};

#[cfg(not(feature = "glib"))]
use crate::no_glib::{ToVariant, Variant};
#[cfg(feature = "glib")]
use glib::{ToVariant, Variant};
use walkdir::WalkDir;

const FLAG_COMPRESSED: u32 = 1 << 0;
const SKIPPED_FILE_NAMES: &[&str] = &["meson.build", "gresource.xml"];

struct FileData<'a> {
    key: String,
    data: Cow<'a, [u8]>,
    flags: u32,
    size: u32,
}

impl<'a> FileData<'a> {
    pub fn new(
        key: String,
        data: Cow<'a, [u8]>,
        path: &Path,
        compressed: bool,
        preprocess: &PreprocessOptions,
    ) -> GResourceBuilderResult<Self> {
        let mut flags = 0;
        let mut data = Self::preprocess(data, preprocess, path)?;
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

    pub fn from_file(
        key: String,
        file_path: &Path,
        compressed: bool,
        preprocess: &PreprocessOptions,
    ) -> GResourceBuilderResult<Self> {
        let mut open_file = std::fs::File::open(&file_path)
            .map_err(|err| GResourceBuilderError::Io(err, Some(file_path.to_path_buf())))?;
        let mut data = Vec::new();
        open_file
            .read_to_end(&mut data)
            .map_err(|err| GResourceBuilderError::Io(err, Some(file_path.to_path_buf())))?;
        FileData::new(key, Cow::Owned(data), &file_path, compressed, preprocess)
    }

    pub fn xml_stripblanks(
        data: Cow<'a, [u8]>,
        path: &Path,
    ) -> GResourceBuilderResult<Cow<'a, [u8]>> {
        let mut output = Vec::new();

        let reader_config = ParserConfig::new()
            .trim_whitespace(true)
            .ignore_comments(true);
        let event_reader = EventReader::new_with_config(&*data, reader_config);

        let writer_config = EmitterConfig::new()
            .perform_indent(false)
            .line_separator("\n");
        let mut event_writer = EventWriter::new_with_config(&mut output, writer_config);

        for event in event_reader {
            if let Some(writer_event) = event
                .map_err(|err| GResourceBuilderError::XmlRead(err, Some(path.to_path_buf())))?
                .as_writer_event()
            {
                event_writer.write(writer_event).map_err(|err| {
                    GResourceBuilderError::XmlWrite(err, Some(path.to_path_buf()))
                })?;
            }
        }

        Ok(Cow::Owned(output))
    }

    pub fn json_stripblanks(
        data: Cow<'a, [u8]>,
        path: &Path,
    ) -> GResourceBuilderResult<Cow<'a, [u8]>> {
        let mut output = Vec::new();

        let json = json::parse(
            &String::from_utf8(data.to_vec())
                .map_err(|err| GResourceBuilderError::Utf8(err, Some(path.to_path_buf())))?,
        )
        .map_err(|err| GResourceBuilderError::Json(err, Some(path.to_path_buf())))?;
        json.write(&mut output)
            .map_err(|err| GResourceBuilderError::Io(err, Some(path.to_path_buf())))?;

        output.push(b'\n');

        Ok(Cow::Owned(output))
    }

    pub fn preprocess(
        mut data: Cow<'a, [u8]>,
        options: &PreprocessOptions,
        path: &Path,
    ) -> GResourceBuilderResult<Cow<'a, [u8]>> {
        if options.xml_stripblanks {
            data = Self::xml_stripblanks(data, path)?;
        }

        if options.json_stripblanks {
            data = Self::json_stripblanks(data, path)?;
        }

        if options.to_pixdata {
            return Err(GResourceBuilderError::Unimplemented(
                "to-pixdata is deprecated since gdk-pixbuf 2.32 and not supported by gvdb-rs"
                    .to_string(),
            ));
        }

        Ok(data)
    }

    pub fn compress(data: Cow<'a, [u8]>, path: &Path) -> GResourceBuilderResult<Cow<'a, [u8]>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder
            .write_all(&data)
            .map_err(|err| GResourceBuilderError::Io(err, Some(path.to_path_buf())))?;
        Ok(Cow::Owned(encoder.finish().map_err(|err| {
            GResourceBuilderError::Io(err, Some(path.to_path_buf()))
        })?))
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn data(&self) -> &Cow<'_, [u8]> {
        &self.data
    }

    /// uncompressed data is zero-terminated
    /// compressed data is not
    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }
}

/// Create a GResource binary file
///
/// # Example
///
/// Create a GResource XML file with [`GResourceXMLDocument`][crate::gresource::GResourceXMLDocument] and
/// [`GResourceBuilder`](crate::gresource::GResourceBuilder)
/// ```
/// use std::borrow::Cow;
/// use std::path::PathBuf;
/// use gvdb::gresource::GResourceBuilder;
/// use gvdb::gresource::GResourceXMLDocument;
/// use gvdb::read::GvdbFile;
///
/// const GRESOURCE_XML: &str = "test/data/gresource/test3.gresource.xml";
///
/// fn create_gresource() {
///     let doc = GResourceXMLDocument::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
///     let builder = GResourceBuilder::from_xml(doc).unwrap();
///     let data = builder.build().unwrap();
///     let root = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();
/// }
/// ```

pub struct GResourceBuilder<'a> {
    files: Vec<FileData<'a>>,
}

impl<'a> GResourceBuilder<'a> {
    /// Create this builder from a GResource XML file
    pub fn from_xml(xml: super::xml::GResourceXMLDocument) -> GResourceBuilderResult<Self> {
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
    /// Compresses all files
    pub fn from_directory(
        prefix: &str,
        directory: &Path,
        strip_blanks: bool,
        compress: bool,
    ) -> GResourceBuilderResult<Self> {
        let mut prefix = prefix.to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }

        let mut files = Vec::new();

        'outer: for entry in WalkDir::new(directory).into_iter().flatten() {
            if entry.path().is_file() {
                let filename = entry.file_name().to_str().ok_or_else(|| {
                    GResourceBuilderError::Generic(format!(
                        "Filename '{}' contains invalid UTF-8 characters",
                        entry.file_name().to_string_lossy()
                    ))
                })?;

                for name in SKIPPED_FILE_NAMES {
                    if filename.ends_with(name) {
                        continue 'outer;
                    }
                }

                let file_abs_path = entry.path();
                let file_path_relative = file_abs_path.strip_prefix(directory).map_err(|_| {
                    GResourceBuilderError::Generic("Strip prefix error".to_string())
                })?;
                let file_path_str_relative = file_path_relative.to_str().ok_or_else(|| {
                    GResourceBuilderError::Generic(format!(
                        "Filename '{}' contains invalid UTF-8 characters",
                        file_path_relative.display()
                    ))
                })?;

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
                let file_data = FileData::from_file(key, file_abs_path, compress, &options)?;
                files.push(file_data);
            }
        }

        Ok(Self { files })
    }

    /// Build the binary GResource data
    pub fn build(self) -> GResourceBuilderResult<Vec<u8>> {
        let builder = GvdbFileWriter::new();
        let mut table_builder = GvdbHashTableBuilder::new();

        for file_data in self.files {
            let tuple = vec![
                file_data.size().to_variant(),
                file_data.flags().to_variant(),
                file_data.data().to_variant(),
            ];
            let variant = Variant::tuple_from_iter(tuple);

            table_builder.insert_variant(file_data.key(), variant)?;
        }

        Ok(builder.write_to_vec_with_table(table_builder)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gresource::xml::GResourceXMLDocument;
    use crate::read::test::{assert_is_file_3, byte_compare_file_3};
    use crate::read::GvdbFile;

    const GRESOURCE_XML: &str = "test/data/gresource/test3.gresource.xml";
    const GRESOURCE_DIR: &str = "test/data/gresource";

    #[test]
    fn file_data() {
        let doc = GResourceXMLDocument::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
        let builder = GResourceBuilder::from_xml(doc).unwrap();

        for file in builder.files {
            assert!(file.key().starts_with("/gvdb/rs/test"));

            if !vec![
                "/gvdb/rs/test/online-symbolic.svg",
                "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
                "/gvdb/rs/test/json/test.json",
            ]
            .contains(&&*file.key())
            {
                panic!("Unknown file with key: {}", file.key())
            }
        }
    }

    #[test]
    fn from_dir_file_data() {
        let builder = GResourceBuilder::from_directory(
            "/gvdb/rs/test",
            &PathBuf::from(GRESOURCE_DIR),
            true,
            true,
        )
        .unwrap();

        for file in builder.files {
            assert!(file.key().starts_with("/gvdb/rs/test"));

            if !vec![
                "/gvdb/rs/test/icons/scalable/actions/online-symbolic.svg",
                "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
                "/gvdb/rs/test/json/test.json",
                "/gvdb/rs/test/test3.gresource.xml",
            ]
            .contains(&&*file.key())
            {
                panic!("Unknown file with key: {}", file.key())
            }
        }
    }

    #[test]
    fn test_file_3() {
        let doc = GResourceXMLDocument::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
        let builder = GResourceBuilder::from_xml(doc).unwrap();
        let data = builder.build().unwrap();
        let root = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();

        assert_is_file_3(&root);
        byte_compare_file_3(&root);
    }

    #[test]
    fn test_file_from_dir() {
        let builder = GResourceBuilder::from_directory(
            "/gvdb/rs/test",
            &PathBuf::from(GRESOURCE_DIR),
            true,
            true,
        )
        .unwrap();
        let data = builder.build().unwrap();
        let root = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();

        let table = root.hash_table().unwrap();
        let mut names = table.get_names().unwrap();
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
        ];
        assert_eq!(names, reference_names);

        let svg2 = table
            .get_value("/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg")
            .unwrap()
            .child_value(0);
        let svg2_size = svg2.child_value(0).get::<u32>().unwrap();
        let svg2_flags = svg2.child_value(1).get::<u32>().unwrap();
        let svg2_content: &[u8] = &svg2.child_value(2).data_as_bytes();

        assert_eq!(svg2_size, 339);
        assert_eq!(svg2_flags, 1);
        let mut decoder = flate2::read::ZlibDecoder::new(svg2_content);
        let mut svg2_data = Vec::new();
        decoder.read_to_end(&mut svg2_data).unwrap();

        // Ensure the last byte is *not* zero and len is not one bigger than specified because
        // compressed data is not zero-padded
        assert_ne!(svg2_data[svg2_data.len() - 1], 0);
        assert_eq!(svg2_size as usize, svg2_data.len());
    }
}
