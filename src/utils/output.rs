use colored::Colorize;

pub fn success(msg: &str) {
    println!("{} {}", "✅".green(), msg.green());
}

pub fn error(msg: &str) {
    eprintln!("{} {:#}", "❌".red(), msg.red());
}

pub fn info(msg: &str) {
    println!("{} {}", "🐜".normal(), msg.cyan());
}

pub fn warn(msg: &str) {
    println!("{} {}", "⚠️".normal(), msg.yellow());
}

pub fn step(msg: &str) {
    println!("  {} {}", "→".dimmed(), msg);
}

pub fn section(msg: &str) {
    println!("\n{}", msg.bold().white());
}
