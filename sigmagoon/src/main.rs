mod injector;
mod repl;

use memory::{DeltaruneError, DeltaruneView};
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("https://github.com/yuvlian/sigmagoon");
    println!("waiting for {}...", DeltaruneView::PROCESS_NAME);

    let view = loop {
        match DeltaruneView::new() {
            Ok(v) => break Rc::new(v),
            Err(DeltaruneError::ProcessNotFound(_)) => {
                std::thread::sleep(std::time::Duration::from_secs(5))
            }
            Err(e) => return Err(e.into()),
        }
    };

    println!("{} found!", DeltaruneView::PROCESS_NAME);
    println!("module base: 0x{:X}", view.module_base);
    repl::run_repl(view);
    Ok(())
}
