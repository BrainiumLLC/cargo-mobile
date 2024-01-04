use crate::{os::Info, util};
use once_cell_regex::regex;

pub fn check() -> Result<Info, util::RunAndSearchError> {
    util::run_and_search(
        &mut bossy::Command::impure_parse("ver"),
        regex!(r"\[Microsoft Windows (?P<version>.*)\]"),
        |_output, caps| caps.name("version").unwrap().as_str().to_owned(),
    )
    .map(|version| Info {
        name: "Windows".to_owned(),
        version,
    })
}
