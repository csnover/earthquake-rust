use core::{convert::TryFrom, iter::Rev};
use super::NUM_SPRITES;

// TODO: Eventually use some crate like bit_field or bitarray
#[derive(Clone, Copy, Default)]
pub struct SpriteBitmask([ u8; SpriteBitmask::SIZE ]);

pub struct BitIter<'owner> {
    owner: &'owner SpriteBitmask,
    index: usize,
}

impl <'owner> Iterator for BitIter<'owner> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index != SpriteBitmask::NUM_CHANNELS {
            let index = self.index;
            self.index += 1;
            if self.owner.contains(index) {
                return Some(index);
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(SpriteBitmask::NUM_CHANNELS))
    }
}

impl <'owner> DoubleEndedIterator for BitIter<'owner> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.index != 0 {
            self.index -= 1;
            if self.owner.contains(self.index) {
                return Some(self.index);
            }
        }

        None
    }
}

impl SpriteBitmask {
    pub(super) const NUM_NON_SPRITE_CHANNELS: usize = 6;
    pub(super) const MIN_SPRITE: usize = Self::NUM_NON_SPRITE_CHANNELS;
    pub(super) const MAX_SPRITE: usize = NUM_SPRITES + Self::NUM_NON_SPRITE_CHANNELS - 1;
    const NUM_CHANNELS: usize = NUM_SPRITES + Self::NUM_NON_SPRITE_CHANNELS;
    const SIZE: usize = (Self::NUM_CHANNELS + 7) / 8;

    pub(super) const SCRIPT: usize     = 0;
    pub(super) const TEMPO: usize      = 1;
    pub(super) const TRANSITION: usize = 2;
    pub(super) const SOUND_2: usize    = 3;
    pub(super) const SOUND_1: usize    = 4;
    pub(super) const PALETTE: usize    = 5;

    #[must_use]
    pub fn all() -> Self {
        let mut bits = [ 0xFF; Self::SIZE ];
        bits[Self::SIZE - 1] &= u8::try_from((1_u16 << (Self::NUM_CHANNELS % 8)) - 1).unwrap();
        SpriteBitmask(bits)
    }

    #[must_use]
    pub fn bits(&self) -> [ u8; Self::SIZE ] {
        self.0
    }

    #[must_use]
    pub fn contains(&self, bit: usize) -> bool {
        assert!(bit < Self::NUM_CHANNELS);
        self.0[bit / 8] & (1 << (bit % 8)) != 0
    }

    #[must_use]
    pub fn empty() -> Self {
        SpriteBitmask::default()
    }

    #[must_use]
    pub fn iter(&self) -> BitIter<'_> {
        BitIter { owner: self, index: 0 }
    }

    pub fn iter_back(&self) -> Rev<BitIter<'_>> {
        BitIter { owner: self, index: Self::NUM_CHANNELS }.rev()
    }

    #[must_use]
    pub fn iter_sprites(&self) -> BitIter<'_> {
        BitIter { owner: self, index: Self::MIN_SPRITE }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0 == [ 0; Self::SIZE ]
    }

    pub fn remove(&mut self, bit: usize) -> &mut Self {
        assert!(bit < Self::NUM_CHANNELS);
        self.0[bit / 8] &= !(1 << (bit % 8));
        self
    }

    pub fn set(&mut self, bit: usize) -> &mut Self {
        assert!(bit < Self::NUM_CHANNELS);
        self.0[bit / 8] |= 1 << (bit % 8);
        self
    }
}

impl std::ops::BitAnd for SpriteBitmask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let mut bits = self;
        std::ops::BitAndAssign::bitand_assign(&mut bits, rhs);
        bits
    }
}

impl std::ops::BitAndAssign for SpriteBitmask {
    fn bitand_assign(&mut self, rhs: Self) {
        for i in 0..Self::SIZE {
            self.0[i] &= rhs.0[i];
        }
    }
}

impl std::ops::BitOr for SpriteBitmask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut bits = self;
        std::ops::BitOrAssign::bitor_assign(&mut bits, rhs);
        bits
    }
}

impl std::ops::BitOrAssign for SpriteBitmask {
    fn bitor_assign(&mut self, rhs: Self) {
        for i in 0..Self::SIZE - 1 {
            self.0[i] |= rhs.0[i];
        }
        self.0[Self::SIZE - 1] |= rhs.0[Self::SIZE - 1] & u8::try_from((1_u16 << (Self::NUM_CHANNELS % 8)) - 1).unwrap();
    }
}

impl std::ops::Not for SpriteBitmask {
    type Output = Self;

    fn not(self) -> Self::Output {
        let mut bits = self;
        for i in 0..Self::SIZE {
            bits.0[i] = !bits.0[i];
        }
        bits
    }
}

impl std::ops::Sub for SpriteBitmask {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut bits = self;
        std::ops::SubAssign::sub_assign(&mut bits, rhs);
        bits
    }
}

impl std::ops::SubAssign for SpriteBitmask {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..Self::SIZE {
            self.0[i] &= !rhs.0[i];
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn sprite_bitmask_default() {
        let bitmask = SpriteBitmask::default();
        for i in 0..SpriteBitmask::NUM_CHANNELS {
            assert!(!bitmask.contains(i));
        }
        assert!(bitmask.is_empty());
    }

    #[test]
    fn sprite_bitmask_empty() {
        let bitmask = SpriteBitmask::empty();
        for i in 0..SpriteBitmask::NUM_CHANNELS {
            assert!(!bitmask.contains(i));
        }
    }

    #[test]
    fn sprite_bitmask_all() {
        let bitmask = SpriteBitmask::all();
        for i in 0..SpriteBitmask::NUM_CHANNELS {
            assert!(bitmask.contains(i));
        }
        assert!(!bitmask.is_empty());
    }

    #[test]
    fn sprite_bitmask_remove() {
        let mut bitmask = SpriteBitmask::all();
        bitmask.remove(0);
        assert!(!bitmask.contains(0));
        assert!(bitmask.contains(8));
    }

    #[test]
    #[should_panic]
    fn sprite_bitmask_clear_invalid() {
        let mut bitmask = SpriteBitmask::default();
        bitmask.remove(SpriteBitmask::NUM_CHANNELS);
    }

    #[test]
    #[should_panic]
    #[allow(unused_must_use)]
    fn sprite_bitmask_contains_invalid() {
        let bitmask = SpriteBitmask::default();
        bitmask.contains(SpriteBitmask::NUM_CHANNELS);
    }

    #[test]
    #[should_panic]
    fn sprite_bitmask_set_invalid() {
        let mut bitmask = SpriteBitmask::default();
        bitmask.set(SpriteBitmask::NUM_CHANNELS);
    }
}

impl std::fmt::Debug for SpriteBitmask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for channel_index in self.iter() {
            match channel_index {
                Self::PALETTE => write!(f, "Pl ")?,
                Self::SCRIPT => write!(f, "Sc ")?,
                Self::SOUND_1 => write!(f, "S1 ")?,
                Self::SOUND_2 => write!(f, "S2 ")?,
                Self::TEMPO => write!(f, "Tm ")?,
                Self::TRANSITION => write!(f, "Tx ")?,
                Self::MIN_SPRITE..=Self::MAX_SPRITE => write!(f, "{:02} ", channel_index - Self::MIN_SPRITE + 1)?,
                _ => write!(f, "X{} ", channel_index)?,
            }
        }
        Ok(())
    }
}
