use crate::read::error::{GvdbReaderError, GvdbReaderResult};
use crate::read::hash_item::{GvdbHashItem, GvdbHashItemType};
use crate::read::header::GvdbHeader;
use crate::read::pointer::GvdbPointer;
use crate::read::GvdbHashTable;
use safe_transmute::transmute_one_pedantic;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[derive(Debug)]
pub(crate) enum GvdbData {
    Cow(Cow<'static, [u8]>),
    #[cfg(feature = "mmap")]
    Mmap(memmap2::Mmap),
}

impl AsRef<[u8]> for GvdbData {
    fn as_ref(&self) -> &[u8] {
        match self {
            GvdbData::Cow(cow) => cow.as_ref(),
            #[cfg(feature = "mmap")]
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
/// let path = PathBuf::from("test-data/test3.gresource");
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
pub struct GvdbFile {
    pub(crate) data: GvdbData,
    pub(crate) byteswapped: bool,
}

impl GvdbFile {
    /// Get the GVDB file header. Will err with GvdbError::DataOffset if the header doesn't fit
    pub(crate) fn get_header(&self) -> GvdbReaderResult<GvdbHeader> {
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
    /// let path = std::path::PathBuf::from("test-data/test3.gresource");
    /// let file = gvdb::read::GvdbFile::from_file(&path).unwrap();
    /// ```
    pub fn from_file(filename: &Path) -> GvdbReaderResult<Self> {
        let mut file =
            File::open(filename).map_err(GvdbReaderError::from_io_with_filename(filename))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(GvdbReaderError::from_io_with_filename(filename))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(GvdbReaderError::from_io_with_filename(filename))?;
        Self::from_bytes(Cow::Owned(data))
    }

    /// Open a file and `mmap` it into memory.
    ///
    /// # Safety
    ///
    /// This is marked unsafe as the file could be modified on-disk while the mmap is active.
    /// This will cause undefined behavior. You must make sure to employ your own locking and to
    /// reload the file yourself when any modification occurs.
    #[cfg(feature = "mmap")]
    pub unsafe fn from_file_mmap(filename: &Path) -> GvdbReaderResult<Self> {
        let file =
            File::open(filename).map_err(GvdbReaderError::from_io_with_filename(filename))?;
        let mmap =
            memmap2::Mmap::map(&file).map_err(GvdbReaderError::from_io_with_filename(filename))?;

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
    pub(crate) fn get_gvariant_for_item(
        &self,
        item: &GvdbHashItem,
    ) -> GvdbReaderResult<glib::Variant> {
        let data = self.get_bytes_for_item(item)?;
        let variant = glib::Variant::from_data_with_type(data, glib::VariantTy::VARIANT);

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
            let context = zvariant::EncodingContext::<byteorder::LE>::new_gvariant(0);
            Ok(zvariant::from_slice(data, context)?)
        } else {
            let context = zvariant::EncodingContext::<byteorder::BE>::new_gvariant(0);
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
                "Unable to parse item for key '{}' as hash table: Expected type 'H', got type '{}'",
                self.get_key(item)?,
                typ
            )))
        }
    }
}

impl std::fmt::Debug for GvdbFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(hash_table) = self.hash_table() {
            f.debug_struct("GvdbFile")
                .field("header", &self.get_header())
                .field("hash_table", &hash_table)
                .finish()
        } else {
            f.debug_struct("GvdbFile")
                .field("header", &self.get_header())
                .finish_non_exhaustive()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::read::file::GvdbFile;
    use std::borrow::Cow;
    use std::mem::size_of;
    use std::path::PathBuf;

    use crate::read::{GvdbHeader, GvdbPointer, GvdbReaderError};
    use crate::test::*;
    use crate::write::{GvdbFileWriter, GvdbHashTableBuilder};
    use matches::assert_matches;
    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};
    use safe_transmute::transmute_one_to_bytes;

    #[test]
    fn test_file_1() {
        let file = GvdbFile::from_file(&TEST_FILE_1).unwrap();
        assert_is_file_1(&file);
    }

    #[cfg(feature = "mmap")]
    #[test]
    fn test_file_1_mmap() {
        let file = unsafe { GvdbFile::from_file_mmap(&TEST_FILE_1).unwrap() };
        assert_is_file_1(&file);
    }

    #[test]
    fn test_file_2() {
        let file = GvdbFile::from_file(&TEST_FILE_2).unwrap();
        assert_is_file_2(&file);
    }

    #[test]
    fn test_file_3() {
        let file = GvdbFile::from_file(&TEST_FILE_3).unwrap();
        assert_is_file_3(&file);
    }

    #[test]
    fn invalid_header() {
        let header = GvdbHeader::new(false, 0, GvdbPointer::new(0, 0));
        let mut data = transmute_one_to_bytes(&header).to_vec();

        data[0] = 0;
        assert_matches!(
            GvdbFile::from_bytes(Cow::Owned(data)),
            Err(GvdbReaderError::DataError(_))
        );
    }

    #[test]
    fn invalid_version() {
        let header = GvdbHeader::new(false, 1, GvdbPointer::new(0, 0));
        let data = transmute_one_to_bytes(&header).to_vec();

        assert_matches!(
            GvdbFile::from_bytes(Cow::Owned(data)),
            Err(GvdbReaderError::DataError(_))
        );
    }

    #[test]
    fn file_does_not_exist() {
        let res = GvdbFile::from_file(&PathBuf::from("this_file_does_not_exist"));
        assert_matches!(res, Err(GvdbReaderError::Io(_, _)));
        println!("{}", res.unwrap_err());
    }

    #[cfg(feature = "mmap")]
    #[test]
    fn file_error_mmap() {
        unsafe {
            assert_matches!(
                GvdbFile::from_file_mmap(&PathBuf::from("this_file_does_not_exist")),
                Err(GvdbReaderError::Io(_, _))
            );
        }
    }

    fn create_minimal_file() -> GvdbFile {
        let header = GvdbHeader::new(false, 0, GvdbPointer::new(0, 0));
        let data = transmute_one_to_bytes(&header).to_vec();
        assert_bytes_eq(
            &data,
            &[
                71, 86, 97, 114, 105, 97, 110, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            "GVDB header",
        );

        GvdbFile::from_bytes(Cow::Owned(data)).unwrap()
    }

    #[test]
    fn test_minimal_file() {
        let _ = create_minimal_file();
    }

    #[test]
    fn broken_hash_table() {
        let writer = GvdbFileWriter::new();
        let mut table = GvdbHashTableBuilder::new();
        table.insert_string("test", "test").unwrap();
        let mut data = writer.write_to_vec_with_table(table).unwrap();

        // Remove data to see if this will throw an error
        data.remove(data.len() - 24);

        // We change the root pointer end to be shorter. Otherwise we will trigger
        // a data offset error when dereferencing. This is a bit hacky.
        // The root pointer end is always at position sizeof(u32 * 5).
        // As this is little endian, we can just modify the first byte.
        let root_ptr_end = size_of::<u32>() * 5;
        data[root_ptr_end] = data[root_ptr_end] - 25;

        let file = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();
        let err = file.hash_table().unwrap_err();
        assert_matches!(err, GvdbReaderError::DataError(_));
        assert!(format!("{}", err).contains("Not enough bytes to fit hash table"));
    }

    #[test]
    fn broken_hash_table2() {
        let writer = GvdbFileWriter::new();
        let mut table = GvdbHashTableBuilder::new();
        table.insert_string("test", "test").unwrap();
        let mut data = writer.write_to_vec_with_table(table).unwrap();

        // We change the root pointer end to be shorter.
        // The root pointer end is always at position sizeof(u32 * 5).
        // As this is little endian, we can just modify the first byte.
        let root_ptr_end = size_of::<u32>() * 5;
        data[root_ptr_end] = data[root_ptr_end] - 23;

        let file = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();
        let err = file.hash_table().unwrap_err();
        assert_matches!(err, GvdbReaderError::DataError(_));
        assert!(format!("{}", err).contains("Remaining size invalid"));
    }

    #[test]
    fn test_dereference_offset1() {
        // Pointer start > EOF
        let file = create_minimal_file();
        let res = file.dereference(&GvdbPointer::new(40, 42), 2);

        assert_matches!(res, Err(GvdbReaderError::DataOffset));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_dereference_offset2() {
        // Pointer start > end
        let file = create_minimal_file();
        let res = file.dereference(&GvdbPointer::new(10, 0), 2);

        assert_matches!(res, Err(GvdbReaderError::DataOffset));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_dereference_offset3() {
        // Pointer end > EOF
        let file = create_minimal_file();
        let res = file.dereference(&GvdbPointer::new(10, 0), 2);

        assert_matches!(res, Err(GvdbReaderError::DataOffset));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_dereference_alignment() {
        // Pointer end > EOF
        let file = create_minimal_file();
        let res = file.dereference(&GvdbPointer::new(1, 2), 2);

        assert_matches!(res, Err(GvdbReaderError::DataAlignment));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_nested_dict() {
        // test file 2 has a nested dictionary
        let file = GvdbFile::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();

        // A table isn't a value
        let table_res = table.get_value("table");
        assert_matches!(table_res, Err(GvdbReaderError::DataError(_)));
    }

    #[test]
    fn test_nested_dict_fail() {
        let file = GvdbFile::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let res = table.get_hash_table("string");
        assert_matches!(res, Err(GvdbReaderError::DataError(_)));
    }
}
