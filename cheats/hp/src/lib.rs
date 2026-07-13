use memory::{DeltaruneView, HpOffset, Result};
use std::rc::Rc;

pub struct HpCheat {
    view: Rc<DeltaruneView>,
    cur_hp_addr: usize,
    max_hp_addr: usize,
}

impl HpCheat {
    pub fn new(view: &Rc<DeltaruneView>, slot: usize) -> Result<Self> {
        let offset = HpOffset::get_for_chapter(view.chapter)?;
        let base = view.resolve_pointer(offset.base_offset, offset.ptr_chain)?;

        let cur_hp_addr = (base as isize + slot as isize * offset.hp_step) as usize;
        let max_hp_addr = (cur_hp_addr as isize + offset.max_hp_offset) as usize;

        Ok(Self {
            view: Rc::clone(view),
            cur_hp_addr,
            max_hp_addr,
        })
    }

    // why not
    fn sanitize(value: f64) -> f64 {
        value.min(u16::MAX as f64).round()
    }

    pub fn get_hp(&self) -> Result<f64> {
        self.view.read(self.cur_hp_addr)
    }

    pub fn set_hp(&self, value: f64) -> Result<()> {
        self.view.write(self.cur_hp_addr, Self::sanitize(value))
    }

    pub fn modify_hp(&self, f: impl Fn(f64) -> f64) -> Result<()> {
        let current = self.get_hp()?;
        self.set_hp(f(current))
    }

    pub fn get_max_hp(&self) -> Result<f64> {
        self.view.read(self.max_hp_addr)
    }

    pub fn set_max_hp(&self, value: f64) -> Result<()> {
        self.view.write(self.max_hp_addr, Self::sanitize(value))
    }

    pub fn modify_max_hp(&self, f: impl Fn(f64) -> f64) -> Result<()> {
        let current = self.get_max_hp()?;
        self.set_max_hp(f(current))
    }

    pub fn damage(&self, amount: f64) -> Result<()> {
        self.modify_hp(|hp| hp - amount)
    }

    pub fn heal(&self, amount: f64) -> Result<()> {
        let max_hp = self.get_max_hp()?;
        self.modify_hp(|hp| (hp + amount).min(max_hp))
    }

    pub fn full_heal(&self) -> Result<()> {
        let max_hp = self.get_max_hp()?;
        self.set_hp(max_hp)
    }

    pub fn is_down(&self) -> Result<bool> {
        Ok(self.get_hp()? < 0.0)
    }

    pub fn is_alive(&self) -> Result<bool> {
        Ok(self.get_hp()? > 0.0)
    }

    pub fn is_full_hp(&self) -> Result<bool> {
        Ok(self.get_hp()? >= self.get_max_hp()?)
    }

    pub fn get_hp_percent(&self) -> Result<f64> {
        let current = self.get_hp()?;
        let max = self.get_max_hp()?;
        if max <= 0.0 {
            return Ok(0.0);
        }
        Ok((current / max).clamp(f64::NEG_INFINITY, 1.0))
    }

    pub fn set_hp_percent(&self, percentage: f64) -> Result<()> {
        let max = self.get_max_hp()?;
        self.set_hp(max * percentage.min(1.0))
    }

    pub fn swoon_but_not_really(&self) -> Result<()> {
        self.set_hp(-999.0)
    }
}
