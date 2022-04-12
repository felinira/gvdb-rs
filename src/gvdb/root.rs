use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::hash::GvdbHashTable;
use crate::gvdb::hash_item::GvdbHashItem;
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
use safe_transmute::{transmute_one, transmute_one_pedantic, transmute_one_to_bytes};
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[derive(Debug)]
pub struct GvdbRoot<'a> {
    data: Cow<'a, [u8]>,
    byteswapped: bool,
    trusted: bool,
}

impl<'a> GvdbRoot<'a> {
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
            root_ptr.dereference(&self.data, 4)?,
            &self,
        )?)
    }

    /// Interpret a chunk of bytes as a GVDB file
    pub fn from_bytes(bytes: Cow<'a, [u8]>, trusted: bool) -> GvdbResult<GvdbRoot<'a>> {
        let mut this = Self {
            data: bytes,
            byteswapped: false,
            trusted,
        };

        let header = this.get_header()?;
        this.byteswapped = header.is_byteswap()?;
        Ok(this)
    }

    /// Open a file and interpret the data as GVDB
    pub fn from_file(filename: &Path) -> GvdbResult<Self> {
        let mut file = File::open(filename)?;
        let mut data = Vec::with_capacity(file.metadata()?.len() as usize);
        file.read_to_end(&mut data)?;
        Self::from_bytes(Cow::Owned(data), false)
    }

    pub(crate) fn with_empty_header(byteswap: bool) -> Self {
        let header = GvdbHeader::new(byteswap, 0, GvdbPointer::NULL);
        let header_data = transmute_one_to_bytes(&header);

        let mut data: Cow<[u8]> = Cow::Owned(Vec::new());
        data.to_mut().extend_from_slice(header_data);

        Self {
            data,
            byteswapped: false,
            trusted: true,
        }
    }

    pub(crate) fn set_root(&mut self, root: GvdbPointer) -> GvdbResult<()> {
        let mut header: GvdbHeader = transmute_one(&self.data)?;
        header.set_root(root);
        Ok(())
    }

    /// gvdb_table_item_get_key
    pub(crate) fn get_key(&self, item: &GvdbHashItem) -> GvdbResult<String> {
        let start = item.key_start() as usize;
        let size = item.key_size() as usize;
        let end = start + size;

        let data = self.data.get(start..end).ok_or(GvdbError::DataOffset)?;
        Ok(String::from_utf8(data.to_vec())?)
    }

    pub(crate) fn get_value_for_item(&self, item: GvdbHashItem) -> GvdbResult<glib::Variant> {
        if item.typ() as char == 'v' {
            let data: &[u8] = item.value_ptr().dereference(&self.data, 8)?;
            Ok(glib::Variant::from_data_with_type(
                data,
                glib::VariantTy::VARIANT,
            ))
        } else {
            Err(GvdbError::DataError(format!(
                "Unable to parse item for key '{}' as GVariant: Expected type 'v', got type {}",
                self.get_key(&item)?,
                item.typ()
            )))
        }
    }

    pub(crate) fn get_hash_table_for_item(&self, item: GvdbHashItem) -> GvdbResult<GvdbHashTable> {
        if item.typ() as char == 'H' {
            GvdbHashTable::for_bytes(item.value_ptr().dereference(&self.data, 4)?, &self)
        } else {
            Err(GvdbError::DataError(format!(
                "Unable to parse item for key '{}' as hash table: Expected type 'H', got type {}",
                self.get_key(&item)?,
                item.typ()
            )))
        }
    }
}
