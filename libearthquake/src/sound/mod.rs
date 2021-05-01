use crate::resources::cast::MemberId;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Channel {
    volume: i16,
    active: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Manager {
    movie_list_nums: [i16; 8],
    cast_members: [MemberId; 8],
    channels: [Channel; 20]
}

impl Manager {
    /// RE: `Sound_Init`
    pub(crate) fn new() -> Self {
        <_>::default()
    }
}
