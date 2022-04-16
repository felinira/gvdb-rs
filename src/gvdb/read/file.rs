use crate::gvdb::read::error::{GvdbError, GvdbResult};
use crate::gvdb::read::hash::GvdbHashTable;
use crate::gvdb::read::hash_item::{GvdbHashItem, GvdbHashItemType};
use crate::gvdb::read::header::GvdbHeader;
use crate::gvdb::read::pointer::GvdbPointer;
use safe_transmute::{transmute_one, transmute_one_pedantic, transmute_one_to_bytes};
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[derive(Debug)]
pub struct GvdbFile<'a> {
    data: Cow<'a, [u8]>,
    byteswapped: bool,
}

impl<'a> GvdbFile<'a> {
    /// Get the GVDB file header. Will err with GvdbError::DataOffset if the header doesn't fit
    fn get_header(&self) -> GvdbResult<GvdbHeader> {
        let header_data = self
            .data
            .get(0..size_of::<GvdbHeader>())
            .ok_or(GvdbError::DataOffset)?;
        Ok(transmute_one_pedantic(header_data)?)
    }

    /// Returns the root hash table of the file
    pub fn hash_table(&self) -> GvdbResult<GvdbHashTable> {
        let header = self.get_header()?;
        let root_ptr = header.root().clone();
        Ok(GvdbHashTable::for_bytes(
            self.dereference(&root_ptr, 4)?,
            &self,
        )?)
    }

    /// Dereference a pointer
    pub fn dereference(&self, pointer: &GvdbPointer, alignment: u32) -> GvdbResult<&[u8]> {
        let start: usize = pointer.start() as usize;
        let end: usize = pointer.end() as usize;
        let alignment: usize = alignment as usize;

        if start > end {
            Err(GvdbError::DataOffset)
        } else if start & (alignment - 1) != 0 {
            Err(GvdbError::DataAlignment)
        } else {
            self.data.get(start..end).ok_or(GvdbError::DataOffset)
        }
    }

    /// Interpret a chunk of bytes as a GVDB file
    pub fn from_bytes(bytes: Cow<'a, [u8]>) -> GvdbResult<GvdbFile<'a>> {
        let mut this = Self {
            data: bytes,
            byteswapped: false,
        };

        let header = this.get_header()?;
        this.byteswapped = header.is_byteswap()?;
        Ok(this)
    }

    /// Open a file and interpret the data as GVDB
    pub fn from_file(filename: &Path) -> GvdbResult<Self> {
        let mut file =
            File::open(filename).map_err(|err| GvdbError::IO(err, Some(filename.to_path_buf())))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(|err| GvdbError::IO(err, Some(filename.to_path_buf())))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(|err| GvdbError::IO(err, Some(filename.to_path_buf())))?;
        Self::from_bytes(Cow::Owned(data))
    }

    pub(crate) fn with_empty_header(byteswap: bool) -> Self {
        let header = GvdbHeader::new(byteswap, 0, GvdbPointer::NULL);
        let header_data = transmute_one_to_bytes(&header);

        let mut data: Cow<[u8]> = Cow::Owned(Vec::new());
        data.to_mut().extend_from_slice(header_data);

        Self {
            data,
            byteswapped: false,
        }
    }

    pub(crate) fn set_root(&mut self, root: GvdbPointer) -> GvdbResult<()> {
        let mut header: GvdbHeader = transmute_one(&self.data)?;
        header.set_root(root);
        Ok(())
    }

    /// gvdb_table_item_get_key
    pub(crate) fn get_key(&self, item: &GvdbHashItem) -> GvdbResult<String> {
        let data = self.dereference(&item.key_ptr(), 1)?;
        Ok(String::from_utf8(data.to_vec())?)
    }

    pub(crate) fn get_value_for_item(&self, item: &GvdbHashItem) -> GvdbResult<glib::Variant> {
        let typ = item.typ()?;
        if typ == GvdbHashItemType::Value {
            let data: &[u8] = self.dereference(&item.value_ptr(), 8)?;
            Ok(glib::Variant::from_data_with_type(
                data,
                glib::VariantTy::VARIANT,
            ))
        } else {
            Err(GvdbError::DataError(format!(
                "Unable to parse item for key '{}' as GVariant: Expected type 'v', got type {}",
                self.get_key(&item)?,
                typ
            )))
        }
    }

    pub(crate) fn get_hash_table_for_item(&self, item: &GvdbHashItem) -> GvdbResult<GvdbHashTable> {
        let typ = item.typ()?;
        if typ == GvdbHashItemType::HashTable {
            GvdbHashTable::for_bytes(self.dereference(&item.value_ptr(), 4)?, &self)
        } else {
            Err(GvdbError::DataError(format!(
                "Unable to parse item for key '{}' as hash table: Expected type 'H', got type {}",
                self.get_key(&item)?,
                typ
            )))
        }
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
pub mod test {
    use crate::gvdb::read::file::GvdbFile;
    use crate::gvdb::read::hash::test::byte_compare_gvdb_hash_table;
    use std::io::Read;
    use std::path::PathBuf;
    use std::str::FromStr;

    use crate::gvdb::test::assert_bytes_eq;
    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};

    const TEST_FILE_DIR: &str = "test/data/";
    const TEST_FILE_1: &str = "test1.gvdb";
    const TEST_FILE_2: &str = "test2.gvdb";
    const TEST_FILE_3: &str = "test3.gresource";

    pub fn byte_compare_gvdb_file(a: &GvdbFile, b: &GvdbFile) {
        assert_eq!(a.data.len(), b.data.len());
        assert_eq!(a.get_header().unwrap(), b.get_header().unwrap());

        let a_hash = a.hash_table().unwrap();
        let b_hash = b.hash_table().unwrap();
        byte_compare_gvdb_hash_table(&a_hash, &b_hash);
    }

    fn byte_compare_file(file: &GvdbFile, reference_filename: &str) {
        let path = PathBuf::from_str(&reference_filename).unwrap();
        let mut reference_file = std::fs::File::open(path).unwrap();
        let mut reference_data = Vec::new();
        reference_file.read_to_end(&mut reference_data).unwrap();

        assert_bytes_eq(
            &reference_data,
            &file.data(),
            &format!("Byte comparing with file '{}'", reference_filename),
        );
    }

    pub fn byte_compare_file_1(file: &GvdbFile) {
        let reference_filename = TEST_FILE_DIR.to_string() + TEST_FILE_1;
        byte_compare_file(file, &reference_filename);
    }

    pub fn assert_is_file_1(file: &GvdbFile) {
        let table = file.hash_table().unwrap();
        let names = table.get_names().unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "root_key");

        let value = table.get_value("root_key").unwrap().child_value(0);
        assert!(value.is_container());
        assert_eq!(value.type_().to_string(), "(uus)");

        assert_eq!(value.child_value(0).get::<u32>().unwrap(), 1234);
        assert_eq!(value.child_value(1).get::<u32>().unwrap(), 98765);
        assert_eq!(
            value.child_value(2).get::<String>().unwrap(),
            "TEST_STRING_VALUE"
        );
    }

    pub fn byte_compare_file_2(file: &GvdbFile) {
        let reference_filename = TEST_FILE_DIR.to_string() + TEST_FILE_2;
        byte_compare_file(file, &reference_filename);
    }

    pub fn assert_is_file_2(file: &GvdbFile) {
        let table = file.hash_table().unwrap();
        let names = table.get_names().unwrap();
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "string");
        assert_eq!(names[1], "table");

        let str_value = table.get_value("string").unwrap().child_value(0);
        assert!(str_value.is_type(glib::VariantTy::STRING));
        assert_eq!(str_value.get::<String>().unwrap(), "test string");

        let sub_table = table.get_hash_table("table").unwrap();
        let sub_table_names = sub_table.get_names().unwrap();
        assert_eq!(sub_table_names.len(), 1);
        assert_eq!(sub_table_names[0], "int");

        let int_value = sub_table.get_value("int").unwrap().child_value(0);
        assert_eq!(int_value.get::<u32>().unwrap(), 42);
    }

    pub fn byte_compare_file_3(file: &GvdbFile) {
        let reference_filename = TEST_FILE_DIR.to_string() + TEST_FILE_3;
        let ref_root = GvdbFile::from_file(&PathBuf::from(reference_filename)).unwrap();
        byte_compare_gvdb_file(&ref_root, file);
    }

    pub fn assert_is_file_3(file: &GvdbFile) {
        let table = file.hash_table().unwrap();
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
            "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
            "/gvdb/rs/test/json/",
            "/gvdb/rs/test/json/test.json",
            "/gvdb/rs/test/online-symbolic.svg",
        ];
        assert_eq!(names, reference_names);

        let svg1 = table
            .get_value("/gvdb/rs/test/online-symbolic.svg")
            .unwrap()
            .child_value(0);
        let svg1_size = svg1.child_value(0).get::<u32>().unwrap();
        let svg1_flags = svg1.child_value(1).get::<u32>().unwrap();
        let svg1_content = svg1.child_value(2).data_as_bytes();

        assert_eq!(svg1_size, 1390);
        assert_eq!(svg1_flags, 0);
        assert_eq!(svg1_size as usize, svg1_content.len() - 1);

        // Ensure the last byte is zero because of zero-padding defined in the format
        assert_eq!(svg1_content[svg1_content.len() - 1], 0);
        let svg1_str = std::str::from_utf8(&svg1_content[0..svg1_content.len() - 1]).unwrap();
        assert!(svg1_str.starts_with(
            &(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string()
                + "\n\n"
                + r#"<svg xmlns="http://www.w3.org/2000/svg" height="16px""#)
        ));

        let svg2 = table
            .get_value("/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg")
            .unwrap()
            .child_value(0);
        let svg2_size = svg2.child_value(0).get::<u32>().unwrap();
        let svg2_flags = svg2.child_value(1).get::<u32>().unwrap();
        let svg2_content: &[u8] = &svg2.child_value(2).data_as_bytes();

        assert_eq!(svg2_size, 345);
        assert_eq!(svg2_flags, 1);
        let mut decoder = flate2::read::ZlibDecoder::new(svg2_content);
        let mut svg2_data = Vec::new();
        decoder.read_to_end(&mut svg2_data).unwrap();

        // Ensure the last byte is *not* zero and len is not one bigger than specified because
        // compressed data is not zero-padded
        assert_ne!(svg2_data[svg2_data.len() - 1], 0);
        assert_eq!(svg2_size as usize, svg2_data.len());
        let svg2_str = std::str::from_utf8(&svg2_data).unwrap();

        let mut svg2_reference = String::new();
        std::fs::File::open("test/data/gresource/icons/scalable/actions/send-symbolic.svg")
            .unwrap()
            .read_to_string(&mut svg2_reference)
            .unwrap();
        assert_str_eq!(svg2_str, svg2_reference);

        let json = table
            .get_value("/gvdb/rs/test/json/test.json")
            .unwrap()
            .child_value(0);
        let json_size = json.child_value(0).get::<u32>().unwrap();
        let json_flags = json.child_value(1).get::<u32>().unwrap();
        let json_content = json.child_value(2).data_as_bytes().to_vec();

        // Ensure the last byte is zero because of zero-padding defined in the format
        assert_eq!(json_content[json_content.len() - 1], 0);
        assert_eq!(json_size as usize, json_content.len() - 1);
        let json_str = std::str::from_utf8(&json_content[0..json_content.len() - 1]).unwrap();

        assert_eq!(json_flags, 0);
        assert_str_eq!(
            json_str,
            r#"{"test":"test_string","int":42,"table":{"bool":true}}"#.to_string() + "\n"
        );
    }

    #[test]
    fn test_file_1() {
        let filename = TEST_FILE_DIR.to_string() + TEST_FILE_1;
        let path = PathBuf::from_str(&filename).unwrap();
        let file = GvdbFile::from_file(&path).unwrap();
        assert_is_file_1(&file);
    }

    #[test]
    fn test_file_2() {
        let filename = TEST_FILE_DIR.to_string() + TEST_FILE_2;
        let path = PathBuf::from_str(&filename).unwrap();
        let file = GvdbFile::from_file(&path).unwrap();
        assert_is_file_2(&file);
    }

    #[test]
    fn test_file_3() {
        let filename = TEST_FILE_DIR.to_string() + TEST_FILE_3;
        let path = PathBuf::from_str(&filename).unwrap();
        let file = GvdbFile::from_file(&path).unwrap();
        assert_is_file_3(&file);
    }
}
