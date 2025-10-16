// Automatically generated rust module for 'messages.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DocumentSyncMessage<'a> {
    pub id: Cow<'a, str>,
    pub message: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for DocumentSyncMessage<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.message = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for DocumentSyncMessage<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == "" { 0 } else { 1 + sizeof_len((&self.id).len()) }
        + if self.message == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.message).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != "" { w.write_with_tag(10, |w| w.write_string(&**&self.id))?; }
        if self.message != Cow::Borrowed(b"") { w.write_with_tag(18, |w| w.write_bytes(&**&self.message))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DocumentSyncError<'a> {
    pub id: Cow<'a, str>,
    pub reason: Option<messages::SyncErrorReason<'a>>,
}

impl<'a> MessageRead<'a> for DocumentSyncError<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.reason = Some(r.read_message::<messages::SyncErrorReason>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for DocumentSyncError<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == "" { 0 } else { 1 + sizeof_len((&self.id).len()) }
        + self.reason.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != "" { w.write_with_tag(10, |w| w.write_string(&**&self.id))?; }
        if let Some(ref s) = self.reason { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SyncErrorReason<'a> {
    pub reason: messages::mod_SyncErrorReason::Reason,
    pub details: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for SyncErrorReason<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.reason = r.read_enum(bytes)?,
                Ok(18) => msg.details = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for SyncErrorReason<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.reason == messages::mod_SyncErrorReason::Reason::UNKNOWN { 0 } else { 1 + sizeof_varint(*(&self.reason) as u64) }
        + if self.details == "" { 0 } else { 1 + sizeof_len((&self.details).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.reason != messages::mod_SyncErrorReason::Reason::UNKNOWN { w.write_with_tag(8, |w| w.write_enum(*&self.reason as i32))?; }
        if self.details != "" { w.write_with_tag(18, |w| w.write_string(&**&self.details))?; }
        Ok(())
    }
}

pub mod mod_SyncErrorReason {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Reason {
    UNKNOWN = 0,
    INVALID_MESSAGE = 1,
    DOCUMENT_NOT_FOUND = 2,
    INTERNAL_ERROR = 3,
}

impl Default for Reason {
    fn default() -> Self {
        Reason::UNKNOWN
    }
}

impl From<i32> for Reason {
    fn from(i: i32) -> Self {
        match i {
            0 => Reason::UNKNOWN,
            1 => Reason::INVALID_MESSAGE,
            2 => Reason::DOCUMENT_NOT_FOUND,
            3 => Reason::INTERNAL_ERROR,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Reason {
    fn from(s: &'a str) -> Self {
        match s {
            "UNKNOWN" => Reason::UNKNOWN,
            "INVALID_MESSAGE" => Reason::INVALID_MESSAGE,
            "DOCUMENT_NOT_FOUND" => Reason::DOCUMENT_NOT_FOUND,
            "INTERNAL_ERROR" => Reason::INTERNAL_ERROR,
            _ => Self::default(),
        }
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct AvailableDocuments<'a> {
    pub ids: Vec<Cow<'a, str>>,
}

impl<'a> MessageRead<'a> for AvailableDocuments<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.ids.push(r.read_string(bytes).map(Cow::Borrowed)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for AvailableDocuments<'a> {
    fn get_size(&self) -> usize {
        0
        + self.ids.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.ids { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct RequestAvailableDocuments { }

impl<'a> MessageRead<'a> for RequestAvailableDocuments {
    fn from_reader(r: &mut BytesReader, _: &[u8]) -> Result<Self> {
        r.read_to_end();
        Ok(Self::default())
    }
}

impl MessageWrite for RequestAvailableDocuments { }

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct RequestDocument<'a> {
    pub id: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for RequestDocument<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for RequestDocument<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == "" { 0 } else { 1 + sizeof_len((&self.id).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != "" { w.write_with_tag(10, |w| w.write_string(&**&self.id))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Document<'a> {
    pub id: Cow<'a, str>,
    pub document: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for Document<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.document = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Document<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == "" { 0 } else { 1 + sizeof_len((&self.id).len()) }
        + if self.document == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.document).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != "" { w.write_with_tag(10, |w| w.write_string(&**&self.id))?; }
        if self.document != Cow::Borrowed(b"") { w.write_with_tag(18, |w| w.write_bytes(&**&self.document))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Message<'a> {
    pub msg: messages::mod_Message::OneOfmsg<'a>,
}

impl<'a> MessageRead<'a> for Message<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.msg = messages::mod_Message::OneOfmsg::sync_message(r.read_message::<messages::DocumentSyncMessage>(bytes)?),
                Ok(18) => msg.msg = messages::mod_Message::OneOfmsg::sync_error(r.read_message::<messages::DocumentSyncError>(bytes)?),
                Ok(26) => msg.msg = messages::mod_Message::OneOfmsg::available_documents(r.read_message::<messages::AvailableDocuments>(bytes)?),
                Ok(34) => msg.msg = messages::mod_Message::OneOfmsg::request_available_documents(r.read_message::<messages::RequestAvailableDocuments>(bytes)?),
                Ok(42) => msg.msg = messages::mod_Message::OneOfmsg::request_document(r.read_message::<messages::RequestDocument>(bytes)?),
                Ok(50) => msg.msg = messages::mod_Message::OneOfmsg::document(r.read_message::<messages::Document>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Message<'a> {
    fn get_size(&self) -> usize {
        0
        + match self.msg {
            messages::mod_Message::OneOfmsg::sync_message(ref m) => 1 + sizeof_len((m).get_size()),
            messages::mod_Message::OneOfmsg::sync_error(ref m) => 1 + sizeof_len((m).get_size()),
            messages::mod_Message::OneOfmsg::available_documents(ref m) => 1 + sizeof_len((m).get_size()),
            messages::mod_Message::OneOfmsg::request_available_documents(ref m) => 1 + sizeof_len((m).get_size()),
            messages::mod_Message::OneOfmsg::request_document(ref m) => 1 + sizeof_len((m).get_size()),
            messages::mod_Message::OneOfmsg::document(ref m) => 1 + sizeof_len((m).get_size()),
            messages::mod_Message::OneOfmsg::None => 0,
    }    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        match self.msg {            messages::mod_Message::OneOfmsg::sync_message(ref m) => { w.write_with_tag(10, |w| w.write_message(m))? },
            messages::mod_Message::OneOfmsg::sync_error(ref m) => { w.write_with_tag(18, |w| w.write_message(m))? },
            messages::mod_Message::OneOfmsg::available_documents(ref m) => { w.write_with_tag(26, |w| w.write_message(m))? },
            messages::mod_Message::OneOfmsg::request_available_documents(ref m) => { w.write_with_tag(34, |w| w.write_message(m))? },
            messages::mod_Message::OneOfmsg::request_document(ref m) => { w.write_with_tag(42, |w| w.write_message(m))? },
            messages::mod_Message::OneOfmsg::document(ref m) => { w.write_with_tag(50, |w| w.write_message(m))? },
            messages::mod_Message::OneOfmsg::None => {},
    }        Ok(())
    }
}

pub mod mod_Message {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOfmsg<'a> {
    sync_message(messages::DocumentSyncMessage<'a>),
    sync_error(messages::DocumentSyncError<'a>),
    available_documents(messages::AvailableDocuments<'a>),
    request_available_documents(messages::RequestAvailableDocuments),
    request_document(messages::RequestDocument<'a>),
    document(messages::Document<'a>),
    None,
}

impl<'a> Default for OneOfmsg<'a> {
    fn default() -> Self {
        OneOfmsg::None
    }
}

}

