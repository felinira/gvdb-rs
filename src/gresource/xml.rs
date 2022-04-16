use crate::gresource::error::{GResourceXMLError, GResourceXMLResult};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde_xml_rs::{Deserializer, EventReader, ParserConfig};
use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct GResourceXMLDoc {
    #[serde(rename = "gresource")]
    pub gresources: Vec<GResource>,

    #[serde(default)]
    pub dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct GResource {
    #[serde(rename = "file", default)]
    pub files: Vec<File>,

    #[serde(default)]
    pub prefix: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct File {
    #[serde(rename = "$value")]
    pub filename: String,

    pub alias: Option<String>,

    #[serde(deserialize_with = "parse_bool_value", default)]
    pub compressed: bool,

    #[serde(deserialize_with = "parse_preprocess_options", default)]
    pub preprocess: PreprocessOptions,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PreprocessOptions {
    pub xml_stripblanks: bool,
    pub to_pixdata: bool,
    pub json_stripblanks: bool,
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

impl GResourceXMLDoc {
    pub fn from_file(path: &Path) -> GResourceXMLResult<Self> {
        let mut file = std::fs::File::open(path)
            .map_err(|err| GResourceXMLError::IO(err, Some(path.to_path_buf())))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(|err| GResourceXMLError::IO(err, Some(path.to_path_buf())))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(|err| GResourceXMLError::IO(err, Some(path.to_path_buf())))?;

        let dir = path.parent().unwrap();
        Self::from_bytes(dir, Cow::Owned(data))
    }

    pub fn from_bytes(dir: &Path, data: Cow<'_, [u8]>) -> GResourceXMLResult<Self> {
        let config = ParserConfig::new()
            .trim_whitespace(true)
            .ignore_comments(true);
        let event_reader = EventReader::new_with_config(&*data, config);

        let mut this = Self::deserialize(&mut Deserializer::new(event_reader))?;
        this.dir = dir.to_path_buf();
        Ok(this)
    }

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
        let doc = GResourceXMLDoc::from_bytes(&test_path, Cow::Borrowed(data.as_bytes())).unwrap();
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
        let doc = GResourceXMLDoc::from_bytes(&test_path, Cow::Borrowed(data.as_bytes())).unwrap();
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
            GResourceXMLDoc::from_string(&test_path, r#"<wrong></wrong>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field == "missing field `gresource`"
        );

        assert_matches!(
            GResourceXMLDoc::from_string(&test_path, r#"<gresources><gresource><file></file></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field == "missing field `$value`"
        );

        assert_matches!(
            GResourceXMLDoc::from_string(&test_path, r#"<gresources><gresource><file compressed="nobool">filename</file></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("got 'nobool', but expected any of")
        );

        assert_matches!(
            GResourceXMLDoc::from_string(&test_path, r#"<gresources><wrong></wrong></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("unknown field `wrong`, expected `gresource`")
        );

        assert_matches!(
            GResourceXMLDoc::from_string(&test_path, r#"<gresources><gresource><wrong>filename</wrong></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("unknown field `wrong`, expected `file` or `prefix`")
        );

        assert_matches!(
            GResourceXMLDoc::from_string(&test_path, r#"<gresources><gresource><file wrong="1">filename</file></gresource></gresources>"#),
            Err(GResourceXMLError::Serde(serde_xml_rs::Error::Custom {
                field
            }, _)) if field.starts_with("unknown field `wrong`, expected one of")
        );
    }
}
