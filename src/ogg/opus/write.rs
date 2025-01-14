use crate::error::{LoftyError, Result};

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use ogg_pager::Page;

pub(crate) fn write_to(
	data: &mut File,
	writer: &mut Vec<u8>,
	ser: u32,
	pages: &mut [Page],
) -> Result<()> {
	let reached_md_end: bool;

	loop {
		let p = Page::read(data, true)?;

		if p.header_type() & 0x01 != 0x01 {
			data.seek(SeekFrom::Start(p.start as u64))?;
			reached_md_end = true;
			break;
		}
	}

	if !reached_md_end {
		return Err(LoftyError::Opus("File ends with comment header"));
	}

	let mut remaining = Vec::new();
	data.read_to_end(&mut remaining)?;

	for mut p in pages.iter_mut() {
		p.serial = ser;
		p.gen_crc();

		writer.write_all(&*p.as_bytes())?;
	}

	writer.write_all(&*remaining)?;

	Ok(())
}
