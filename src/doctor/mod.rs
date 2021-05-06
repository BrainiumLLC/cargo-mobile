mod section;

use self::section::Section;
use crate::util::cli::TextWrapper;

#[derive(Debug)]
pub struct Doctor {
    sections: Vec<Section>,
}

impl Doctor {
    pub fn check() -> Self {
        Self {
            sections: vec![
                section::cargo_mobile::check(),
                #[cfg(target_os = "macos")]
                section::apple::check(),
                section::android::check(),
                section::device_list::check(),
            ],
        }
    }

    pub fn print(&self, wrapper: &TextWrapper) {
        for section in &self.sections {
            println!();
            section.print(wrapper);
        }
    }
}
