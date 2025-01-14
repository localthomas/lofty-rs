use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2::{
	tag::{tag_frames, Id3v2TagRef},
	Id3v2TagFlags,
};
#[cfg(feature = "riff_info_list")]
use crate::iff::wav::tag::RiffInfoListRef;
#[allow(unused_imports)]
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "riff_info_list")]
		TagType::RiffInfo => Into::<RiffInfoListRef>::into(tag).write_to(data),
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 => Id3v2TagRef::new(Id3v2TagFlags::default(), tag_frames(tag)).write_to(data),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
