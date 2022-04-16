use crate::gresource::error::{GResourceBuilderError, GResourceBuilderResult};
use crate::gresource::xml::PreprocessOptions;
use crate::gvdb::write::builder::{GvdbFileWriter, GvdbHashTableBuilder};
use flate2::write::ZlibEncoder;
use glib::ToVariant;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use xml::{EmitterConfig, EventReader, EventWriter, ParserConfig};

#[cfg(test)]
const BYTE_COMPATIBILITY: bool = true;
#[cfg(not(test))]
const BYTE_COMPATIBILITY: bool = false;

const FLAG_COMPRESSED: u32 = 1 << 0;

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
        } else if BYTE_COMPATIBILITY {
            data.to_mut().push(0);
        }

        Ok(Self {
            key,
            data,
            flags,
            size,
        })
    }

    pub fn xml_stripblanks(
        data: Cow<'a, [u8]>,
        path: &Path,
    ) -> GResourceBuilderResult<Cow<'a, [u8]>> {
        if BYTE_COMPATIBILITY {
            return Err(GResourceBuilderError::Unimplemented(
                "xml-stripblanks can't create byte-compatible files to glib-compile-resources yet"
                    .to_string(),
            ));
        }

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

        if BYTE_COMPATIBILITY {
            output.push(b'\n');
        }

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

pub struct GResourceBuilder<'a> {
    files: Vec<FileData<'a>>,
}

impl<'a> GResourceBuilder<'a> {
    pub fn from_xml(xml: super::xml::GResourceXMLDoc) -> GResourceBuilderResult<Self> {
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

                let mut open_file = std::fs::File::open(&filename)
                    .map_err(|err| GResourceBuilderError::Io(err, Some(filename.to_path_buf())))?;
                let mut data = Vec::new();
                open_file
                    .read_to_end(&mut data)
                    .map_err(|err| GResourceBuilderError::Io(err, Some(filename.to_path_buf())))?;
                let file_data = FileData::new(
                    key,
                    Cow::Owned(data),
                    &filename,
                    file.compressed,
                    &file.preprocess,
                )?;
                files.push(file_data);
            }
        }

        Ok(Self { files })
    }

    pub fn build(self) -> GResourceBuilderResult<Vec<u8>> {
        #[cfg(target_endian = "big")]
        let byteswap = true;
        #[cfg(target_endian = "little")]
        let byteswap = false;

        let builder = GvdbFileWriter::new(byteswap);
        let mut table_builder = GvdbHashTableBuilder::new();

        for file_data in self.files {
            let tuple = vec![
                file_data.size().to_variant(),
                file_data.flags().to_variant(),
                glib::Bytes::from(file_data.data()).to_variant(),
            ];
            let variant = glib::Variant::tuple_from_iter(tuple);

            table_builder.insert_variant(file_data.key(), variant)?;
        }

        Ok(builder.write_into_vec_with_table(table_builder)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gresource::xml::GResourceXMLDoc;
    use crate::gvdb::read::file::test::{assert_is_file_3, byte_compare_file_3};
    use crate::gvdb::read::file::GvdbFile;

    const GRESOURCE_XML: &str = "test/data/gresource/test3.gresource.xml";

    #[test]
    fn file_data() {
        let doc = GResourceXMLDoc::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
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
    fn test_file_3() {
        let doc = GResourceXMLDoc::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
        let builder = GResourceBuilder::from_xml(doc).unwrap();
        let data = builder.build().unwrap();
        let root = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();
        assert_is_file_3(&root);
        byte_compare_file_3(&root);
    }
}
