#![allow(dead_code)]

use std::borrow::Borrow;
use std::io::prelude::*;
use std::sync::Arc;
use std::io;

use super::boundary::BoundaryReader;
use super::field::PrivReadEntry;
pub use super::field::{FieldHeaders, MultipartField, MultipartData, ReadEntry, ReadEntryResult};
pub use super::save::{Entries, SaveResult, SavedField};

// /// Replacement typedef for `Arc<str>` for older Rust releases.
// #[cfg(feature = "no_arc_str")]
pub type ArcStr = Arc<String>;

// /// Typedef for `Arc<str>`.
// ///
// /// Construction of `Arc<str>` was only stabilized in Rust 1.21, so to continue to support
// /// older versions, an alternate typedef of `Arc<String>` is available under the `no_arc_str` feature.
// #[cfg(not(feature = "no_arc_str"))]
//pub type ArcStr = Arc<str>;

/// The server-side implementation of `multipart/form-data` requests.
///
/// Implements `Borrow<R>` to allow access to the request body, if desired.
#[derive(Debug)]
pub struct Multipart<R> {
    reader: BoundaryReader<R>,
}

impl<R: Read> Multipart<R> {
    /// Construct a new `Multipart` with the given body reader and boundary.
    ///
    /// ## Note: `boundary`
    /// This will prepend the requisite `--` to the boundary string as documented in
    /// [IETF RFC 1341, Section 7.2.1: "Multipart: the common syntax"][rfc1341-7.2.1].
    /// Simply pass the value of the `boundary` key from the `Content-Type` header in the
    /// request (or use `Multipart::from_request()`, if supported).
    ///
    /// [rfc1341-7.2.1]: https://tools.ietf.org/html/rfc1341#page-30
    pub fn with_body<Bnd: Into<String>>(body: R, boundary: Bnd) -> Self {
        let boundary = boundary.into();

        Multipart { 
            reader: BoundaryReader::from_reader(body, boundary),
        }
    }

    /// Read the next entry from this multipart request, returning a struct with the field's name and
    /// data. See `MultipartField` for more info.
    ///
    /// ## Warning: Risk of Data Loss
    /// If the previously returned entry had contents of type `MultipartField::File`,
    /// calling this again will discard any unread contents of that entry.
    pub fn read_entry(&mut self) -> io::Result<Option<MultipartField<&mut Self>>> {
        self.read_entry_mut().into_result()
    }

    /// Read the next entry from this multipart request, returning a struct with the field's name and
    /// data. See `MultipartField` for more info.
    pub fn into_entry(self) -> ReadEntryResult<Self> {
        self.read_entry()
    }

    /// Call `f` for each entry in the multipart request.
    /// 
    /// This is a substitute for Rust not supporting streaming iterators (where the return value
    /// from `next()` borrows the iterator for a bound lifetime).
    ///
    /// Returns `Ok(())` when all fields have been read, or the first error.
    pub fn foreach_entry<F>(&mut self, mut foreach: F) -> io::Result<()> where F: FnMut(MultipartField<&mut Self>) {
        loop {
            match self.read_entry() {
                Ok(Some(field)) => foreach(field),
                Ok(None) => return Ok(()),
                Err(err) => return Err(err),
            }
        }
    }
}

impl<R> Borrow<R> for Multipart<R> {
    fn borrow(&self) -> &R {
        self.reader.borrow()
    }
}

impl<R: Read> PrivReadEntry for Multipart<R> {
    type Source = BoundaryReader<R>;

    fn source(&mut self) -> &mut BoundaryReader<R> {
        &mut self.reader
    }

    /// Consume the next boundary.
    /// Returns `true` if the last boundary was read, `false` otherwise.
    fn consume_boundary(&mut self) -> io::Result<bool> {
        self.reader.consume_boundary()
    }
}
