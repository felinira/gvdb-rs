mod error;

pub use error::*;

use serde::Deserialize;
use serde::de::Error;
use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A GResource XML document
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct XmlManifest {
    /// The list of GResource sections
    #[serde(rename = "gresource")]
    pub gresources: Vec<GResource>,

    /// The directory of the XML file
    #[serde(default)]
    pub dir: PathBuf,
}

/// A GResource section inside a GResource XML document
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct GResource {
    /// The files for this GResource section
    #[serde(rename = "file", default)]
    pub files: Vec<File>,

    /// An optional prefix to prepend to the containing file keys
    #[serde(default, rename = "@prefix")]
    pub prefix: String,
}

/// A file within a GResource section
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct File {
    /// The on-disk file name of the file
    #[serde(rename = "$value")]
    pub filename: String,

    /// The alias for this file if it should be named differently inside the GResource file
    #[serde(rename = "@alias")]
    pub alias: Option<String>,

    /// Whether the file should be compressed using zlib
    #[serde(deserialize_with = "parse_bool_value", default, rename = "@compressed")]
    pub compressed: bool,

    /// A list of preprocessing options
    #[serde(
        deserialize_with = "parse_preprocess_options",
        default,
        rename = "@preprocess"
    )]
    pub preprocess: PreprocessOptions,
}

/// Preprocessing options for files that will be put in a GResource
#[derive(Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PreprocessOptions {
    /// Strip whitespace from XML file
    pub xml_stripblanks: bool,

    /// Unimplemented
    pub to_pixdata: bool,

    /// Strip whitespace from JSON file
    pub json_stripblanks: bool,
}

impl PreprocessOptions {
    /// An empty set of preprocessing options
    ///
    /// No preprocessing will be done
    pub fn empty() -> Self {
        Self {
            xml_stripblanks: false,
            to_pixdata: false,
            json_stripblanks: false,
        }
    }

    /// XML strip blanks preprocessing will be applied
    pub fn xml_stripblanks() -> Self {
        Self {
            xml_stripblanks: true,
            to_pixdata: false,
            json_stripblanks: false,
        }
    }

    /// JSON strip blanks preprocessing will be applied
    pub fn json_stripblanks() -> Self {
        Self {
            xml_stripblanks: false,
            to_pixdata: false,
            json_stripblanks: true,
        }
    }
}

fn parse_bool_value<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match &*String::deserialize(d)? {
        "true" | "t" | "yes" | "y" | "1" => Ok(true),
        "false" | "f" | "no" | "n" | "0" => Ok(false),
        other => Err(D::Error::custom(format!(
            "got '{other}', but expected any of 'true', 't', 'yes', 'y', '1' / 'false', 'f', 'no', 'n', '0'"
        ))),
    }
}

fn parse_preprocess_options<'de, D>(d: D) -> Result<PreprocessOptions, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut this = PreprocessOptions::default();

    for item in String::deserialize(d)?.split(',') {
        match item {
            "json-stripblanks" => this.json_stripblanks = true,
            "xml-stripblanks" => this.xml_stripblanks = true,
            "to-pixdata" => this.to_pixdata = true,
            other => {
                return Err(D::Error::custom(format!(
                    "got '{other}' but expected any of 'json-stripblanks', 'xml-stripblanks'"
                )));
            }
        }
    }

    Ok(this)
}

impl XmlManifest {
    /// Load a GResource XML file from disk using `path`
    pub fn from_file(path: &Path) -> error::XmlManifestResult<Self> {
        let mut file = std::fs::File::open(path)
            .map_err(error::XmlManifestError::from_io_with_filename(path))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(error::XmlManifestError::from_io_with_filename(path))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(error::XmlManifestError::from_io_with_filename(path))?;

        let dir = path.parent().unwrap();
        Self::from_bytes_with_filename(dir, Some(path.to_path_buf()), Cow::Owned(data))
    }

    /// Load a GResource XML file from the provided `Cow<[u8]>` bytes. A filename is provided for
    /// error context
    fn from_bytes_with_filename(
        dir: &Path,
        filename: Option<PathBuf>,
        data: Cow<'_, [u8]>,
    ) -> error::XmlManifestResult<Self> {
        let mut this: Self = quick_xml::de::from_str(
            std::str::from_utf8(&data)
                .map_err(|err| error::XmlManifestError::Utf8(err, filename.clone()))?,
        )
        .map_err(|err| error::XmlManifestError::Serde(err, filename))?;

        this.dir = dir.to_path_buf();
        Ok(this)
    }

    /// Load a GResource XML file from the provided `Cow<[u8]>` bytes
    pub fn from_bytes(dir: &Path, data: Cow<'_, [u8]>) -> error::XmlManifestResult<Self> {
        Self::from_bytes_with_filename(dir, None, data)
    }

    /// Load a GResource XML file from a `&str` or `String`
    pub fn from_string(dir: &Path, str: impl ToString) -> error::XmlManifestResult<Self> {
        Self::from_bytes(dir, Cow::Borrowed(str.to_string().as_bytes()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use matches::assert_matches;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserialize_simple() {
        let test_path = PathBuf::from("/TEST");

        let data = r#"<gresources><gresource><file compressed="false" preprocess="xml-stripblanks">test</file></gresource></gresources>"#;
        let doc = XmlManifest::from_bytes(&test_path, Cow::Borrowed(data.as_bytes())).unwrap();
        println!("{doc:?}");
        assert_eq!(doc, doc);
        assert_eq!(doc.gresources.len(), 1);
        assert_eq!(doc.gresources[0].files.len(), 1);
        assert_eq!(doc.gresources[0].files[0].filename, "test");
        assert_eq!(doc.gresources[0].files[0].preprocess.xml_stripblanks, true);
        assert_eq!(
            doc.gresources[0].files[0].preprocess.json_stripblanks,
            false
        );
        assert_eq!(doc.gresources[0].files[0].preprocess.to_pixdata, false);
        assert_eq!(doc.gresources[0].files[0].compressed, false);
    }

    #[test]
    fn deserialize_complex() {
        let test_path = PathBuf::from("/TEST");

        let data = r#"<gresources><gresource prefix="/bla/blub"><file compressed="true" preprocess="json-stripblanks,to-pixdata">test.json</file></gresource></gresources>"#;
        let doc = XmlManifest::from_bytes(&test_path, Cow::Borrowed(data.as_bytes())).unwrap();
        assert_eq!(doc.gresources.len(), 1);
        assert_eq!(doc.gresources[0].files.len(), 1);
        assert_eq!(doc.gresources[0].files[0].filename, "test.json");
        assert_eq!(doc.gresources[0].files[0].compressed, true);
        assert_eq!(doc.gresources[0].files[0].preprocess.json_stripblanks, true);
        assert_eq!(doc.gresources[0].files[0].preprocess.to_pixdata, true);
        assert_eq!(doc.gresources[0].files[0].preprocess.xml_stripblanks, false);
        assert_eq!(doc.gresources[0].prefix, "/bla/blub")
    }

    #[test]
    fn deserialize_fail() {
        let test_path = PathBuf::from("/TEST");

        let res = XmlManifest::from_string(&test_path, r#"<wrong></wrong>"#);
        assert!(format!("{res:?}").contains("parsing XML"));
        assert_matches!(
            res,
            Err(error::XmlManifestError::Serde(quick_xml::DeError::Custom(field), _)) if field == "missing field `gresource`"
        );

        let string = r#"<gresources><gresource><file></file></gresource></gresources>"#.to_string();
        let res = XmlManifest::from_bytes_with_filename(
            &test_path,
            Some(PathBuf::from("test_filename")),
            Cow::Borrowed(string.as_bytes()),
        );
        assert!(format!("{res:?}").contains("test_filename"));
        assert_matches!(
            res,
            Err(error::XmlManifestError::Serde(quick_xml::de::DeError::Custom(field), _)) if field == "missing field `$value`"
        );

        assert_matches!(
            XmlManifest::from_string(&test_path, r#"<gresources><gresource><file compressed="nobool">filename</file></gresource></gresources>"#),
            Err(error::XmlManifestError::Serde(quick_xml::de::DeError::Custom(field), _)) if field.starts_with("got 'nobool', but expected any of")
        );

        assert_matches!(
            XmlManifest::from_string(&test_path, r#"<gresources><wrong></wrong></gresources>"#),
            Err(error::XmlManifestError::Serde(quick_xml::de::DeError::Custom(field), _))if field.starts_with("unknown field `wrong`, expected `gresource`")
        );

        assert_matches!(
            XmlManifest::from_string(&test_path, r#"<gresources><gresource><wrong>filename</wrong></gresource></gresources>"#),
            Err(error::XmlManifestError::Serde(quick_xml::de::DeError::Custom(field), _)) if field.starts_with("unknown field `wrong`, expected `file` or `@prefix`")
        );

        assert_matches!(
            XmlManifest::from_string(&test_path, r#"<gresources><gresource><file wrong="1">filename</file></gresource></gresources>"#),
            Err(error::XmlManifestError::Serde(quick_xml::de::DeError::Custom(field), _)) if field.starts_with("unknown field `@wrong`, expected one of")
        );

        assert_matches!(
            XmlManifest::from_string(&test_path, r#"<gresources><gresource><file preprocess="fail">filename</file></gresource></gresources>"#),
            Err(error::XmlManifestError::Serde(quick_xml::de::DeError::Custom(field), _)) if field.starts_with("got 'fail' but expected any of")
        );

        let res = XmlManifest::from_bytes(&test_path, Cow::Borrowed(&[0x80, 0x81])).unwrap_err();

        println!("{res}");
        assert_matches!(res, error::XmlManifestError::Utf8(..));
    }

    #[test]
    fn io_error() {
        let test_path = PathBuf::from("invalid_file_name.xml");
        let res = XmlManifest::from_file(&test_path);
        assert_matches!(res, Err(error::XmlManifestError::Io(_, _)));
        assert!(format!("{res:?}").contains("invalid_file_name.xml"));
    }
}
