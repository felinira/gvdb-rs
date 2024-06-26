use crate::read::error::{Error, Result};
use crate::read::header::Header;
use crate::read::pointer::Pointer;
use crate::read::HashTable;
use safe_transmute::transmute_one_pedantic;
use std::borrow::Cow;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[derive(Debug)]
pub(crate) enum Data<'a> {
    Cow(Cow<'a, [u8]>),
    #[cfg(feature = "mmap")]
    Mmap(memmap2::Mmap),
}

impl AsRef<[u8]> for Data<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Data::Cow(cow) => cow.as_ref(),
            #[cfg(feature = "mmap")]
            Data::Mmap(mmap) => mmap.as_ref(),
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
/// use gvdb::read::File;
///
/// let path = PathBuf::from("test-data/test3.gresource");
/// let file = File::from_file(&path).unwrap();
/// let table = file.hash_table().unwrap();
///
/// #[derive(serde::Deserialize, zvariant::Type)]
/// struct SvgData {
///     size: u32,
///     flags: u32,
///     content: Vec<u8>
/// }
///
/// let svg: SvgData = table
///     .get("/gvdb/rs/test/online-symbolic.svg")
///     .unwrap();
/// let svg_str = std::str::from_utf8(&svg.content).unwrap();
///
/// println!("{}", svg_str);
/// ```
///
/// Query the root hash table
///
/// ```
/// use gvdb::read::File;
///
/// fn query_hash_table(file: File) {
///     let table = file.hash_table().unwrap();
///     let names = table.keys().unwrap();
///     assert_eq!(names.len(), 2);
///     assert_eq!(names[0], "string");
///     assert_eq!(names[1], "table");
///
///     let str_value: String = table.get("string").unwrap();
///     assert_eq!(str_value, "test string");
///
///     let sub_table = table.get_hash_table("table").unwrap();
///     let sub_table_names = sub_table.keys().unwrap();
///     assert_eq!(sub_table_names.len(), 1);
///     assert_eq!(sub_table_names[0], "int");
///
///     let int_value: u32 = sub_table.get("int").unwrap();
///     assert_eq!(int_value, 42);
/// }
/// ```
pub struct File<'a> {
    pub(crate) data: Data<'a>,
    pub(crate) byteswapped: bool,
}

impl<'a> File<'a> {
    /// Get the GVDB file header. Returns [`Error::DataOffset`]` if the header doesn't fit
    pub(crate) fn get_header(&self) -> Result<Header> {
        let header_data = self
            .data
            .as_ref()
            .get(0..size_of::<Header>())
            .ok_or(Error::DataOffset)?;
        Ok(transmute_one_pedantic(header_data)?)
    }

    /// Returns the root hash table of the file
    pub fn hash_table(&self) -> Result<HashTable> {
        let header = self.get_header()?;
        let root_ptr = header.root();
        HashTable::for_bytes(*root_ptr, self)
    }

    /// Dereference a pointer
    pub(crate) fn dereference(&self, pointer: &Pointer, alignment: u32) -> Result<&[u8]> {
        let start: usize = pointer.start() as usize;
        let end: usize = pointer.end() as usize;
        let alignment: usize = alignment as usize;

        if start > end {
            Err(Error::DataOffset)
        } else if start & (alignment - 1) != 0 {
            Err(Error::DataAlignment)
        } else {
            self.data.as_ref().get(start..end).ok_or(Error::DataOffset)
        }
    }

    fn read_header(&mut self) -> Result<()> {
        let header = self.get_header()?;
        if !header.header_valid() {
            return Err(Error::Data(
                "Invalid GVDB header. Is this a GVDB file?".to_string(),
            ));
        }

        self.byteswapped = header.is_byteswap()?;

        if header.version() != 0 {
            return Err(Error::Data(format!(
                "Unknown GVDB file format version: {}",
                header.version()
            )));
        }

        Ok(())
    }

    /// Interpret a slice of bytes as a GVDB file
    pub fn from_bytes(bytes: Cow<'a, [u8]>) -> Result<Self> {
        let mut this = Self {
            data: Data::Cow(bytes),
            byteswapped: false,
        };

        this.read_header()?;

        Ok(this)
    }

    /// Open a file and interpret the data as GVDB
    /// ```
    /// let path = std::path::PathBuf::from("test-data/test3.gresource");
    /// let file = gvdb::read::File::from_file(&path).unwrap();
    /// ```
    pub fn from_file(filename: &Path) -> Result<Self> {
        let mut file =
            std::fs::File::open(filename).map_err(Error::from_io_with_filename(filename))?;
        let mut data = Vec::with_capacity(
            file.metadata()
                .map_err(Error::from_io_with_filename(filename))?
                .len() as usize,
        );
        file.read_to_end(&mut data)
            .map_err(Error::from_io_with_filename(filename))?;
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
    pub unsafe fn from_file_mmap(filename: &Path) -> Result<Self> {
        let file = std::fs::File::open(filename).map_err(Error::from_io_with_filename(filename))?;
        let mmap = memmap2::Mmap::map(&file).map_err(Error::from_io_with_filename(filename))?;

        let mut this = Self {
            data: Data::Mmap(mmap),
            byteswapped: false,
        };

        this.read_header()?;

        Ok(this)
    }

    /// Determine the endianess to use for zvariant
    pub(crate) fn zvariant_endianess(&self) -> zvariant::Endian {
        if cfg!(target_endian = "little") && !self.byteswapped
            || cfg!(target_endian = "big") && self.byteswapped
        {
            zvariant::LE
        } else {
            zvariant::BE
        }
    }
}

impl std::fmt::Debug for File<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(hash_table) = self.hash_table() {
            f.debug_struct("File")
                .field("byteswapped", &self.byteswapped)
                .field("header", &self.get_header())
                .field("hash_table", &hash_table)
                .finish()
        } else {
            f.debug_struct("File")
                .field("byteswapped", &self.byteswapped)
                .field("header", &self.get_header())
                .finish_non_exhaustive()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::read::file::File;
    use std::borrow::Cow;
    use std::mem::size_of;
    use std::path::PathBuf;

    use crate::read::{Error, HashItem, Header, Pointer};
    use crate::test::*;
    use crate::write::{FileWriter, HashTableBuilder};
    use matches::assert_matches;
    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};
    use safe_transmute::transmute_one_to_bytes;

    #[test]
    fn test_file_1() {
        let file = File::from_file(&TEST_FILE_1).unwrap();
        assert_is_file_1(&file);
    }

    #[cfg(feature = "mmap")]
    #[test]
    fn test_file_1_mmap() {
        let file = unsafe { File::from_file_mmap(&TEST_FILE_1).unwrap() };
        assert_is_file_1(&file);
    }

    #[test]
    fn test_file_2() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        assert_is_file_2(&file);
    }

    #[test]
    fn test_file_3() {
        let file = File::from_file(&TEST_FILE_3).unwrap();
        assert_is_file_3(&file);
    }

    #[test]
    fn invalid_header() {
        let header = Header::new_be(0, Pointer::new(0, 0));
        let mut data = transmute_one_to_bytes(&header).to_vec();

        data[0] = 0;
        assert_matches!(File::from_bytes(Cow::Owned(data)), Err(Error::Data(_)));
    }

    #[test]
    fn invalid_version() {
        let header = Header::new_le(1, Pointer::new(0, 0));
        let data = transmute_one_to_bytes(&header).to_vec();

        assert_matches!(File::from_bytes(Cow::Owned(data)), Err(Error::Data(_)));
    }

    #[test]
    fn file_does_not_exist() {
        let res = File::from_file(&PathBuf::from("this_file_does_not_exist"));
        assert_matches!(res, Err(Error::Io(_, _)));
        println!("{}", res.unwrap_err());
    }

    #[cfg(feature = "mmap")]
    #[test]
    fn file_error_mmap() {
        unsafe {
            assert_matches!(
                File::from_file_mmap(&PathBuf::from("this_file_does_not_exist")),
                Err(Error::Io(_, _))
            );
        }
    }

    fn create_minimal_file() -> File<'static> {
        let header = Header::new_le(0, Pointer::new(0, 0));
        let data = transmute_one_to_bytes(&header).to_vec();
        assert_bytes_eq(
            &data,
            &[
                71, 86, 97, 114, 105, 97, 110, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            "GVDB header",
        );

        File::from_bytes(Cow::Owned(data)).unwrap()
    }

    #[test]
    fn test_minimal_file() {
        let file = create_minimal_file();
        format!("{file:?}");
    }

    #[test]
    fn broken_hash_table() {
        let writer = FileWriter::new();
        let mut table = HashTableBuilder::new();
        table.insert_string("test", "test").unwrap();
        let mut data = writer.write_to_vec_with_table(table).unwrap();

        // Remove data to see if this will throw an error
        data.remove(data.len() - 24);

        // We change the root pointer end to be shorter. Otherwise we will trigger
        // a data offset error when dereferencing. This is a bit hacky.
        // The root pointer end is always at position sizeof(u32 * 5).
        // As this is little endian, we can just modify the first byte.
        let root_ptr_end = size_of::<u32>() * 5;
        data[root_ptr_end] -= 25;

        let file = File::from_bytes(Cow::Owned(data)).unwrap();
        let err = file.hash_table().unwrap_err();
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("Not enough bytes to fit hash table"));
    }

    #[test]
    fn broken_hash_table2() {
        let writer = FileWriter::new();
        let mut table = HashTableBuilder::new();
        table.insert_string("test", "test").unwrap();
        let mut data = writer.write_to_vec_with_table(table).unwrap();

        // We change the root pointer end to be shorter.
        // The root pointer end is always at position sizeof(u32 * 5).
        // As this is little endian, we can just modify the first byte.
        let root_ptr_end = size_of::<u32>() * 5;
        data[root_ptr_end] -= 23;

        let file = File::from_bytes(Cow::Owned(data)).unwrap();
        let err = file.hash_table().unwrap_err();
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("Remaining size invalid"));
    }

    #[test]
    fn parent_invalid_offset() {
        let writer = FileWriter::new();
        let mut table = HashTableBuilder::new();
        table.insert_string("parent/test", "test").unwrap();
        let mut data = writer.write_to_vec_with_table(table).unwrap();

        let file = File::from_bytes(Cow::Owned(data.clone())).unwrap();

        // We change the parent offset to be bigger than the item size in the hash table.
        // 'test' will always end up being item 2.
        // The parent field is at +4.
        let hash_item_size = size_of::<HashItem>();
        let start = file.hash_table().unwrap().hash_items_offset() + hash_item_size * 2;

        let parent_field = start + 4;
        data[parent_field..parent_field + size_of::<u32>()]
            .copy_from_slice(safe_transmute::transmute_one_to_bytes(&10u32.to_le()));

        println!("{:?}", File::from_bytes(Cow::Owned(data.clone())).unwrap());

        let file = File::from_bytes(Cow::Owned(data)).unwrap();
        let err = file.hash_table().unwrap().keys().unwrap_err();
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("Parent with invalid offset"));
        assert!(format!("{}", err).contains("10"));
    }

    #[test]
    fn parent_loop() {
        let writer = FileWriter::new();
        let mut table = HashTableBuilder::new();
        table.insert_string("parent/test", "test").unwrap();
        let mut data = writer.write_to_vec_with_table(table).unwrap();

        let file = File::from_bytes(Cow::Owned(data.clone())).unwrap();

        // We change the parent offset to be pointing to itself.
        // 'test' will always end up being item 2.
        // The parent field is at +4.
        let hash_item_size = size_of::<HashItem>();
        let start = file.hash_table().unwrap().hash_items_offset() + hash_item_size * 2;

        let parent_field = start + 4;
        data[parent_field..parent_field + size_of::<u32>()]
            .copy_from_slice(safe_transmute::transmute_one_to_bytes(&1u32.to_le()));

        println!("{:?}", File::from_bytes(Cow::Owned(data.clone())).unwrap());

        let file = File::from_bytes(Cow::Owned(data)).unwrap();
        let err = file.hash_table().unwrap().keys().unwrap_err();
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("loop"));
    }

    #[test]
    fn test_dereference_offset1() {
        // Pointer start > EOF
        let file = create_minimal_file();
        let res = file.dereference(&Pointer::new(40, 42), 2);

        assert_matches!(res, Err(Error::DataOffset));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_dereference_offset2() {
        // Pointer start > end
        let file = create_minimal_file();
        let res = file.dereference(&Pointer::new(10, 0), 2);

        assert_matches!(res, Err(Error::DataOffset));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_dereference_offset3() {
        // Pointer end > EOF
        let file = create_minimal_file();
        let res = file.dereference(&Pointer::new(10, 0), 2);

        assert_matches!(res, Err(Error::DataOffset));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_dereference_alignment() {
        // Pointer end > EOF
        let file = create_minimal_file();
        let res = file.dereference(&Pointer::new(1, 2), 2);

        assert_matches!(res, Err(Error::DataAlignment));
        println!("{}", res.unwrap_err());
    }

    #[test]
    fn test_nested_dict() {
        // test file 2 has a nested dictionary
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();

        // A table isn't a value
        let table_res = table.get_value("table");
        assert_matches!(table_res, Err(Error::Data(_)));
    }

    #[test]
    fn test_nested_dict_fail() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let res = table.get_hash_table("string");
        assert_matches!(res, Err(Error::Data(_)));
    }

    #[test]
    fn test_from_file_lifetime() {
        // Ensure the lifetime of the file is not bound by the filename
        let filename = TEST_FILE_2.clone();
        let file = File::from_file(&filename).unwrap();
        drop(filename);

        // Ensure the hash table only borrows the file immutably
        let table = file.hash_table().unwrap();
        let table2 = file.hash_table().unwrap();
        table2.keys().unwrap();
        table.keys().unwrap();
    }
}
