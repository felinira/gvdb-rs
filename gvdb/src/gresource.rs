mod bundle;
mod xml;

pub use bundle::{BuilderError, BuilderResult, BundleBuilder, FileData};
pub use xml::{PreprocessOptions, XmlManifest, XmlManifestError, XmlManifestResult};

/// Deprecated type aliases
mod deprecated {
    use super::*;

    /// Type has been renamed. Use [`BundleBuilder`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::BundleBuilder instead."]
    pub type GResourceBuilder<'a> = BundleBuilder<'a>;

    /// Type has been renamed. Use [`FileData`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::FileData instead."]
    pub type GResourceFileData<'a> = FileData<'a>;

    /// DType has been renamed. Use [`XmlManifest`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::XmlManifest instead."]
    pub type GResourceXMLDocument = XmlManifest;

    /// Type has been renamed. Use [`BuilderError`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::BuilderError instead."]
    pub type GResourceBuilderError = BuilderError;

    /// Type has been renamed. Use [`BuilderResult`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::BuilderResult instead."]
    pub type GResourceBuilderResult<T> = BuilderResult<T>;

    /// Type has been renamed. Use [`XmlManifestError`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::XmlManifestError instead."]
    pub type GResourceXMLError = XmlManifestError;

    /// Type has been renamed. Use [`XmlManifestResult`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::gresource::XmlManifestResult instead."]
    pub type GResourceXMLResult<T> = XmlManifestResult<T>;
}

pub use deprecated::*;
