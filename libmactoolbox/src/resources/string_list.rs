use binread::derive_binread;
use crate::types::PString;
use derive_more::{Deref, DerefMut, Index, IndexMut, IntoIterator};

#[derive_binread]
#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut, IntoIterator)]
#[into_iterator(owned, ref, ref_mut)]
pub struct StringList(
    #[br(temp)]
    u16,

    #[deref]
    #[deref_mut]
    #[index]
    #[index_mut]
    #[into_iterator]
    #[br(count = self_0)]
    Vec<PString>
);

impl StringList {
    #[must_use]
    pub fn into_vec(self) -> Vec<PString> {
        self.0
    }
}
