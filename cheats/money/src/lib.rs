use memory::{DeltaruneError, DeltaruneView};
use std::rc::Rc;
use std::sync::OnceLock;

pub struct MoneyCheat {
    view: Rc<DeltaruneView>,
    resolved_address: OnceLock<usize>,
}

impl MoneyCheat {
    const BASE_OFFSET: usize = 0x006A9CA8;
    const OFFSETS: &'static [isize] = &[0x48, 0x10, 0xA50, 0xD60];

    pub fn new(view: &Rc<DeltaruneView>) -> Self {
        Self {
            view: Rc::clone(view),
            resolved_address: OnceLock::new(),
        }
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

    pub fn get_money(&self) -> Result<f64, DeltaruneError> {
        let addr = self.get_base_addr()?;
        self.view.read(addr)
    }

    pub fn set_money(&self, value: f64) -> Result<(), DeltaruneError> {
        let addr = self.get_base_addr()?;
        self.view.write(addr, value)
    }

    pub fn modify_money(&self, f: impl Fn(f64) -> f64) -> Result<(), DeltaruneError> {
        let current = self.get_money()?;
        self.set_money(f(current))
    }
}
