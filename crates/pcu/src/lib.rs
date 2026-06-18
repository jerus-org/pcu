mod cli;
mod client;
mod error;
mod ops;
mod pr_title;
mod utilities;
mod workspace;

pub use cli::{CIExit, Cli, Commands};
pub use client::Client;
pub use error::{Error, GraphQLWrapper};
pub use ops::{
    export_ci_branch, import_gpg_key, write_ci_branch_export, GitOps, MakeRelease, Sign,
    SignConfig, UpdateFromPr,
};
pub use pr_title::PrTitle;
pub use workspace::{Package, Workspace};

#[cfg(test)]
mod git2_build_features {
    //! Guards the linked libgit2 build features. `git2` 0.21 changed its
    //! default features to `[]` (0.20 defaulted to `["ssh", "https"]`), so a
    //! bare `git2 = "0.21"` builds libgit2 WITHOUT a TLS backend — any HTTPS
    //! git operation then fails at runtime with
    //! `there is no TLS stream available; class=Ssl (16)` (pcu checkout/pr/push).
    //! These tests fail fast at build/test time if the features regress again.

    #[test]
    fn libgit2_has_https_support() {
        assert!(
            git2::Version::get().https(),
            "linked libgit2 has no HTTPS/TLS backend — pcu's HTTPS git operations \
             (checkout/pr/push) will fail with 'there is no TLS stream available'. \
             Ensure the `git2` dependency enables the `https` feature (git2 0.21 \
             dropped it from the default feature set)."
        );
    }

    #[test]
    fn libgit2_has_ssh_support() {
        assert!(
            git2::Version::get().ssh(),
            "linked libgit2 has no SSH backend — SSH git remotes will fail. \
             Ensure the `git2` dependency enables the `ssh` feature (git2 0.21 \
             dropped it from the default feature set)."
        );
    }
}
