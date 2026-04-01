use clap::Parser;
use color_eyre::Result;

use super::CIExit;
use crate::Error;

#[derive(Debug, Parser, Clone)]
pub struct Trigger {
    /// Webhook URL to POST to
    #[clap(long)]
    pub webhook: String,
    /// Log what would be sent without making the HTTP request
    #[clap(long)]
    pub dry_run: bool,
}

impl Trigger {
    pub async fn run(&self) -> Result<CIExit, Error> {
        if self.dry_run {
            log::info!("Dry run: would POST to {}", self.webhook);
            return Ok(CIExit::WebhookTriggered(self.webhook.clone()));
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&self.webhook)
            .send()
            .await
            .map_err(|e| Error::Trigger(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unreadable body>"));

        log::info!("POST {} → {} {}", self.webhook, status.as_u16(), body);

        if status.is_success() {
            Ok(CIExit::WebhookTriggered(self.webhook.clone()))
        } else {
            Err(Error::Trigger(format!(
                "Webhook returned non-2xx status {}: {}",
                status.as_u16(),
                body
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::Cli;

    #[test]
    fn test_trigger_parses_webhook_flag() {
        let args = Cli::try_parse_from(["pcu", "trigger", "--webhook", "https://example.com/hook"])
            .unwrap();
        match args.command {
            crate::Commands::Trigger(t) => {
                assert_eq!(t.webhook, "https://example.com/hook");
                assert!(!t.dry_run);
            }
            _ => panic!("expected Trigger command"),
        }
    }

    #[test]
    fn test_trigger_parses_dry_run_flag() {
        let args = Cli::try_parse_from([
            "pcu",
            "trigger",
            "--webhook",
            "https://example.com/hook",
            "--dry-run",
        ])
        .unwrap();
        match args.command {
            crate::Commands::Trigger(t) => {
                assert!(t.dry_run);
            }
            _ => panic!("expected Trigger command"),
        }
    }

    #[test]
    fn test_trigger_requires_webhook() {
        // Missing --webhook should fail to parse
        let result = Cli::try_parse_from(["pcu", "trigger"]);
        assert!(result.is_err(), "trigger without --webhook should fail");
    }
}
