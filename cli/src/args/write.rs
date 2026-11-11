//! The `write` verb's operations: scaffolding a record and changing one.

use super::help;
use super::sub::NewRecordArgs;
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum WriteCmd {
    #[command(about = help::NEW_ABOUT, long_about = None, after_help = help::with_exit_codes(help::NEW_EXAMPLES))]
    New(NewRecordArgs),
    /// `title` also rewrites the heading, renames the file to the new slug
    /// and repoints every in-bundle reference; it is refused on a record
    /// closed for good. `number` renames the record to a free number
    /// (`NNNN` or `next`), the repair for two records sharing a number.
    #[command(about = help::SET_ABOUT, long_about = None, after_help = help::with_exit_codes(help::SET_EXAMPLES))]
    Set {
        #[arg(help = help::reference_help())]
        reference: String,
        #[arg(help = help::set_key_help())]
        key: String,
        /// The new value; for the `section` key, the section heading.
        value: Option<String>,
        /// Reads the value, or for `section` the text under the heading, from
        /// this file or from standard input for `-`. `section` replaces the
        /// named body section or appends it when absent.
        #[arg(long, value_name = "PATH")]
        from: Option<String>,
    },
    #[command(about = help::SUPERSEDE_ABOUT, long_about = None, after_help = help::with_exit_codes(help::SUPERSEDE_EXAMPLES))]
    Supersede {
        #[arg(help = format!("the record being retired: {}", help::reference_help()))]
        old: String,
        #[arg(help = format!("the record that replaces it: {}", help::reference_help()))]
        new: String,
    },
}
