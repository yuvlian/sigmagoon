use crate::error::{DeltaruneError, Result};

#[derive(Clone, Copy)]
pub struct HpOffset {
    pub base_offset: usize,
    pub ptr_chain: &'static [isize],
    pub hp_step: isize,
    pub max_hp_offset: isize,
}

impl HpOffset {
    const OFFSETS: [Option<Self>; 7] = [
        None,
        None,
        Some(Self::new(
            0x006AAE50,
            &[-0x3F8, -0x28, -0x250, -0x320, -0x350, 0x390, 0x210],
            0x10,
            -0x200,
        )),
        None,
        None,
        None,
        None,
    ];

    pub fn get_for_chapter(chapter: usize) -> Result<Self> {
        let Some(Some(o)) = Self::OFFSETS.get(chapter - 1) else {
            return Err(DeltaruneError::UnimplementedOffset(chapter));
        };
        Ok(*o)
    }

    const fn new(
        base_offset: usize,
        ptr_chain: &'static [isize],
        hp_step: isize,
        max_hp_offset: isize,
    ) -> Self {
        Self {
            base_offset,
            ptr_chain,
            hp_step,
            max_hp_offset,
        }
    }
}

#[derive(Clone, Copy)]
pub struct MoneyOffset {
    pub base_offset: usize,
    pub ptr_chain: &'static [isize],
}

impl MoneyOffset {
    const OFFSETS: [Option<Self>; 7] = [
        None,
        None,
        Some(Self::new(0x006A9CA8, &[0x48, 0x10, 0xA50, 0xD60])),
        None,
        None,
        None,
        None,
    ];

    pub fn get_for_chapter(chapter: usize) -> Result<Self> {
        let Some(Some(o)) = Self::OFFSETS.get(chapter - 1) else {
            return Err(DeltaruneError::UnimplementedOffset(chapter));
        };
        Ok(*o)
    }

    const fn new(base_offset: usize, ptr_chain: &'static [isize]) -> Self {
        Self {
            base_offset,
            ptr_chain,
        }
    }
}
