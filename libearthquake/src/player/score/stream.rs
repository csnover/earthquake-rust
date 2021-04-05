use anyhow::{Context, Result as AResult};
use binrw::{BinRead, BinReaderExt, io::{Cursor, Read, Seek, SeekFrom}};
use core::convert::TryFrom;
use libcommon::SeekExt;
use smart_default::SmartDefault;
use super::{Frame, SpriteBitmask, Version};

#[derive(Clone, Debug, SmartDefault)]
pub struct Stream {
    input: Cursor<Vec<u8>>,
    data_start_pos: u32,
    data_end_pos: u32,
    version: Version,
    last_frame: Frame,
    #[default([ 0; Frame::V5_SIZE as usize ])]
    raw_last_frame: [ u8; Frame::V5_SIZE as usize ],
}

impl Stream {
    pub(super) fn new(mut input: Cursor<Vec<u8>>, data_start_pos: u32, data_end_pos: u32, version: Version) -> Self {
        input.seek(SeekFrom::Start(data_start_pos.into())).unwrap();
        Self {
            input,
            data_start_pos,
            data_end_pos,
            version,
            last_frame: Frame::default(),
            raw_last_frame: [ 0; Frame::V5_SIZE as usize ],
        }
    }

    pub(super) fn next(&mut self, channels_to_keep: SpriteBitmask) -> AResult<Option<Frame>> {
        if self.input.pos()? >= self.data_end_pos.into() {
            return Ok(None);
        }

        let mut bytes_to_read = self.input.read_be::<i16>().context("Can’t read compressed score frame size")?;

        if self.version < Version::V4 {
            bytes_to_read = (bytes_to_read - 2).max(0);
        } else {
            // In D5 this check was >= 1 but obviously it needs to be at least 2
            // bytes to read a chunk size
            ensure_sample!(bytes_to_read > 1, "Invalid compressed score frame size {}", bytes_to_read);
            bytes_to_read -= 2;
        }

        let mut new_data = self.raw_last_frame;

        while bytes_to_read > 0 {
            let (chunk_size, chunk_offset) = if self.version < Version::V4 {
                let chunk_size = i16::from(self.input.read_be::<u8>().context("Can’t read compressed score frame chunk size")?) * 2;
                let chunk_offset = usize::from(self.input.read_be::<u8>().context("Can’t read compressed score frame chunk offset")?) * 2;
                bytes_to_read -= chunk_size + 2;
                (chunk_size, chunk_offset)
            } else {
                let chunk_size = self.input.read_be::<i16>().context("Can’t read compressed score frame chunk size")?;
                if chunk_size < 0 {
                    break;
                }
                ensure_sample!(chunk_size & 1 == 0, "Chunk size {} is not a multiple of two", chunk_size);
                let chunk_offset = usize::try_from(self.input.read_be::<i16>().context("Can’t read compressed score frame chunk offset")?).unwrap();
                bytes_to_read -= chunk_size + 4;
                (chunk_size, chunk_offset)
            };

            self.input.read_exact(&mut new_data[chunk_offset..chunk_offset + usize::try_from(chunk_size).unwrap()]).context("Can’t read frame chunk data")?;
        }

        let cursor = &mut Cursor::new(&new_data);
        let mut new_frame = Frame::read_args(cursor, (self.version, )).context("Can’t read frame")?;

        for channel_index in channels_to_keep.iter() {
            match channel_index {
                SpriteBitmask::PALETTE => {
                    new_frame.palette = self.last_frame.palette;
                },
                SpriteBitmask::SOUND_1 => {
                    new_frame.sound_1 = self.last_frame.sound_1;
                },
                SpriteBitmask::SOUND_2 => {
                    new_frame.sound_2 = self.last_frame.sound_2;
                },
                SpriteBitmask::TEMPO => {
                    new_frame.tempo = self.last_frame.tempo;
                },
                SpriteBitmask::MIN_SPRITE..=SpriteBitmask::MAX_SPRITE => {
                    let sprite_index = channel_index - SpriteBitmask::NUM_NON_SPRITE_CHANNELS;
                    let sprite = &mut new_frame.sprites[sprite_index];
                    let script_id = sprite.script();
                    let flags = sprite.score_color_flags();
                    *sprite = self.last_frame.sprites[sprite_index];
                    *sprite.script_mut() = script_id;
                    // TODO: This flag normally comes from the Movie global,
                    // by way of flag 0x100 in the corresponding VWFI
                    // field 0xC.
                    let todo_movie_legacy_flag = false;
                    if todo_movie_legacy_flag {
                        sprite.set_score_color_flags(flags);
                    }
                },
                _ => unreachable!("Invalid frame copy channel data")
            }
        }

        self.raw_last_frame = new_data;

        Ok(Some(new_frame))
    }

    pub(super) fn reset(&mut self) -> AResult<()> {
        self.raw_last_frame = [ 0; Frame::V5_SIZE as usize ];
        self.input.seek(SeekFrom::Start(self.data_start_pos.into())).context("Can’t reset score stream")?;
        Ok(())
    }
}
