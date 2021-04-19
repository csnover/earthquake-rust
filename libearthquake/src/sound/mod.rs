use crate::resources::cast::MemberId;

#[derive(Debug, Default)]
pub struct Manager {
    movie_list_nums: [i16; 8],
    cast_members: [MemberId; 8],
}

impl Manager {
    /// RE: `Sound_Init`
    pub fn new() -> Self {
        <_>::default()
    }
}
