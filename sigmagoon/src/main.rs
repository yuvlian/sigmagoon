mod injector;
mod repl;

use hp::HpCheat;
use memory::{DeltaruneError, DeltaruneView};
use money::MoneyCheat;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("https://github.com/yuvlian/sigmagoon");
    println!("waiting for {}...", DeltaruneView::PROCESS_NAME);
    println!("be sure you only run this once you're ingame and not in chapter/save select!");

    let view = loop {
        match DeltaruneView::new() {
            Ok(v) => break v,

            Err(DeltaruneError::ProcessNotFound(_)) => {
                std::thread::sleep(std::time::Duration::from_secs(5));
            }

            Err(
                DeltaruneError::ChapterWindowNotFound
                | DeltaruneError::ChapterNumberNotFound
                | DeltaruneError::ChapterParseError(_)
                | DeltaruneError::InvalidChapterNumber(_),
            ) => {
                println!("couldnt get chapter number automatically. you can enter it manually.");
                let chapter = loop {
                    match repl::read_line("enter chapter (1-7): ").parse::<usize>() {
                        Ok(ch @ 1..=7) => break ch,
                        _ => {}
                    }
                };
                break DeltaruneView::with_chapter(chapter)?;
            }

            Err(e) => return Err(e.into()),
        }
    };

    println!("{} found!", DeltaruneView::PROCESS_NAME);
    println!("module base: 0x{:X}", view.module_base);
    println!("chapter: {}", view.chapter);

    let view = Rc::new(view);
    let mc = MoneyCheat::new(&view)?;
    let hpcs = [
        HpCheat::new(&view, 0)?,
        HpCheat::new(&view, 1)?,
        HpCheat::new(&view, 2)?,
    ];

    repl::run_repl(mc, &hpcs);

    Ok(())
}
