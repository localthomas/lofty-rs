#[cfg(feature = "aiff_text_chunks")]
use super::tag::AiffTextChunks;
use super::AiffFile;
use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::logic::id3::v2::tag::Id3v2Tag;
use crate::logic::iff::chunk::Chunks;

use std::io::{Read, Seek, SeekFrom};

use byteorder::BigEndian;

pub(in crate::logic::iff) fn verify_aiff<R>(data: &mut R) -> Result<()>
where
	R: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if !(&id[..4] == b"FORM" && (&id[8..] == b"AIFF" || &id[..8] == b"AIFC")) {
		return Err(LoftyError::UnknownFormat);
	}

	Ok(())
}

pub(in crate::logic) fn read_from<R>(data: &mut R) -> Result<AiffFile>
where
	R: Read + Seek,
{
	verify_aiff(data)?;

	let mut comm = None;
	let mut stream_len = 0;

	#[cfg(feature = "aiff_text_chunks")]
	let mut text_chunks = AiffTextChunks::default();
	#[cfg(feature = "id3v2")]
	let mut id3v2_tag: Option<Id3v2Tag> = None;

	let mut chunks = Chunks::<BigEndian>::new();

	while chunks.next(data).is_ok() {
		match &chunks.fourcc {
			#[cfg(feature = "id3v2")]
			b"ID3 " | b"id3 " => id3v2_tag = Some(chunks.id3_chunk(data)?),
			b"COMM" => {
				if comm.is_none() {
					if chunks.size < 18 {
						return Err(LoftyError::Aiff(
							"File has an invalid \"COMM\" chunk size (< 18)",
						));
					}

					comm = Some(chunks.content(data)?);
				}
			}
			b"SSND" => {
				stream_len = chunks.size;
				data.seek(SeekFrom::Current(i64::from(chunks.size)))?;
			}
			#[cfg(feature = "aiff_text_chunks")]
			b"NAME" => {
				let value = String::from_utf8(chunks.content(data)?)?;
				text_chunks.name = Some(value);
			}
			#[cfg(feature = "aiff_text_chunks")]
			b"AUTH" => {
				let value = String::from_utf8(chunks.content(data)?)?;
				text_chunks.author = Some(value);
			}
			#[cfg(feature = "aiff_text_chunks")]
			b"(c) " => {
				let value = String::from_utf8(chunks.content(data)?)?;
				text_chunks.copyright = Some(value);
			}
			_ => {
				data.seek(SeekFrom::Current(i64::from(chunks.size)))?;
			}
		}

		chunks.correct_position(data)?;
	}

	if comm.is_none() {
		return Err(LoftyError::Aiff("File does not contain a \"COMM\" chunk"));
	}

	if stream_len == 0 {
		return Err(LoftyError::Aiff("File does not contain a \"SSND\" chunk"));
	}

	let properties = super::properties::read_properties(&mut &*comm.unwrap(), stream_len)?;

	Ok(AiffFile {
		properties,
		#[cfg(feature = "aiff_text_chunks")]
		text_chunks: match text_chunks {
			AiffTextChunks {
				name: None,
				author: None,
				copyright: None,
			} => None,
			_ => Some(text_chunks),
		},
		#[cfg(feature = "id3v2")]
		id3v2_tag,
	})
}
