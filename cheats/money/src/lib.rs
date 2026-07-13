use memory::{DeltaruneView, MoneyOffset, Result};
use std::rc::Rc;

pub struct MoneyCheat {
    view: Rc<DeltaruneView>,
    money_addr: usize,
}

impl MoneyCheat {
    pub fn new(view: &Rc<DeltaruneView>) -> Result<Self> {
        let offset = MoneyOffset::get_for_chapter(view.chapter)?;
        let money_addr = view.resolve_pointer(offset.base_offset, offset.ptr_chain)?;

        Ok(Self {
            view: Rc::clone(view),
            money_addr,
        })
    }

    pub fn get_money(&self) -> Result<f64> {
        self.view.read(self.money_addr)
    }

    pub fn set_money(&self, value: f64) -> Result<()> {
        self.view.write(self.money_addr, value)
    }

    pub fn modify_money(&self, f: impl Fn(f64) -> f64) -> Result<()> {
        let current = self.get_money()?;
        self.set_money(f(current))
    }
}
