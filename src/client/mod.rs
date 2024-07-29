use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs,
    io::Write,
    path::{self, Path},
    process::{Command, Stdio},
    str::FromStr,
    sync::Arc,
};

mod pull_request;

use self::pull_request::PullRequest;

use chrono::{Datelike, NaiveDate, Utc};
use config::Config;
use git2::{BranchType, Cred, Direction, RemoteCallbacks, Repository};
use keep_a_changelog::{
    changelog::ChangelogBuilder, ChangeKind, Changelog, ChangelogParseOptions, Release, Version,
};
use octocrab::Octocrab;
use url::Url;

use crate::PrTitle;
use crate::{release_notes_provider::ReleaseNotesProvider, Error};

const GIT_USER_SIGNATURE: &str = "user.signingkey";

pub struct Client {
    #[allow(dead_code)]
    settings: Config,
    git_repo: Repository,
    owner: String,
    repo: String,
    branch: Option<String>,
    pull_request: Option<PullRequest>,
    changelog: OsString,
    changelog_update: Option<PrTitle>,
    unreleased: Option<String>,
}

impl Client {
    pub async fn new_with(settings: Config) -> Result<Self, Error> {
        log::trace!(
            "new_with settings: {:#?}",
            settings
                .clone()
                .try_deserialize::<HashMap<String, String>>()?,
        );

        let cmd = settings
            .get::<String>("command")
            .map_err(|_| Error::CommandNotSet)?;
        log::trace!("cmd: {:?}", cmd);

        // Use the username config settings to direct to the appropriate CI environment variable to find the owner
        log::trace!("owner: {:?}", settings.get::<String>("username"));
        let pcu_owner: String = settings
            .get("username")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let owner = env::var(pcu_owner).map_err(|_| Error::EnvVarBranchNotFound)?;

        // Use the reponame config settings to direct to the appropriate CI environment variable to find the repo
        log::trace!("repo: {:?}", settings.get::<String>("reponame"));
        let pcu_owner: String = settings
            .get("reponame")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let repo = env::var(pcu_owner).map_err(|_| Error::EnvVarBranchNotFound)?;

        let (branch, pull_request) = if &cmd == "pull-request" {
            // Use the branch config settings to direct to the appropriate CI environment variable to find the branch data
            log::trace!("branch: {:?}", settings.get::<String>("branch"));
            let pcu_branch: String = settings
                .get("branch")
                .map_err(|_| Error::EnvVarBranchNotSet)?;
            let branch = env::var(pcu_branch).map_err(|_| Error::EnvVarBranchNotFound)?;
            let branch = if branch.is_empty() {
                None
            } else {
                Some(branch)
            };

            let pull_request = PullRequest::new_pull_request_opt(&settings).await?;
            (branch, pull_request)
        } else {
            let branch = None;
            let pull_request = None;
            (branch, pull_request)
        };

        // Use the log config setting to set the default change log file name
        log::trace!("log: {:?}", settings.get::<String>("log"));
        let default_change_log: String = settings
            .get("log")
            .map_err(|_| Error::DefaultChangeLogNotSet)?;

        // Get the name of the changelog file
        let mut changelog = OsString::from(default_change_log);
        if let Ok(files) = std::fs::read_dir(".") {
            for file in files.into_iter().flatten() {
                log::trace!("File: {:?}", file.path());

                if file
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("change")
                    && file.file_type().unwrap().is_file()
                {
                    changelog = file.file_name();
                    break;
                }
            }
        };

        let git_repo = git2::Repository::open(".")?;

        Ok(Self {
            settings,
            git_repo,
            branch,
            owner,
            repo,
            pull_request,
            changelog,
            changelog_update: None,
            unreleased: None,
        })
    }

    pub fn branch(&self) -> &str {
        if let Some(branch) = self.branch.as_ref() {
            branch
        } else {
            ""
        }
    }

    pub fn pull_request(&self) -> &str {
        if let Some(pr) = self.pull_request.as_ref() {
            &pr.pull_request
        } else {
            ""
        }
    }

    pub fn title(&self) -> &str {
        if let Some(pr) = self.pull_request.as_ref() {
            &pr.title
        } else {
            ""
        }
    }

    pub fn pr_number(&self) -> u64 {
        if let Some(pr) = self.pull_request.as_ref() {
            pr.pr_number
        } else {
            0
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    pub fn set_title(&mut self, title: &str) {
        if self.pull_request.is_some() {
            self.pull_request.as_mut().unwrap().title = title.to_string();
        }
    }

    pub fn is_default_branch(&self) -> bool {
        let default_branch = self
            .settings
            .get::<String>("default_branch")
            .unwrap_or("main".to_string());
        self.branch == Some(default_branch)
    }

    pub fn create_entry(&mut self) -> Result<(), Error> {
        let mut pr_title = PrTitle::parse(self.title())?;
        pr_title.pr_id = Some(self.pr_number());
        pr_title.pr_url = Some(Url::from_str(self.pull_request())?);
        pr_title.calculate_section_and_entry();

        self.changelog_update = Some(pr_title);

        Ok(())
    }

    pub fn update_changelog(&mut self) -> Result<Option<(ChangeKind, String)>, Error> {
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

    #[allow(dead_code)]
    pub fn commit_changelog(&self) -> Result<String, Error> {
        let mut index = self.git_repo.index()?;
        index.add_path(Path::new(self.changelog()))?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let head = self.git_repo.head()?;
        let parent = self.git_repo.find_commit(head.target().unwrap())?;
        let sig = self.git_repo.signature()?;
        let msg: String = self.settings.get("commit_message")?;

        let commit_id = self.git_repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &self.git_repo.find_tree(tree_id)?,
            &[&parent],
        )?;

        Ok(commit_id.to_string())
    }

    #[allow(dead_code)]
    pub fn commit_changelog_gpg(&self) -> Result<String, Error> {
        let mut index = self.git_repo.index()?;
        index.add_path(Path::new(self.changelog()))?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let head = self.git_repo.head()?;
        let parent = self.git_repo.find_commit(head.target().unwrap())?;
        let sig = self.git_repo.signature()?;
        let msg: String = self.settings.get("commit_message")?;

        let commit_buffer = self.git_repo.commit_create_buffer(
            &sig,
            &sig,
            &msg,
            &self.git_repo.find_tree(tree_id)?,
            &[&parent],
        )?;
        let commit_str = std::str::from_utf8(&commit_buffer).unwrap();

        let signature = self.git_repo.config()?.get_string(GIT_USER_SIGNATURE)?;

        let short_sign = signature[12..].to_string();
        log::trace!("Signature short: {short_sign}");

        let gpg_args = vec!["--status-fd", "2", "-bsau", signature.as_str()];
        log::trace!("gpg args: {:?}", gpg_args);

        let mut cmd = Command::new("gpg");
        cmd.args(gpg_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let mut stdin = child.stdin.take().ok_or(Error::Stdin)?;
        log::trace!("Secured access to stdin");

        log::trace!("Input for signing:\n-----\n{}\n-----", commit_str);

        stdin.write_all(commit_str.as_bytes())?;
        log::trace!("writing complete");
        drop(stdin); // close stdin to not block indefinitely
        log::trace!("stdin closed");

        let output = child.wait_with_output()?;
        log::trace!("secured output");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::trace!("stderr: {}", stderr);
            return Err(Error::Stdout(stderr.to_string()));
        }

        let stderr = std::str::from_utf8(&output.stderr)?;

        if !stderr.contains("\n[GNUPG:] SIG_CREATED ") {
            return Err(Error::GpgError(
                "failed to sign data, program gpg failed, SIG_CREATED not seen in stderr"
                    .to_string(),
            ));
        }
        log::trace!("Error checking completed without error");

        let commit_signature = std::str::from_utf8(&output.stdout)?;

        log::trace!("secured signed commit:\n{}", commit_signature);

        let commit_id =
            self.git_repo
                .commit_signed(commit_str, commit_signature, Some("gpgsig"))?;

        log::trace!("commit id: {}", commit_id);
        // manually advance to the new commit id
        self.git_repo.head()?.set_target(commit_id, &msg)?;

        log::trace!("head updated");

        Ok(commit_id.to_string())
    }

    pub fn push_changelog(&self) -> Result<(), Error> {
        let mut remote = self.git_repo.find_remote("origin")?;
        log::trace!("Pushing changes to {:?}", remote.name());
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap())
        });
        let mut connection = remote.connect_auth(Direction::Push, Some(callbacks), None)?;
        let remote = connection.remote();

        log::trace!("Branch: {:?} or {}", self.branch, self.branch());

        let branch = match self.branch {
            Some(ref branch) => branch,
            None => "main",
        };

        let branch = self.git_repo.find_branch(branch, BranchType::Local)?;
        log::trace!("Found branch: {}", branch.name()?.unwrap());
        let push_refs = branch.into_reference();
        log::trace!("Push refs: {}", push_refs.name().unwrap());

        remote.push(&[push_refs.name().unwrap()], None)?;

        Ok(())
    }

    /// Update the unreleased section to the changelog to `version`
    pub fn update_unreleased(&mut self, version: &str) -> Result<(), Error> {
        log::debug!(
            "Updating unreleased section: {:?} with version {:?}",
            self.changelog,
            version,
        );

        if self.changelog.is_empty() {
            return Err(Error::NoChangeLogFileFound);
        }

        if !version.is_empty() {
            #[allow(clippy::needless_question_mark)]
            return Ok(self.release_unreleased(version)?);
        }

        Ok(())
    }

    pub fn release_unreleased(&mut self, version: &str) -> Result<(), Error> {
        let Some(log_file) = self.changelog.to_str() else {
            return Err(Error::InvalidPath(self.changelog.to_owned()));
        };

        let repo_url =
        //  match &self.pr_url {
        //     Some(pr_url) => {
        //         let url_string = pr_url.to_string();
        //         let components = url_string.split('/').collect::<Vec<&str>>();
        //         let url = format!("https://github.com/{}/{}", components[3], components[4]);
        //         Some(url)
        //     }
        //     None => 
        None
        // }
        ;

        let mut change_log = if path::Path::new(log_file).exists() {
            let file_contents = fs::read_to_string(path::Path::new(log_file))?;
            log::trace!("file contents:\n---\n{}\n---\n\n", file_contents);
            let options = if repo_url.is_some() {
                Some(ChangelogParseOptions {
                    url: repo_url.clone(),
                    ..Default::default()
                })
            } else {
                None
            };

            Changelog::parse_from_file(log_file, options)
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?
        } else {
            log::trace!("The changelog does not exist! Create a default changelog.");
            let mut changelog = ChangelogBuilder::default()
                .url(repo_url)
                .build()
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            log::debug!("Changelog: {:#?}", changelog);
            let release = Release::builder()
                .build()
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            changelog.add_release(release);
            log::debug!("Changelog: {:#?}", changelog);

            changelog
                .save_to_file(log_file)
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            changelog
        };

        // Get the unreleased section from the Changelog.
        // If there is no unreleased section create it and add it to the changelog
        let unreleased = if let Some(unreleased) = change_log.get_unreleased_mut() {
            unreleased
        } else {
            let release = Release::builder()
                .build()
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            change_log.add_release(release);
            let unreleased = change_log.get_unreleased_mut().unwrap();
            unreleased
        };

        let version = Version::parse(version).map_err(|e| Error::InvalidVersion(e.to_string()))?;
        unreleased.set_version(version);

        let today =
            NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), Utc::now().day());
        if let Some(today) = today {
            unreleased.set_date(today);
        };

        let unreleased_string = unreleased.to_string();
        log::trace!("Release notes:\n\n---\n{}\n---\n\n", unreleased_string);
        let _ = fs::write("release_notes.md", unreleased_string.clone());

        self.unreleased = Some(unreleased_string);

        change_log
            .save_to_file(log_file)
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;

        Ok(())
    }

    pub async fn make_release(&self, version: &str) -> Result<(), Error> {
        log::debug!("Making release {version}");

        log::debug!("Creating octocrab instance {:?}", self.settings);
        log::trace!(
            "Creating octocrab for owner: {} and repo: {}",
            self.owner(),
            self.repo()
        );

        let opts = ChangelogParseOptions::default();
        let changelog = match Changelog::parse_from_file(self.changelog(), Some(opts)) {
            Ok(changelog) => changelog,
            Err(e) => {
                log::error!("Error parsing changelog: {e}");
                return Err(Error::InvalidPath(self.changelog.clone()));
            }
        };

        let release_notes = changelog.release_notes(version)?;
        log::trace!("Release notes: {:#?}", release_notes);

        let octocrab = match self.settings.get::<String>("pat") {
            Ok(pat) => {
                log::debug!("Using personal access token for authentication");
                Arc::new(
                    Octocrab::builder()
                        .base_uri("https://api.github.com")?
                        .personal_token(pat)
                        .build()?,
                )
            }
            // base_uri: https://api.github.com
            // auth: None
            // client: http client with the octocrab user agent.
            Err(_) => {
                log::debug!("Creating un-authenticated instance");
                octocrab::instance()
            }
        };

        let commit = Client::get_commitish_for_tag(self, &octocrab, version).await?;
        log::trace!("Commit: {:#?}", commit);

        let release = octocrab
            .repos(self.owner(), self.repo())
            .releases()
            .create(format!("v{version}").as_str())
            .name(&release_notes.name)
            .body(&release_notes.body)
            .send()
            .await?;

        log::trace!("Release: {:#?}", release);

        Ok(())
    }

    pub async fn get_commitish_for_tag(
        &self,
        octocrab: &Octocrab,
        version: &str,
    ) -> Result<String, Error> {
        for tag in octocrab
            .repos(self.owner(), self.repo())
            .list_tags()
            .send()
            .await?
        {
            if tag.name == format!("v{version}").as_str() {
                return Ok(tag.commit.sha);
            }
        }

        Err(Error::TagNotFound(version.to_string()))
    }
}

impl Client {
    pub fn section(&self) -> Option<&str> {
        if let Some(update) = &self.changelog_update {
            if let Some(section) = &update.section {
                match section {
                    ChangeKind::Added => Some("Added"),
                    ChangeKind::Changed => Some("Changed"),
                    ChangeKind::Deprecated => Some("Deprecated"),
                    ChangeKind::Fixed => Some("Fixed"),
                    ChangeKind::Removed => Some("Removed"),
                    ChangeKind::Security => Some("Security"),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn entry(&self) -> Option<&str> {
        if let Some(update) = &self.changelog_update {
            Some(&update.entry)
        } else {
            None
        }
    }

    pub fn changelog(&self) -> &str {
        if let Some(cl) = &self.changelog.to_str() {
            cl
        } else {
            ""
        }
    }

    pub fn set_changelog(&mut self, changelog: &str) {
        self.changelog = changelog.into();
    }

    pub fn repo_status(&self) -> Result<String, Error> {
        let statuses = self.git_repo.statuses(None)?;

        log::trace!("Repo status length: {:?}", statuses.len());

        let report = print_long(&statuses);
        Ok(report)
    }

    pub fn branch_list(&self) -> Result<String, Error> {
        let branches = self.git_repo.branches(None)?;

        let mut output = String::from("\nList of branches:\n");
        for item in branches {
            let (branch, branch_type) = item?;
            output = format!(
                "{}\n# Branch and type: {:?}\t{:?}",
                output,
                branch.name(),
                branch_type
            );
        }
        output = format!("{}\n", output);

        Ok(output)
    }

    pub fn branch_status(&self) -> Result<String, Error> {
        let branch_remote = self.git_repo.find_branch(
            format!("origin/{}", self.branch()).as_str(),
            git2::BranchType::Remote,
        )?;

        if branch_remote.get().target() == self.git_repo.head()?.target() {
            return Ok(format!(
                "\n\nOn branch {}\nYour branch is up to date with `{}`\n",
                self.branch(),
                branch_remote.name()?.unwrap()
            ));
        }

        let local = self.git_repo.head()?.target().unwrap();
        let remote = branch_remote.get().target().unwrap();

        let (ahead, behind) = self.git_repo.graph_ahead_behind(local, remote)?;

        let output = format!(
            "Your branch is {} commits ahead and {} commits behind\n",
            ahead, behind
        );

        Ok(output)
    }
}

// This function print out an output similar to git's status command in long
// form, including the command-line hints.
fn print_long(statuses: &git2::Statuses) -> String {
    let mut header = false;
    let mut rm_in_workdir = false;
    let mut changes_in_index = false;
    let mut changed_in_workdir = false;

    let mut output = String::new();

    // Print index changes
    for entry in statuses
        .iter()
        .filter(|e| e.status() != git2::Status::CURRENT)
    {
        if entry.status().contains(git2::Status::WT_DELETED) {
            rm_in_workdir = true;
        }
        let istatus = match entry.status() {
            s if s.contains(git2::Status::INDEX_NEW) => "new file: ",
            s if s.contains(git2::Status::INDEX_MODIFIED) => "modified: ",
            s if s.contains(git2::Status::INDEX_DELETED) => "deleted: ",
            s if s.contains(git2::Status::INDEX_RENAMED) => "renamed: ",
            s if s.contains(git2::Status::INDEX_TYPECHANGE) => "typechange:",
            _ => continue,
        };
        if !header {
            output = format!(
                "{}\n\
                # Changes to be committed:
                #   (use \"git reset HEAD <file>...\" to unstage)
                #",
                output
            );
            header = true;
        }

        let old_path = entry.head_to_index().unwrap().old_file().path();
        let new_path = entry.head_to_index().unwrap().new_file().path();
        match (old_path, new_path) {
            (Some(old), Some(new)) if old != new => {
                output = format!(
                    "{}\n#\t{}  {} -> {}",
                    output,
                    istatus,
                    old.display(),
                    new.display()
                );
            }
            (old, new) => {
                output = format!(
                    "{}\n#\t{}  {}",
                    output,
                    istatus,
                    old.or(new).unwrap().display()
                );
            }
        }
    }

    if header {
        changes_in_index = true;
        output = format!("{}\n", output);
    }
    header = false;

    // Print workdir changes to tracked files
    for entry in statuses.iter() {
        // With `Status::OPT_INCLUDE_UNMODIFIED` (not used in this example)
        // `index_to_workdir` may not be `None` even if there are no differences,
        // in which case it will be a `Delta::Unmodified`.
        if entry.status() == git2::Status::CURRENT || entry.index_to_workdir().is_none() {
            continue;
        }

        let istatus = match entry.status() {
            s if s.contains(git2::Status::WT_MODIFIED) => "modified: ",
            s if s.contains(git2::Status::WT_DELETED) => "deleted: ",
            s if s.contains(git2::Status::WT_RENAMED) => "renamed: ",
            s if s.contains(git2::Status::WT_TYPECHANGE) => "typechange:",
            _ => continue,
        };

        if !header {
            output = format!(
                "{}\n# Changes not staged for commit:\n#   (use \"git add{} <file>...\" to update what will be committed)\n#   (use \"git checkout -- <file>...\" to discard changes in working directory)\n#               ",
                output,
                if rm_in_workdir { "/rm" } else { "" }
            );
            header = true;
        }

        let old_path = entry.index_to_workdir().unwrap().old_file().path();
        let new_path = entry.index_to_workdir().unwrap().new_file().path();
        match (old_path, new_path) {
            (Some(old), Some(new)) if old != new => {
                output = format!(
                    "{}\n#\t{}  {} -> {}",
                    output,
                    istatus,
                    old.display(),
                    new.display()
                );
            }
            (old, new) => {
                output = format!(
                    "{}\n#\t{}  {}",
                    output,
                    istatus,
                    old.or(new).unwrap().display()
                );
            }
        }
    }

    if header {
        changed_in_workdir = true;
        output = format!("{}\n#\n", output);
    }
    header = false;

    // Print untracked files
    for entry in statuses
        .iter()
        .filter(|e| e.status() == git2::Status::WT_NEW)
    {
        if !header {
            output = format!(
                "{}# Untracked files\n#   (use \"git add <file>...\" to include in what will be committed)\n#",
                output
            );
            header = true;
        }
        let file = entry.index_to_workdir().unwrap().old_file().path().unwrap();
        output = format!("{}\n#\t{}", output, file.display());
    }
    header = false;

    // Print ignored files
    for entry in statuses
        .iter()
        .filter(|e| e.status() == git2::Status::IGNORED)
    {
        if !header {
            output = format!(
                "{}\n# Ignored files\n#   (use \"git add -f <file>...\" to include in what will be committed)\n#",
                output
            );
            header = true;
        }
        let file = entry.index_to_workdir().unwrap().old_file().path().unwrap();
        output = format!("{}\n#\t{}", output, file.display());
    }

    if !changes_in_index && changed_in_workdir {
        output = format!(
            "{}\n
            no changes added to commit (use \"git add\" and/or \
            \"git commit -a\")",
            output
        );
    }

    output
}
