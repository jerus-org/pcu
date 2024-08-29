use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use clap::ValueEnum;
use color_eyre::owo_colors::OwoColorize;
use git2::{
    BranchType, Cred, Direction, Oid, PushOptions, RemoteCallbacks, Signature, StatusOptions,
};
use log::log_enabled;
use tracing::instrument;

use crate::client::graphql::GraphQLGetOpenPRs;
use crate::client::graphql::GraphQLLabelPR;
use crate::Client;
use crate::Error;

const GIT_USER_SIGNATURE: &str = "user.signingkey";
const DEFAULT_COMMIT_MESSAGE: &str = "chore: commit staged files";
const DEFAULT_REBASE_LOGIN: &str = "renovate";

#[derive(ValueEnum, Debug, Default, Clone)]
pub enum Sign {
    #[default]
    Gpg,
    None,
}

pub trait GitOps {
    fn branch_status(&self) -> Result<String, Error>;
    fn branch_list(&self) -> Result<String, Error>;
    fn repo_status(&self) -> Result<String, Error>;
    fn repo_files_not_staged(&self) -> Result<Vec<String>, Error>;
    fn repo_files_staged(&self) -> Result<Vec<String>, Error>;
    fn stage_files(&self, files: Vec<String>) -> Result<(), Error>;
    fn commit_staged(
        &self,
        sign: Sign,
        commit_message: &str,
        tag: Option<&str>,
    ) -> Result<(), Error>;
    fn push_commit(&self, version: Option<&str>, no_push: bool) -> Result<(), Error>;
    #[allow(async_fn_in_trait)]
    async fn rebase_next_pr(&self, login: Option<&str>) -> Result<Option<String>, Error>;
    fn create_tag(&self, tag: &str, commit_id: Oid, sig: &Signature) -> Result<(), Error>;
    #[allow(async_fn_in_trait)]
    async fn get_commitish_for_tag(&self, version: &str) -> Result<String, Error>;
}

impl GitOps for Client {
    fn create_tag(&self, tag: &str, commit_id: Oid, sig: &Signature) -> Result<(), Error> {
        let object = self.git_repo.find_object(commit_id, None)?;
        self.git_repo.tag(tag, &object, sig, tag, true)?;

        let mut revwalk = self.git_repo.revwalk()?;
        let reference = format!("refs/tags/{tag}");
        revwalk.push_ref(&reference)?;
        Ok(())
    }

    async fn get_commitish_for_tag(&self, tag: &str) -> Result<String, Error> {
        log::trace!("Get commitish for tag: {tag}");
        log::trace!(
            "Get tags for owner {:?} and repo: {:?}",
            self.owner(),
            self.repo()
        );
        for t in self
            .github_rest
            .repos
            .list_tags(self.owner(), self.repo())
            .send()
            .await?
        {
            log::trace!("Tag: {}", t.name);
            if t.name == tag {
                return Ok(t.commit.sha);
            }
        }

        Err(Error::TagNotFound(tag.to_string()))
    }

    /// Report the status of the git repo in a human readable format
    fn repo_status(&self) -> Result<String, Error> {
        let statuses = self.git_repo.statuses(None)?;

        log::trace!("Repo status length: {:?}", statuses.len());

        Ok(print_long(&statuses))
    }

    /// Report a list of the files that have not been staged
    fn repo_files_not_staged(&self) -> Result<Vec<String>, Error> {
        let mut options = StatusOptions::new();
        options.show(git2::StatusShow::Workdir);
        options.include_untracked(true);
        let statuses = self.git_repo.statuses(Some(&mut options))?;

        log::trace!("Repo status length: {:?}", statuses.len());

        let files: Vec<String> = statuses
            .iter()
            .map(|s| s.path().unwrap_or_default().to_string())
            .collect();

        Ok(files)
    }

    /// Report a list of the files that have not been staged
    fn repo_files_staged(&self) -> Result<Vec<String>, Error> {
        let mut options = StatusOptions::new();
        options.show(git2::StatusShow::Index);
        options.include_untracked(true);
        let statuses = self.git_repo.statuses(Some(&mut options))?;

        log::trace!("Repo status length: {:?}", statuses.len());

        let files: Vec<String> = statuses
            .iter()
            .map(|s| s.path().unwrap_or_default().to_string())
            .collect();

        Ok(files)
    }

    fn stage_files(&self, files: Vec<String>) -> Result<(), Error> {
        let mut index = self.git_repo.index()?;

        for file in files {
            index.add_path(Path::new(&file))?;
        }

        index.write()?;

        Ok(())
    }

    fn commit_staged(
        &self,
        sign: Sign,
        commit_message: &str,
        tag: Option<&str>,
    ) -> Result<(), Error> {
        log::trace!("Commit staged with sign {sign:?}");

        let commit_message = if !self.commit_message.is_empty() {
            &self.commit_message.clone()
        } else if !commit_message.is_empty() {
            commit_message
        } else {
            DEFAULT_COMMIT_MESSAGE
        };

        log::debug!("Commit message: {commit_message}");

        let mut index = self.git_repo.index()?;
        let tree_id = index.write_tree()?;
        let head = self.git_repo.head()?;
        let parent = self.git_repo.find_commit(head.target().unwrap())?;
        let sig = self.git_repo.signature()?;

        let commit_id = match sign {
            Sign::None => self.git_repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                commit_message,
                &self.git_repo.find_tree(tree_id)?,
                &[&parent],
            )?,
            Sign::Gpg => {
                let commit_buffer = self.git_repo.commit_create_buffer(
                    &sig,
                    &sig,
                    commit_message,
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

                // manually advance to the new commit id
                self.git_repo
                    .head()?
                    .set_target(commit_id, commit_message)?;

                log::trace!("head updated");

                commit_id
            }
        };

        if let Some(version_tag) = tag {
            let version_tag = format!("v{}", version_tag);
            self.create_tag(&version_tag, commit_id, &sig)?;
        }

        Ok(())
    }

    fn push_commit(&self, version: Option<&str>, no_push: bool) -> Result<(), Error> {
        let mut remote = self.git_repo.find_remote("origin")?;
        log::trace!("Pushing changes to {:?}", remote.name());
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap())
        });
        let mut connection = remote.connect_auth(Direction::Push, Some(callbacks), None)?;
        let remote = connection.remote();

        let local_branch = self
            .git_repo
            .find_branch(self.branch_or_main(), BranchType::Local)?;
        log::trace!("Found local branch: {}", local_branch.name()?.unwrap());

        if log_enabled!(log::Level::Trace) {
            list_tags();
        };

        let branch_ref = local_branch.into_reference();

        let mut push_refs = vec![branch_ref.name().unwrap()];

        #[allow(unused_assignments)]
        let mut tag_ref = String::from("");

        if let Some(version_tag) = version {
            log::trace!("Found version tag: {}", version_tag);
            tag_ref = format!("refs/tags/v{version_tag}");
            log::trace!("Tag ref: {tag_ref}");
            push_refs.push(&tag_ref);
        };

        log::trace!("Push refs: {:?}", push_refs);
        let mut call_backs = RemoteCallbacks::new();
        call_backs.push_transfer_progress(progress_bar);
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(call_backs);

        if !no_push {
            remote.push(&push_refs, Some(&mut push_opts))?;
        }

        Ok(())
    }

    /// Rebase the next pr of dependency updates if any
    #[instrument(skip(self))]
    async fn rebase_next_pr(&self, login: Option<&str>) -> Result<Option<String>, Error> {
        tracing::debug!("Rebase next PR");

        let prs = self.get_open_pull_requests().await?;

        if prs.is_empty() {
            return Ok(None);
        };

        tracing::trace!("Found {:?} open PRs", prs);

        // filter to PRs created by a specfic login

        let login = if let Some(login) = login {
            login
        } else {
            DEFAULT_REBASE_LOGIN
        };

        let mut prs: Vec<_> = prs.iter().filter(|pr| pr.login == login).collect();

        if prs.is_empty() {
            tracing::trace!("Found no open PRs for {login}");
            return Ok(None);
        };

        tracing::trace!("Found {:?} open PRs for {login}", prs);

        prs.sort_by(|a, b| a.number.cmp(&b.number));
        let next_pr = &prs[0];

        tracing::trace!("Next PR: {}", next_pr.number);

        self.add_label_to_pr(next_pr.number).await?;

        Ok(Some(next_pr.number.to_string()))
    }

    fn branch_list(&self) -> Result<String, Error> {
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

    fn branch_status(&self) -> Result<String, Error> {
        let branch_remote = self.git_repo.find_branch(
            format!("origin/{}", self.branch_or_main()).as_str(),
            git2::BranchType::Remote,
        )?;

        if branch_remote.get().target() == self.git_repo.head()?.target() {
            return Ok(format!(
                "\n\nOn branch {}\nYour branch is up to date with `{}`\n",
                self.branch_or_main(),
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

fn progress_bar(current: usize, total: usize, bytes: usize) {
    let percent = (current as f32 / total as f32) * 100.0;

    let percent = percent as u8;

    log::trace!("Calculated percent: {}", percent);

    match percent {
        10 => log::trace!("{}%", percent),
        25 => log::trace!("{}%", percent),
        40 => log::trace!("{}%", percent),
        55 => log::trace!("{}%", percent),
        80 => log::trace!("{}%", percent),
        95 => log::trace!("{}%", percent),
        100 => log::trace!("{}%", percent),
        _ => {}
    }

    log::trace!(
        "{}:- current: {}, total: {}, bytes: {}",
        "Push progress".blue().underline().bold(),
        current.blue().bold(),
        total.blue().bold(),
        bytes.blue().bold()
    );
}

fn list_tags() {
    let output = Command::new("ls")
        .arg("-R")
        .arg(".git/refs")
        .output()
        .expect("ls of the git refs");
    let stdout = output.stdout;
    log::trace!("ls: {}", String::from_utf8_lossy(&stdout));

    let out_string = String::from_utf8_lossy(&stdout);

    let files = out_string.split_terminator("\n").collect::<Vec<&str>>();
    log::trace!("Files: {:#?}", files);
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
