// This file is part of Noir.

// Copyright (c) Haderech Pte. Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use alloc::vec::Vec;
use bincode::config;
use serde::Serialize;

pub use bincode::error::{DecodeError, EncodeError};

/// Serializes an object directly into a `Writer` using the default configuration.
///
/// If the serialization would take more bytes than allowed by the size limit, an error
/// is returned and *no bytes* will be written into the `Writer`.
//pub fn serialize_into<W, T: ?Sized>(writer: W, value: &T) -> Result<(), EncodeError>
//where
//    W: std::io::Write,
//    T: serde::Serialize,
//{
pub fn serialize_into<T: ?Sized>(writer: &mut [u8], value: &T) -> Result<(), EncodeError>
where
    T: serde::Serialize,
{
    let writer = bincode::enc::write::SliceWriter::new(writer);
    bincode::serde::encode_into_writer(value, writer, config::legacy())
}
/// Serializes a serializable object into a `Vec` of bytes using the default configuration.
pub fn serialize<T: ?Sized>(value: &T) -> Result<Vec<u8>, EncodeError>
where
    T: Serialize,
{
    bincode::serde::encode_to_vec(value, config::legacy())
}

/// Deserializes a slice of bytes into an instance of `T` using the default configuration.
pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, DecodeError>
where
    T: serde::de::Deserialize<'a>,
{
    bincode::serde::decode_borrowed_from_slice(bytes, config::legacy())
}

/// Returns the size that an object would be if serialized using Bincode with the default
/// configuration.
pub fn serialized_size<T: ?Sized>(value: &T) -> Result<u64, EncodeError>
where
    T: serde::Serialize,
{
    let mut writer = bincode::enc::write::SizeWriter::default();
    bincode::serde::encode_into_writer(value, &mut writer, config::legacy())?;
    Ok(writer.bytes_written as u64)
}
