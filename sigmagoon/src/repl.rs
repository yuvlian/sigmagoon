use crate::injector;
use hp::HpCheat;
use memory::DeltaruneView;
use money::MoneyCheat;
use std::io::{self, Write};
use std::rc::Rc;

fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return String::new();
    }
    input.trim().to_string()
}

pub fn run_repl(view: Rc<DeltaruneView>) {
    println!("\n=== sigmagoon repl ===");
    println!("type 'help' for list of commands or 'exit' to exit, duh.");

    let mc = MoneyCheat::new(&view);
    let hp_cheats = [
        HpCheat::new(&view, 0),
        HpCheat::new(&view, 1),
        HpCheat::new(&view, 2),
    ];

    loop {
        let cmd = read_line("\nsigmagoon> ").to_lowercase();
        if cmd.is_empty() {
            continue;
        }

        match cmd.as_str() {
            "exit" | "quit" | "q" => {
                println!("bye bye!");
                break;
            }
            "help" => {
                println!("available commands:");
                println!("  hp     - Edit character HP/Max HP");
                println!("  money  - Edit/view current dark dollars");
                println!("  speed  - Speed hack / speed multiplier");
                println!("  exit   - Exit the REPL");
            }
            "money" => {
                match mc.get_money() {
                    Ok(curr) => println!("current money: {}", curr),
                    Err(e) => {
                        println!("error reading money: {}", e);
                        continue;
                    }
                }

                println!(
                    "options: [1] Set Money   [2] Add Money   [3] Subtract Money   [4] Cancel"
                );
                let opt = read_line("> ");
                match opt.as_str() {
                    "1" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match mc.set_money(val) {
                                Ok(_) => println!("money set to {}", val),
                                Err(e) => println!("error writing money: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "2" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match mc.modify_money(|m| m + val) {
                                Ok(_) => println!("added {} to money", val),
                                Err(e) => println!("error modifying money: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "3" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match mc.modify_money(|m| m - val) {
                                Ok(_) => println!("subtracted {} from money", val),
                                Err(e) => println!("error modifying money: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    _ => {
                        println!("cancelled.");
                    }
                }
            }
            "hp" => {
                let slot_str = read_line("character slot (1-3)? ");
                let Ok(slot_num) = slot_str.parse::<usize>() else {
                    println!("invalid slot!");
                    continue;
                };

                if slot_num < 1 || slot_num > 3 {
                    println!("slot must be 1, 2, or 3!");
                    continue;
                }

                let slot = slot_num - 1;
                println!(
                    "* note: this assumes character slot {} exists in battle party",
                    slot_num
                );

                let hc = &hp_cheats[slot];

                match (hc.get_hp(), hc.get_max_hp()) {
                    (Ok(curr), Ok(max)) => {
                        println!("current hp: {} / {}", curr, max);
                    }
                    (Err(e1), Err(e2)) => {
                        println!("failed to read hp: {} {}", e1, e2);
                    }
                    (Ok(curr), Err(e2)) => {
                        println!("current hp: {} (error reading max: {})", curr, e2);
                    }
                    (Err(e1), Ok(max)) => {
                        println!("max hp: {} (error reading current: {})", max, e1);
                    }
                }

                println!("options:");
                println!("  [1] Set HP      [2] Set Max HP   [3] Damage       [4] Heal");
                println!("  [5] Full Heal   [6] Fake Swoon   [7] Cancel");
                let opt = read_line("> ");

                match opt.as_str() {
                    "1" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match hc.set_hp(val) {
                                Ok(_) => println!("hp set to {}", val),
                                Err(e) => println!("error writing hp: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "2" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match hc.set_max_hp(val) {
                                Ok(_) => println!("max hp set to {}", val),
                                Err(e) => println!("error writing max hp: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "3" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match hc.damage(val) {
                                Ok(_) => println!("damaged character by {}", val),
                                Err(e) => println!("error writing hp: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "4" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match hc.heal(val) {
                                Ok(_) => println!("healed character by {}", val),
                                Err(e) => println!("error writing hp: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "5" => match hc.full_heal() {
                        Ok(_) => println!("fully healed character"),
                        Err(e) => println!("error writing hp: {}", e),
                    },
                    "6" => match hc.swoon_but_not_really() {
                        Ok(_) => println!("swooned character! (not really)"),
                        Err(e) => println!("error writing hp: {}", e),
                    },
                    _ => {
                        println!("cancelled.");
                    }
                }
            }
            "speed" => {
                println!("options: [1] Set Speed   [2] Reset Speed   [3] Cancel");
                let opt = read_line("> ");
                match opt.as_str() {
                    "1" => {
                        let val_str = read_line("how many? ");
                        if let Ok(val) = val_str.parse::<f64>() {
                            match injector::inject_speed_cheat(val) {
                                Ok(_) => println!("speed environment set to {}", val),
                                Err(e) => println!("injection failed: {}", e),
                            }
                        } else {
                            println!("invalid number! make sure its a float");
                        }
                    }
                    "2" => match injector::inject_speed_cheat(-1.0) {
                        Ok(_) => println!("speed reset to default"),
                        Err(e) => println!("injection/reset failed: {}", e),
                    },
                    _ => {
                        println!("cancelled.");
                    }
                }
            }
            _ => {
                println!(
                    "unknown command: '{}'. type 'help' for a list of commands.",
                    cmd
                );
            }
        }
    }
}
