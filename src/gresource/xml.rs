use crate::gresource::error::{GResourceXMLError, GResourceXMLResult};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde_xml_rs::{Deserializer, EventReader, ParserConfig};
use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A GResource XML document
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct GResourceXMLDocument {
    /// The list of GResource sections
    #[serde(rename = "gresource")]
    pub gresources: Vec<GResource>,

    /// The directory of the XML file
    #[serde(default)]
    pub dir: PathBuf,
}

/// A GResource section inside a GResource XML document
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct GResource {
    /// The files for this GResource section
    #[serde(rename = "file", default)]
    pub files: Vec<File>,

    /// An optional prefix to prepend to the containing file keys
    #[serde(default)]
    pub prefix: String,
}

/// A file within a GResource section
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct File {
    /// The on-disk file name of the file
    #[serde(rename = "$value")]
    pub filename: String,

    /// The alias for this file if it should be named differently inside the GResource file
    pub alias: Option<String>,

    /// Whether the file should be compressed using zlib
    #[serde(deserialize_with = "parse_bool_value", default)]
    pub compressed: bool,

    /// A list of preprocessing options
    #[serde(deserialize_with = "parse_preprocess_options", default)]
    pub preprocess: PreprocessOptions,
}

/// Preprocessing options for files that will be put in a GResource
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
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
        other => Err(D::Error::custom(format!("got '{}', but expected any of 'true', 't', 'yes', 'y', '1' / 'false', 'f', 'no', 'n', '0'", other))),
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
                    "got '{}' but expected any of 'json-stripblanks', 'xml-stripblanks'",
                    other
                )))
            }
        }
    }

    Ok(this)
}

impl GResourceXMLDocument {
    /// Load a GResource XML file from disk using `path`
    pub fn from_file(path: &Path) -> GResourceXMLResult<Self> {
        let mut file = std::fs::File::open(path)
            .map_err(|err| GResourceXMLError::Io(err, Some(path.to_path_buf())))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(|err| GResourceXMLError::Io(err, Some(path.to_path_buf())))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(|err| GResourceXMLError::Io(err, Some(path.to_path_buf())))?;

        let dir = path.parent().unwrap();
        Self::from_bytes(dir, Cow::Owned(data))
    }

    /// Load a GResource XML file from the provided `Cow<[u8]>` bytes
    pub fn from_bytes(dir: &Path, data: Cow<'_, [u8]>) -> GResourceXMLResult<Self> {
        let config = ParserConfig::new()
            .trim_whitespace(true)
            .ignore_comments(true);
        let event_reader = EventReader::new_with_config(&*data, config);

        let mut this = Self::deserialize(&mut Deserializer::new(event_reader))?;
        this.dir = dir.to_path_buf();
        Ok(this)
    }

    /// Load a GResource XML file from a `&str` or `String`
    pub fn from_string(dir: &Path, str: impl ToString) -> GResourceXMLResult<Self> {
        Self::from_bytes(dir, Cow::Borrowed(str.to_string().as_bytes()))
    }
}

#[cfg(test)]
mod test {
    use super::super::error::GResourceXMLError;
    use super::*;
    use matches::assert_matches;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_deserialize_simple() {
        let test_path = PathBuf::from("/TEST");

        let data = r#"<gresources><gresource><file compressed="false">test</file></gresource></gresources>"#;
        let doc =
            GResourceXMLDocument::from_bytes(&test_path, Cow::Borrowed(data.as_bytes())).unwrap();
        assert_eq!(doc.gresources.len(), 1);
        assert_eq!(doc.gresources[0].files.len(), 1);
        assert_eq!(doc.gresources[0].files[0].filename, "test");
        assert_eq!(doc.gresources[0].files[0].preprocess.xml_stripblanks, false);
        assert_eq!(
            doc.gresources[0].files[0].preprocess.json_stripblanks,
            false
        );
        assert_eq!(doc.gresources[0].files[0].preprocess.to_pixdata, false);
        assert_eq!(doc.gresources[0].files[0].compressed, false);
    }

    #[test]
    fn test_deserialize_complex() {
        let test_path = PathBuf::from("/TEST");

        let data = r#"<gresources><gresource prefix="/bla/blub"><file compressed="true" preprocess="json-stripblanks,to-pixdata">test.json</file></gresource></gresources>"#;
        let doc =
            GResourceXMLDocument::from_bytes(&test_path, Cow::Borrowed(data.as_bytes())).unwrap();
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
    fn test_deserialize_fail() {
        let test_path = PathBuf::from("/TEST");

        assert_matches!(
            GResourceXMLDocument::from_string(&test_path, r#"<wrong></wrong>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field == "missing field `gresource`"
        );

        assert_matches!(
            GResourceXMLDocument::from_string(&test_path, r#"<gresources><gresource><file></file></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field == "missing field `$value`"
        );

        assert_matches!(
            GResourceXMLDocument::from_string(&test_path, r#"<gresources><gresource><file compressed="nobool">filename</file></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("got 'nobool', but expected any of")
        );

        assert_matches!(
            GResourceXMLDocument::from_string(&test_path, r#"<gresources><wrong></wrong></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("unknown field `wrong`, expected `gresource`")
        );

        assert_matches!(
            GResourceXMLDocument::from_string(&test_path, r#"<gresources><gresource><wrong>filename</wrong></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("unknown field `wrong`, expected `file` or `prefix`")
        );

        assert_matches!(
            GResourceXMLDocument::from_string(&test_path, r#"<gresources><gresource><file wrong="1">filename</file></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("unknown field `wrong`, expected one of")
        );
    }
}
