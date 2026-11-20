# Security Policy

## Supported versions

Security fixes are made against the **latest released version** of pcu only.
pcu is pre-1.0 and has no long-term-support branch, so the way to receive a fix
is to upgrade to the latest release.

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions, pull requests or comments.**

Report vulnerabilities privately using **GitHub's private vulnerability
reporting**:

<https://github.com/jerus-org/pcu/security/advisories/new>

(From the repository, use the **Security** tab, then **Report a vulnerability**.)

Please include as much of the following as you can, so we can reproduce and
assess the issue quickly:

- A description of the vulnerability and its potential impact.
- Steps to reproduce, or a proof of concept.
- The affected version(s) and your environment (OS, Rust toolchain).
- Any suggested remediation, if you have one.

## Our response process

This is a small project; timelines are best effort, but we aim to:

| Stage | Target |
|-------|--------|
| **Acknowledge** your report | within 3 business days |
| **Initial assessment** (triage, severity, whether it is accepted) | within 10 business days |
| **Fix and release** for accepted vulnerabilities | as soon as practical, prioritised by severity |
| **Disclosure** | coordinated with you after a fix is available |

We will keep you informed of progress. If you do not receive an acknowledgement
within the target window, please add a comment to your report.

## Disclosure and credit

- We follow **coordinated disclosure**: please give us reasonable time to
  release a fix before any public disclosure. We will agree the timing with you.
- We will **credit reporters** in the advisory and release notes, unless you ask
  to remain anonymous.
- Fixed vulnerabilities are published as a
  [GitHub Security Advisory](https://github.com/jerus-org/pcu/security/advisories)
  and noted in the release notes.

## Scope

pcu is a **CI support tool**, and most of its features are designed to run only
in CI.

In scope: vulnerabilities in pcu's own code and release artefacts, including
anything that lets a change get past the checks pcu performs.

Out of scope:

- Weaknesses in how you configure your own CI, credentials or repositories.
- Vulnerabilities in a dependency that do not affect pcu. Please report those
  to the dependency's maintainers.
- Findings from automated scanners without a demonstrated impact on pcu.
