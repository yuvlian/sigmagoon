use memory::{DeltaruneError, DeltaruneView};
use std::rc::Rc;
use std::sync::OnceLock;

pub struct HpCheat {
    view: Rc<DeltaruneView>,
    slot: usize, // 0, 1, or 2
    resolved_address: OnceLock<usize>,
}

impl HpCheat {
    const BASE_OFFSET: usize = 0x006AAE50;
    const OFFSETS: &'static [isize] = &[-0x3F8, -0x28, -0x250, -0x320, -0x350, 0x390, 0x210];
    const HP_STEP: isize = 0x10; // by adding this on cur_hp address, we get the next party member's cur_hp
    const MAX_HP_OFFSET: isize = -0x200; // by adding this on cur_hp address, we get max_hp

    pub fn new(view: &Rc<DeltaruneView>, slot: usize) -> Self {
        Self {
            view: Rc::clone(view),
            slot,
            resolved_address: OnceLock::new(),
        }
    }

    // why not
    fn sanitize(value: f64) -> f64 {
        value.min(u16::MAX as f64).round()
    }

    fn get_base_addr(&self) -> Result<usize, DeltaruneError> {
        if let Some(&addr) = self.resolved_address.get() {
            return Ok(addr);
        }

        let addr = self
            .view
            .resolve_pointer(Self::BASE_OFFSET, Self::OFFSETS)?;

        self.resolved_address.set(addr).unwrap();
        Ok(addr)
    }

    // base address + slot offset
    fn get_cur_addr(&self) -> Result<usize, DeltaruneError> {
        let base = self.get_base_addr()?;
        Ok((base as isize + (self.slot as isize * Self::HP_STEP)) as usize)
    }

    pub fn get_hp(&self) -> Result<f64, DeltaruneError> {
        self.view.read(self.get_cur_addr()?)
    }

    pub fn set_hp(&self, value: f64) -> Result<(), DeltaruneError> {
        self.view.write(self.get_cur_addr()?, Self::sanitize(value))
    }

    pub fn modify_hp(&self, f: impl Fn(f64) -> f64) -> Result<(), DeltaruneError> {
        let current = self.get_hp()?;
        self.set_hp(f(current))
    }

    pub fn get_max_hp(&self) -> Result<f64, DeltaruneError> {
        let addr = (self.get_cur_addr()? as isize + Self::MAX_HP_OFFSET) as usize;
        self.view.read(addr)
    }

    pub fn set_max_hp(&self, value: f64) -> Result<(), DeltaruneError> {
        let addr = (self.get_cur_addr()? as isize + Self::MAX_HP_OFFSET) as usize;
        self.view.write(addr, Self::sanitize(value))
    }

    pub fn modify_max_hp(&self, f: impl Fn(f64) -> f64) -> Result<(), DeltaruneError> {
        let current = self.get_max_hp()?;
        self.set_max_hp(f(current))
    }

    pub fn damage(&self, amount: f64) -> Result<(), DeltaruneError> {
        self.modify_hp(|hp| hp - amount)
    }

    pub fn heal(&self, amount: f64) -> Result<(), DeltaruneError> {
        let max_hp = self.get_max_hp()?;
        self.modify_hp(|hp| (hp + amount).min(max_hp))
    }

    pub fn full_heal(&self) -> Result<(), DeltaruneError> {
        let max_hp = self.get_max_hp()?;
        self.set_hp(max_hp)
    }

    pub fn is_down(&self) -> Result<bool, DeltaruneError> {
        Ok(self.get_hp()? < 0.0)
    }

    pub fn is_alive(&self) -> Result<bool, DeltaruneError> {
        Ok(self.get_hp()? > 0.0)
    }

    pub fn is_full_hp(&self) -> Result<bool, DeltaruneError> {
        Ok(self.get_hp()? >= self.get_max_hp()?)
    }

    pub fn get_hp_percent(&self) -> Result<f64, DeltaruneError> {
        let current = self.get_hp()?;
        let max = self.get_max_hp()?;
        if max <= 0.0 {
            return Ok(0.0);
        }
        Ok((current / max).clamp(f64::NEG_INFINITY, 1.0))
    }

    pub fn set_hp_percent(&self, percentage: f64) -> Result<(), DeltaruneError> {
        let max = self.get_max_hp()?;
        self.set_hp(max * percentage.min(1.0))
    }

    pub fn swoon_but_not_really(&self) -> Result<(), DeltaruneError> {
        self.set_hp(-999.0)
    }
}
