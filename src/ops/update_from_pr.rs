use std::str::FromStr;

use keep_a_changelog::ChangeKind;
use url::Url;

use crate::{Client, Error, PrTitle};

pub trait UpdateFromPr {
    fn update_changelog(&mut self) -> Result<Option<(ChangeKind, String)>, Error>;
    fn create_entry(&mut self) -> Result<(), Error>;
}

impl UpdateFromPr for Client {
    fn create_entry(&mut self) -> Result<(), Error> {
        let mut pr_title = PrTitle::parse(self.title())?;
        pr_title.pr_id = Some(self.pr_number());
        pr_title.pr_url = Some(Url::from_str(self.pull_request())?);
        pr_title.calculate_section_and_entry();

        self.changelog_update = Some(pr_title);

        Ok(())
    }

    fn update_changelog(&mut self) -> Result<Option<(ChangeKind, String)>, Error> {
        log::debug!(
            "Updating changelog: {:?} with entry {:?}",
            self.changelog,
            self.changelog_update
        );

        if self.changelog.is_empty() {
            return Err(Error::NoChangeLogFileFound);
        }

        if let Some(update) = &mut self.changelog_update {
            #[allow(clippy::needless_question_mark)]
            return Ok(update.update_changelog(&self.changelog)?);
        }
        Ok(None)
    }
}
