//! This crate offers convenience macros for [gvdb](https://!github.com/felinira/gvdb-rs).
//! The macros are [`include_gresource_from_xml!()`] and
//! [`include_gresource_from_dir!()`]
//!
//! ## Examples
//!
//! Compile a GResource XML file and include the bytes in the file.
//!
//! ```
//! use gvdb_macros::include_gresource_from_xml;
//! static GRESOURCE_BYTES: &[u8] = include_gresource_from_xml!("test/test3.gresource.xml");
//! ```
//!
//! Scan a directory and create a GResource file with all the contents of the directory.
//!
//! ```
//! use gvdb_macros::include_gresource_from_dir;
//! static GRESOURCE_BYTES: &[u8] = include_gresource_from_dir!("/gvdb/rs/test", "test/");
//! ```

#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

extern crate proc_macro;

use std::path::PathBuf;
use litrs::{Literal, StringLit};
use proc_macro2::TokenTree;
use quote::quote;

fn quote_bytes(bytes: &[u8]) -> proc_macro2::TokenStream {
    let bytes_lit = proc_macro2::Literal::byte_string(bytes);

    quote! {
        {{
            #[repr(C, align(16))]
            struct __Aligned<T: ?Sized>(T);

            static __DATA: &'static __Aligned<[u8]> = &__Aligned(*#bytes_lit);

            &__DATA.0
        }}
    }
}

fn include_gresource_from_xml_with_filename(filename: &str) -> proc_macro2::TokenStream {
    let path = PathBuf::from(filename);
    let xml = gvdb::gresource::GResourceXMLDocument::from_file(&path).unwrap();
    let builder = gvdb::gresource::GResourceBuilder::from_xml(xml).unwrap();
    let data = builder.build().unwrap();

    quote_bytes(&data)
}

fn include_gresource_from_xml_inner(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let mut iter = input.into_iter();

    let first = iter.next().expect("Expected exactly one string literal argument (gresource file location)");
    let second = iter.next();
    if let Some(second) = second {
        panic!("Unexpected token '{}', expected exactly one string literal argument (gresource file location)", second)
    }

    match Literal::try_from(first) {
        Err(e) => proc_macro2::TokenStream::from(e.to_compile_error()),
        Ok(Literal::String(str)) => {
            include_gresource_from_xml_with_filename(str.value())
        }
        Ok(other) => panic!("Unexpected token '{:?}', expected exactly one string literal argument (gresource file location)", other)
    }
}

/// Compile a GResource XML file to its binary representation and include it in the source file.
///
/// ```
/// use gvdb_macros::include_gresource_from_xml;
/// static GRESOURCE_BYTES: &[u8] = include_gresource_from_xml!("test/test3.gresource.xml");
/// ```
#[proc_macro]
pub fn include_gresource_from_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let output = include_gresource_from_xml_inner(input);
    proc_macro::TokenStream::from(output)
}

fn include_gresource_from_dir_str(prefix: &str, directory: &str) -> proc_macro2::TokenStream {
    let path = PathBuf::from(directory);
    let builder = gvdb::gresource::GResourceBuilder::from_directory(prefix, &path, true, true).unwrap();
    let data = builder.build().unwrap();

    quote_bytes(&data)
}

fn include_gresource_from_dir_inner(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let err_msg = "expected exactly two string literal arguments (prefix, gresource directory)";
    let (prefix, directory) = match &*input.into_iter().collect::<Vec<_>>() {
        [TokenTree::Literal(str1), TokenTree::Punct(comma), TokenTree::Literal(str2)] => {
            if comma.as_char() != ',' {
                panic!("{}", err_msg);
            }

            (StringLit::try_from(str1).expect(err_msg), StringLit::try_from(str2).expect(err_msg))
        }
        _ => panic!("{}", err_msg),
    };

    include_gresource_from_dir_str(prefix.value(), directory.value())
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
/// The first argument to this macro is the prefix for the GResource file. The second argument is
/// the path to the folder containing the files to include in the file.
///
/// This acts as if every xml file uses the option `xml-stripblanks` in the GResource XML and every
/// JSON file uses `json-stripblanks`.
///
/// JSON files are all files with the extension '.json'.
/// XML files are all files with the extensions '.xml', '.ui', '.svg'
///
/// All files that end with `.ui` and `.css` are compressed.
/// ```
/// use gvdb_macros::include_gresource_from_dir;
/// static GRESOURCE_BYTES: &[u8] = include_gresource_from_dir!("/gvdb/rs/test", "test/");
/// ```
#[proc_macro]
pub fn include_gresource_from_dir(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
   let input = proc_macro2::TokenStream::from(input);
   let output = include_gresource_from_dir_inner(input);
    proc_macro::TokenStream::from(output)
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use super::*;

    #[test]
    fn include_gresource_from_xml() {
        let tokens = include_gresource_from_xml_inner(quote! {"test/test3.gresource.xml"});
        assert!(tokens.to_string().contains(r#"b"GVariant"#));
    }

    #[test]
    #[should_panic]
    fn include_gresource_from_xml_panic() {
        include_gresource_from_xml_inner(quote! {4});
    }

    #[test]
    fn include_gresource_from_dir() {
        let tokens = include_gresource_from_dir_inner(quote! {"/gvdb/rs/test", "test"});
        assert!(tokens.to_string().contains(r#"b"GVariant"#));
    }

    #[test]
    #[should_panic]
    fn include_gresource_from_dir_panic1() {
        include_gresource_from_dir_inner(quote! {"/gvdb/rs/test",});
    }

    #[test]
    #[should_panic]
    fn include_gresource_from_dir_panic2() {
        include_gresource_from_dir_inner(quote! {"/gvdb/rs/test"});
    }

    #[test]
    #[should_panic]
    fn include_gresource_from_dir_panic3() {
        include_gresource_from_dir_inner(quote! {"/gvdb/rs/test","bla","bla"});
    }

    #[test]
    #[should_panic]
    fn include_gresource_from_dir_panic4() {
        include_gresource_from_dir_inner(quote! {"/gvdb/rs/test","INVALID_DIRECTORY"});
    }
}
