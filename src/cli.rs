use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The index of the monitor to screenshot.
    #[clap(short, long, default_value_t = 0)]
    pub monitor: usize,

    /// The duration of each test pass in ms.
    #[clap(short, long, default_value_t = 5000)]
    pub duration: u64,

    /// The duration of each rest period in ms.
    #[clap(short, long, default_value_t = 1000)]
    pub rest: u64,

    /// Enables verbose output.
    #[clap(short, long)]
    pub verbose: bool,
}
