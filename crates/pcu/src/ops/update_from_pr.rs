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
        log::trace!("******************/n** create entry **/n******************/n");
        let mut pr_title = PrTitle::parse(self.title())?;
        pr_title.pr_id = Some(self.pr_number());
        pr_title.pr_url = Some(Url::from_str(self.pull_request())?);
        pr_title.calculate_section_and_entry();
        log::trace!("pr_title: {pr_title:#?}");

        self.prlog_update = Some(pr_title);

        Ok(())
    }

    fn update_changelog(&mut self) -> Result<Option<(ChangeKind, String)>, Error> {
        log::debug!(
            "Updating changelog: {:?} with entry {:?}",
            self.prlog,
            self.prlog_update
        );

        if self.prlog.is_empty() {
            return Err(Error::NoChangeLogFileFound);
        }

        let opts = self.prlog_parse_options.clone();

        if let Some(update) = &mut self.prlog_update {
            #[allow(clippy::needless_question_mark)]
            return Ok(update.update_changelog(&self.prlog, opts)?);
        }
        Ok(None)
    }
}
