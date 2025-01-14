pub(super) mod read;
mod write;

use crate::error::Result;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Accessor, Tag, TagType};

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

macro_rules! impl_accessor {
	($($name:ident, $key:literal;)+) => {
		paste::paste! {
			impl Accessor for RiffInfoList {
				$(
					fn $name(&self) -> Option<&str> {
						self.get($key)
					}

					fn [<set_ $name>](&mut self, value: String) {
						self.insert(String::from($key), value)
					}

					fn [<remove_ $name>](&mut self) {
						self.remove($key)
					}
				)+
			}
		}
	}
}

#[derive(Default, Debug, PartialEq, Clone)]
/// A RIFF INFO LIST
///
/// ## Supported file types
///
/// * [`FileType::WAV`](crate::FileType::WAV)
///
/// ## Conversions
///
/// ## From `Tag`
///
/// Two conditions must be met:
///
/// * The [`TagItem`] has a value other than [`ItemValue::Binary`](crate::ItemValue::Binary)
/// * It has a key that is 4 bytes in length and within the ASCII range
pub struct RiffInfoList {
	/// A collection of chunk-value pairs
	pub(crate) items: Vec<(String, String)>,
}

impl_accessor!(
	artist, "IART";
	title,  "INAM";
	album,  "IPRD";
	genre,  "IGNR";
);

impl RiffInfoList {
	/// Get an item by key
	pub fn get(&self, key: &str) -> Option<&str> {
		self.items
			.iter()
			.find(|(k, _)| k == key)
			.map(|(_, v)| v.as_str())
	}

	/// Insert an item
	///
	/// NOTE: This will do nothing if `key` is invalid
	///
	/// This will case-insensitively replace any item with the same key
	pub fn insert(&mut self, key: String, value: String) {
		if valid_key(key.as_str()) {
			self.items
				.iter()
				.position(|(k, _)| k.eq_ignore_ascii_case(key.as_str()))
				.map(|p| self.items.remove(p));
			self.items.push((key, value))
		}
	}

	/// Remove an item by key
	///
	/// This will case-insensitively remove an item with the key
	pub fn remove(&mut self, key: &str) {
		self.items
			.iter()
			.position(|(k, _)| k.eq_ignore_ascii_case(key))
			.map(|p| self.items.remove(p));
	}

	/// Returns the tag's items in (key, value) pairs
	pub fn items(&self) -> &[(String, String)] {
		self.items.as_slice()
	}
}

impl RiffInfoList {
	/// Writes the tag to a path
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * See [`RiffInfoList::write_to`]
	pub fn write_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
		self.write_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<RiffInfoListRef>::into(self).write_to(file)
	}

	/// Dumps the tag to a writer
	///
	/// This will only write the LIST chunk, it will not create a usable
	/// file.
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	pub fn dump_to<W: Write>(&self, writer: &mut W) -> Result<()> {
		Into::<RiffInfoListRef>::into(self).dump_to(writer)
	}
}

impl From<RiffInfoList> for Tag {
	fn from(input: RiffInfoList) -> Self {
		let mut tag = Tag::new(TagType::RiffInfo);

		for (k, v) in input.items {
			let item_key = ItemKey::from_key(TagType::RiffInfo, &k);

			tag.items.push(TagItem::new(
				item_key,
				ItemValue::Text(v.trim_matches('\0').to_string()),
			));
		}

		tag
	}
}

impl From<Tag> for RiffInfoList {
	fn from(input: Tag) -> Self {
		let mut riff_info = RiffInfoList::default();

		for item in input.items {
			if let ItemValue::Text(val) | ItemValue::Locator(val) = item.item_value {
				let item_key = match item.item_key {
					ItemKey::Unknown(unknown) => {
						if unknown.len() == 4 && unknown.is_ascii() {
							unknown.to_string()
						} else {
							continue;
						}
					},
					k => {
						if let Some(key) = k.map_key(TagType::RiffInfo, false) {
							key.to_string()
						} else {
							continue;
						}
					},
				};

				riff_info.items.push((item_key, val))
			}
		}

		riff_info
	}
}

pub(crate) struct RiffInfoListRef<'a> {
	items: Box<dyn Iterator<Item = (&'a str, &'a String)> + 'a>,
}

impl<'a> Into<RiffInfoListRef<'a>> for &'a RiffInfoList {
	fn into(self) -> RiffInfoListRef<'a> {
		RiffInfoListRef {
			items: Box::new(self.items.iter().map(|(k, v)| (k.as_str(), v))),
		}
	}
}

impl<'a> Into<RiffInfoListRef<'a>> for &'a Tag {
	fn into(self) -> RiffInfoListRef<'a> {
		RiffInfoListRef {
			items: Box::new(self.items.iter().filter_map(|i| {
				if let ItemValue::Text(val) | ItemValue::Locator(val) = &i.item_value {
					let item_key = i.key().map_key(TagType::RiffInfo, true).unwrap();

					if item_key.len() == 4 && item_key.is_ascii() {
						Some((item_key, val))
					} else {
						None
					}
				} else {
					None
				}
			})),
		}
	}
}

impl<'a> RiffInfoListRef<'a> {
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		write::write_riff_info(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let mut temp = Vec::new();
		write::create_riff_info(&mut self.items, &mut temp)?;

		writer.write_all(&*temp)?;

		Ok(())
	}
}

fn valid_key(key: &str) -> bool {
	key.len() == 4 && key.is_ascii()
}

#[cfg(test)]
mod tests {
	use crate::iff::RiffInfoList;
	use crate::{Tag, TagType};

	use std::io::Cursor;

	#[test]
	fn parse_riff_info() {
		let mut expected_tag = RiffInfoList::default();

		expected_tag.insert(String::from("IART"), String::from("Bar artist"));
		expected_tag.insert(String::from("ICMT"), String::from("Qux comment"));
		expected_tag.insert(String::from("ICRD"), String::from("1984"));
		expected_tag.insert(String::from("INAM"), String::from("Foo title"));
		expected_tag.insert(String::from("IPRD"), String::from("Baz album"));
		expected_tag.insert(String::from("IPRT"), String::from("1"));

		let tag = crate::tag_utils::test_utils::read_path("tests/tags/assets/test.riff");
		let mut parsed_tag = RiffInfoList::default();

		super::read::parse_riff_info(
			&mut Cursor::new(&tag[..]),
			(tag.len() - 1) as u64,
			&mut parsed_tag,
		)
		.unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn riff_info_re_read() {
		let tag = crate::tag_utils::test_utils::read_path("tests/tags/assets/test.riff");
		let mut parsed_tag = RiffInfoList::default();

		super::read::parse_riff_info(
			&mut Cursor::new(&tag[..]),
			(tag.len() - 1) as u64,
			&mut parsed_tag,
		)
		.unwrap();

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let mut temp_parsed_tag = RiffInfoList::default();

		// Remove the LIST....INFO from the tag
		super::read::parse_riff_info(
			&mut Cursor::new(&writer[12..]),
			(tag.len() - 13) as u64,
			&mut temp_parsed_tag,
		)
		.unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn riff_info_to_tag() {
		let tag_bytes = crate::tag_utils::test_utils::read_path("tests/tags/assets/test.riff");

		let mut reader = std::io::Cursor::new(&tag_bytes[..]);
		let mut riff_info = RiffInfoList::default();

		super::read::parse_riff_info(&mut reader, (tag_bytes.len() - 1) as u64, &mut riff_info)
			.unwrap();

		let tag: Tag = riff_info.into();

		crate::tag_utils::test_utils::verify_tag(&tag, true, false);
	}

	#[test]
	fn tag_to_riff_info() {
		let tag = crate::tag_utils::test_utils::create_tag(TagType::RiffInfo);

		let riff_info: RiffInfoList = tag.into();

		assert_eq!(riff_info.get("INAM"), Some("Foo title"));
		assert_eq!(riff_info.get("IART"), Some("Bar artist"));
		assert_eq!(riff_info.get("IPRD"), Some("Baz album"));
		assert_eq!(riff_info.get("ICMT"), Some("Qux comment"));
		assert_eq!(riff_info.get("IPRT"), Some("1"));
	}
}
