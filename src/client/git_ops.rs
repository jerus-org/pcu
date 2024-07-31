use std::{
    fs::read_to_string,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use git2::{BranchType, Cred, Direction, Oid, RemoteCallbacks, Signature};
use octocrab::Octocrab;

use crate::Error;

use super::Client;

const GIT_USER_SIGNATURE: &str = "user.signingkey";

pub trait GitOps {
    fn create_tag(&self, tag: &str, commit_id: Oid, sig: &Signature) -> Result<(), Error>;
    #[allow(async_fn_in_trait)]
    async fn get_commitish_for_tag(
        &self,
        octocrab: &Octocrab,
        version: &str,
    ) -> Result<String, Error>;
    fn push_changelog(&self, version: Option<&str>) -> Result<(), Error>;
    fn commit_changelog_gpg(&mut self, tag: Option<&str>) -> Result<String, Error>;
    fn commit_changelog(&self, tag: Option<&str>) -> Result<String, Error>;
}

impl GitOps for Client {
    #[allow(dead_code)]
    fn commit_changelog(&self, tag: Option<&str>) -> Result<String, Error> {
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

        if let Some(version_tag) = tag {
            let version_tag = format!("v{}", version_tag);
            self.create_tag(&version_tag, commit_id, &sig)?;
        };

        Ok(commit_id.to_string())
    }

    #[allow(dead_code)]
    fn commit_changelog_gpg(&mut self, tag: Option<&str>) -> Result<String, Error> {
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

        if let Some(version_tag) = tag {
            let version_tag = format!("v{}", version_tag);
            self.create_tag(&version_tag, commit_id, &sig)?;
        };

        // manually advance to the new commit id
        self.git_repo.head()?.set_target(commit_id, &msg)?;

        log::trace!("head updated");

        Ok(commit_id.to_string())
    }

    fn create_tag(&self, tag: &str, commit_id: Oid, sig: &Signature) -> Result<(), Error> {
        let object = self.git_repo.find_object(commit_id, None)?;
        self.git_repo.tag(tag, &object, sig, tag, true)?;

        let mut revwalk = self.git_repo.revwalk()?;
        let reference = format!("refs/tags/{tag}");
        revwalk.push_ref(&reference)?;
        Ok(())
    }

    fn push_changelog(&self, version: Option<&str>) -> Result<(), Error> {
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
            Some(ref branch) => {
                log::trace!("*** found a branch: {}", branch);
                branch
            }
            None => {
                log::trace!("*** no branch found, defaulting to main");
                "main"
            }
        };

        let local_branch = self.git_repo.find_branch(branch, BranchType::Local)?;
        log::trace!("Found branch: {}", local_branch.name()?.unwrap());

        log::trace!("Got these refs: {:#?}", list_tags());

        let branch_ref = local_branch.into_reference();

        let mut push_refs = vec![branch_ref.name().unwrap()];

        let tag_ref = if let Some(version_tag) = version {
            log::trace!("Found version tag: {}", version_tag);
            format!("/refs/tags/v{version_tag}")
        } else {
            String::from("")
        };
        if !tag_ref.is_empty() {
            push_refs.push(&tag_ref);
        }
        log::trace!("Push refs: {:?}", push_refs);

        remote.push(&push_refs, None)?;

        Ok(())
    }

    async fn get_commitish_for_tag(&self, octocrab: &Octocrab, tag: &str) -> Result<String, Error> {
        log::trace!("Get commitish for tag: {tag}");
        for t in octocrab
            .repos(self.owner(), self.repo())
            .list_tags()
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
}

fn list_tags() -> String {
    let output = Command::new("ls")
        .arg(".git/refs/tags")
        .output()
        .expect("ls of the git refs");
    let stdout = output.stdout;
    log::trace!("ls: {}", String::from_utf8_lossy(&stdout));

    let out_string = String::from_utf8_lossy(&stdout);

    let files = out_string.split_terminator(" ").collect::<Vec<&str>>();
    log::trace!("Files: {:#?}", files);

    if let Some(last_file) = files.last() {
        let filename = last_file.to_string();
        let filename = format!(".git/refs/tags/{filename}");
        log::trace!("Filename: {filename}");
        let file_contents = read_to_string(&filename).unwrap_or("nothing read".to_string());
        log::trace!("File contents: {file_contents}");
        format!("{}\n`{}`", filename, file_contents)
    } else {
        "".to_string()
    }
}
