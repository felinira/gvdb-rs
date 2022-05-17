use crate::read::error::{GvdbReaderError, GvdbReaderResult};
use crate::read::hash::GvdbHashTable;
use crate::read::hash_item::{GvdbHashItem, GvdbHashItemType};
use crate::read::header::GvdbHeader;
use crate::read::pointer::GvdbPointer;
use memmap2::Mmap;
use safe_transmute::transmute_one_pedantic;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[cfg(feature = "glib")]
use glib::{Variant, VariantTy};
use zvariant::EncodingContext;

#[derive(Debug)]
enum GvdbData {
    Cow(Cow<'static, [u8]>),
    Mmap(Mmap),
}

impl AsRef<[u8]> for GvdbData {
    fn as_ref(&self) -> &[u8] {
        match self {
            GvdbData::Cow(cow) => cow.as_ref(),
            GvdbData::Mmap(mmap) => mmap.as_ref(),
        }
    }
}

/// The root of a GVDB file
///
/// # Examples
///
/// Load a GResource file from disk
///
/// ```
/// use std::path::PathBuf;
/// use serde::Deserialize;
/// use gvdb::read::GvdbFile;
///
/// let path = PathBuf::from("test/data/test3.gresource");
/// let file = GvdbFile::from_file(&path).unwrap();
/// let table = file.hash_table().unwrap();
///
/// #[derive(serde::Deserialize, zvariant::Type)]
/// struct SvgData {
///     size: u32,
///     flags: u32,
///     content: Vec<u8>
/// }
///
/// let value = table
///     .get_value("/gvdb/rs/test/online-symbolic.svg")
///     .unwrap();
/// let svg = value.downcast_ref::<zvariant::Structure>().unwrap().fields();
/// let svg1_size = svg[0].downcast_ref::<u32>().unwrap();
/// let svg1_flags = svg[1].downcast_ref::<u32>().unwrap();
/// let svg1_content = svg[2].clone().downcast::<Vec<u8>>().unwrap();
/// let svg1_str = std::str::from_utf8(&svg1_content[0..svg1_content.len() - 1]).unwrap();
///
/// println!("{}", svg1_str);
/// ```
///
/// Query the root hash table
///
/// ```
/// use gvdb::read::GvdbFile;
///
/// fn query_hash_table(file: GvdbFile) {
///     let table = file.hash_table().unwrap();
///     let names = table.get_names().unwrap();
///     assert_eq!(names.len(), 2);
///     assert_eq!(names[0], "string");
///     assert_eq!(names[1], "table");
///
///     let str_value: String = table.get("string").unwrap();
///     assert_eq!(str_value, "test string");
///
///     let sub_table = table.get_hash_table("table").unwrap();
///     let sub_table_names = sub_table.get_names().unwrap();
///     assert_eq!(sub_table_names.len(), 1);
///     assert_eq!(sub_table_names[0], "int");
///
///     let int_value: u32 = sub_table.get("int").unwrap();
///     assert_eq!(int_value, 42);
/// }
/// ```
#[derive(Debug)]
pub struct GvdbFile {
    data: GvdbData,
    byteswapped: bool,
}

impl GvdbFile {
    /// Get the GVDB file header. Will err with GvdbError::DataOffset if the header doesn't fit
    fn get_header(&self) -> GvdbReaderResult<GvdbHeader> {
        let header_data = self
            .data
            .as_ref()
            .get(0..size_of::<GvdbHeader>())
            .ok_or(GvdbReaderError::DataOffset)?;
        Ok(transmute_one_pedantic(header_data)?)
    }

    /// Returns the root hash table of the file
    pub fn hash_table(&self) -> GvdbReaderResult<GvdbHashTable> {
        let header = self.get_header()?;
        let root_ptr = header.root();
        GvdbHashTable::for_bytes(self.dereference(root_ptr, 4)?, self)
    }

    /// Dereference a pointer
    pub(crate) fn dereference(
        &self,
        pointer: &GvdbPointer,
        alignment: u32,
    ) -> GvdbReaderResult<&[u8]> {
        let start: usize = pointer.start() as usize;
        let end: usize = pointer.end() as usize;
        let alignment: usize = alignment as usize;

        if start > end {
            Err(GvdbReaderError::DataOffset)
        } else if start & (alignment - 1) != 0 {
            Err(GvdbReaderError::DataAlignment)
        } else {
            self.data
                .as_ref()
                .get(start..end)
                .ok_or(GvdbReaderError::DataOffset)
        }
    }

    fn read_header(&mut self) -> GvdbReaderResult<()> {
        let header = self.get_header()?;
        if !header.header_valid() {
            return Err(GvdbReaderError::DataError(
                "Invalid GVDB header. Is this a GVDB file?".to_string(),
            ));
        }

        self.byteswapped = header.is_byteswap()?;

        if header.version() != 0 {
            return Err(GvdbReaderError::DataError(format!(
                "Unknown GVDB file format version: {}",
                header.version()
            )));
        }

        Ok(())
    }

    /// Interpret a slice of bytes as a GVDB file
    pub fn from_bytes(bytes: Cow<'static, [u8]>) -> GvdbReaderResult<GvdbFile> {
        let mut this = Self {
            data: GvdbData::Cow(bytes),
            byteswapped: false,
        };

        this.read_header()?;

        Ok(this)
    }

    /// Open a file and interpret the data as GVDB
    /// ```
    /// let path = std::path::PathBuf::from("test/data/test3.gresource");
    /// let file = gvdb::read::GvdbFile::from_file(&path).unwrap();
    /// ```
    pub fn from_file(filename: &Path) -> GvdbReaderResult<Self> {
        let mut file = File::open(filename)
            .map_err(|err| GvdbReaderError::Io(err, Some(filename.to_path_buf())))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(|err| GvdbReaderError::Io(err, Some(filename.to_path_buf())))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(|err| GvdbReaderError::Io(err, Some(filename.to_path_buf())))?;
        Self::from_bytes(Cow::Owned(data))
    }

    /// Open a file and `mmap` it into memory.
    ///
    /// # Safety
    ///
    /// This is marked unsafe as the file could be modified on-disk while the mmap is active.
    /// This will cause undefined behavior. You must make sure to employ your own locking and to
    /// reload the file yourself when any modification occurs.
    pub unsafe fn from_file_mmap(filename: &Path) -> GvdbReaderResult<Self> {
        let file = File::open(filename)
            .map_err(|err| GvdbReaderError::Io(err, Some(filename.to_path_buf())))?;
        let mmap = Mmap::map(&file)
            .map_err(|err| GvdbReaderError::Io(err, Some(filename.to_path_buf())))?;

        let mut this = Self {
            data: GvdbData::Mmap(mmap),
            byteswapped: false,
        };

        this.read_header()?;

        Ok(this)
    }

    /// gvdb_table_item_get_key
    pub(crate) fn get_key(&self, item: &GvdbHashItem) -> GvdbReaderResult<String> {
        let data = self.dereference(&item.key_ptr(), 1)?;
        Ok(String::from_utf8(data.to_vec())?)
    }

    fn get_bytes_for_item(&self, item: &GvdbHashItem) -> GvdbReaderResult<&[u8]> {
        let typ = item.typ()?;
        if typ == GvdbHashItemType::Value {
            Ok(self.dereference(item.value_ptr(), 8)?)
        } else {
            Err(GvdbReaderError::DataError(format!(
                "Unable to parse item for key '{}' as GVariant: Expected type 'v', got type {}",
                self.get_key(item)?,
                typ
            )))
        }
    }

    #[cfg(feature = "glib")]
    pub(crate) fn get_gvariant_for_item(&self, item: &GvdbHashItem) -> GvdbReaderResult<Variant> {
        let data = self.get_bytes_for_item(item).unwrap();
        let variant = Variant::from_data_with_type(data, VariantTy::VARIANT);

        if self.byteswapped {
            Ok(variant.byteswap())
        } else {
            Ok(variant)
        }
    }

    pub(crate) fn get_value_for_item(
        &self,
        item: &GvdbHashItem,
    ) -> GvdbReaderResult<zvariant::Value> {
        let data = self.get_bytes_for_item(item)?;
        #[cfg(target_endian = "little")]
        let le = true;
        #[cfg(target_endian = "big")]
        let le = false;

        if le && !self.byteswapped || !le && self.byteswapped {
            let context = EncodingContext::<byteorder::LE>::new_gvariant(0);
            Ok(zvariant::from_slice(data, context)?)
        } else {
            let context = EncodingContext::<byteorder::BE>::new_gvariant(0);
            Ok(zvariant::from_slice(data, context)?)
        }
    }

    pub(crate) fn get_hash_table_for_item(
        &self,
        item: &GvdbHashItem,
    ) -> GvdbReaderResult<GvdbHashTable> {
        let typ = item.typ()?;
        if typ == GvdbHashItemType::HashTable {
            GvdbHashTable::for_bytes(self.dereference(item.value_ptr(), 4)?, self)
        } else {
            Err(GvdbReaderError::DataError(format!(
                "Unable to parse item for key '{}' as hash table: Expected type 'H', got type {}",
                self.get_key(item)?,
                typ
            )))
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::read::file::GvdbFile;
    use crate::read::hash::test::byte_compare_gvdb_hash_table;
    use std::io::Read;
    use std::path::PathBuf;
    use std::str::FromStr;

    use crate::test::assert_bytes_eq;
    use matches::assert_matches;
    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};

    const TEST_FILE_DIR: &str = "test/data/";
    const TEST_FILE_1: &str = "test1.gvdb";
    const TEST_FILE_2: &str = "test2.gvdb";
    const TEST_FILE_3: &str = "test3.gresource";

    #[allow(dead_code)]
    pub fn byte_compare_gvdb_file(a: &GvdbFile, b: &GvdbFile) {
        assert_eq!(a.data.as_ref().len(), b.data.as_ref().len());
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
            file.data.as_ref(),
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

        let value = table.get_value("root_key").unwrap();
        assert_matches!(value, zvariant::Value::Structure(_));
        assert_eq!(value.value_signature(), "(uus)");

        let tuple = value.downcast::<zvariant::Structure>().unwrap();
        let fields = tuple.into_fields();

        assert_eq!(*fields[0].downcast_ref::<u32>().unwrap(), 1234);
        assert_eq!(*fields[1].downcast_ref::<u32>().unwrap(), 98765);
        assert_eq!(
            fields[2].downcast_ref::<str>().unwrap(),
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

        let str_value = table.get_value("string").unwrap();
        assert_matches!(str_value, zvariant::Value::Str(_));
        assert_eq!(str_value.downcast::<String>().unwrap(), "test string");

        let sub_table = table.get_hash_table("table").unwrap();
        let sub_table_names = sub_table.get_names().unwrap();
        assert_eq!(sub_table_names.len(), 1);
        assert_eq!(sub_table_names[0], "int");

        let int_value = sub_table.get_value("int").unwrap();
        assert_eq!(int_value.downcast::<u32>().unwrap(), 42);
    }

    #[allow(dead_code)]
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

        #[derive(serde::Deserialize, zvariant::Type, zvariant::OwnedValue)]
        struct SvgData {
            size: u32,
            flags: u32,
            content: Vec<u8>,
        }

        let svg1: SvgData = table.get("/gvdb/rs/test/online-symbolic.svg").unwrap();

        assert_eq!(svg1.size, 1390);
        assert_eq!(svg1.flags, 0);
        assert_eq!(svg1.size as usize, svg1.content.len() - 1);

        // Ensure the last byte is zero because of zero-padding defined in the format
        assert_eq!(svg1.content[svg1.content.len() - 1], 0);
        let svg1_str = std::str::from_utf8(&svg1.content[0..svg1.content.len() - 1]).unwrap();
        assert!(svg1_str.starts_with(
            &(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string()
                + "\n\n"
                + r#"<svg xmlns="http://www.w3.org/2000/svg" height="16px""#)
        ));

        let svg2 = table
            .get_value("/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg")
            .unwrap();
        assert_matches!(svg2, zvariant::Value::Structure(_));
        let svg2_fields = svg2
            .clone()
            .downcast::<zvariant::Structure>()
            .unwrap()
            .into_fields();

        let svg2_size = *svg2_fields[0].downcast_ref::<u32>().unwrap();
        let svg2_flags = *svg2_fields[1].downcast_ref::<u32>().unwrap();
        let svg2_content: Vec<u8> = svg2_fields[2].clone().downcast::<Vec<u8>>().unwrap();

        assert_eq!(svg2_size, 345);
        assert_eq!(svg2_flags, 1);
        let mut decoder = flate2::read::ZlibDecoder::new(&*svg2_content);
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
            .downcast::<zvariant::Structure>()
            .unwrap()
            .into_fields();
        let json_size = *json[0].downcast_ref::<u32>().unwrap();
        let json_flags = *json[1].downcast_ref::<u32>().unwrap();
        let json_content = json[2].clone().downcast::<Vec<u8>>().unwrap();

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
    fn test_file_1_mmap() {
        let filename = TEST_FILE_DIR.to_string() + TEST_FILE_1;
        let path = PathBuf::from_str(&filename).unwrap();
        let file = unsafe { GvdbFile::from_file_mmap(&path).unwrap() };
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
