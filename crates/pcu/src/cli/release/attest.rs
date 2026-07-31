//! SLSA provenance attestation, gated behind the `attest` feature.
//!
//! Split out so that consumers taking `default-features = false` — pcu used
//! purely for git and GitHub operations — do not compile, or depend on,
//! `openidconnect` and `sigstore`. Those carry `rsa`, whose Marvin Attack
//! advisory (RUSTSEC-2023-0071) has no fixed version, so a consumer that never
//! signs anything would otherwise have to suppress an advisory it cannot act
//! on.
//!
//! Nothing here is reachable without the feature: the `Attest` subcommand and
//! its dispatch arm are gated too, so the CLI simply does not offer the command
//! when it is compiled out.

use octocrate::{APIConfig, PersonalAccessToken};

use super::{resolve_version, Mode, Release};
use crate::{CIExit, Client, Error};

impl Release {
    /// Attest a published crate with SLSA v0.2 provenance signed via Sigstore keyless.
    ///
    /// Steps:
    /// 1. Download the .crate from crates.io (with retry for indexing delay)
    /// 2. Compute SHA256 of the downloaded artifact
    /// 3. Generate SLSA v0.2 provenance JSON recording source, environment, and artifact
    /// 4. Sign the .crate with cosign-compatible keyless signing (CircleCI OIDC → Fulcio → Rekor)
    /// 5. Upload the .sigstore.json bundle and provenance.json to the GitHub release
    ///
    /// Requires CIRCLE_OIDC_TOKEN_V2 with audience "sigstore" in the environment.
    pub(super) async fn attest(self, client: Client) -> Result<CIExit, Error> {
        let Mode::Attest(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let version = resolve_version(&cmd.version);
        if should_skip_attest(&version) {
            log::info!("No version to attest — skipping");
            return Ok(CIExit::Released);
        }

        let pkg = &cmd.package;
        let crate_filename = format!("{pkg}-{version}.crate");
        let crate_url = format!("https://static.crates.io/crates/{pkg}/{crate_filename}");
        let bundle_filename = format!("{crate_filename}.sigstore.json");
        let provenance_filename = format!("{pkg}-{version}.provenance.json");

        // Step 1: Check whether attestation assets already exist on the GitHub release.
        // If both assets are present the previous run completed successfully — skip all work.
        let release_tag = format!("{}{}", cmd.crate_tag_prefix, version);
        // Draft-aware: attestation assets are uploaded BEFORE publication on a
        // draft-first pipeline (assets freeze at publication), so the release
        // being attested is normally still a draft — invisible to
        // get_release_by_tag.
        let release_ref = client
            .find_release_for_tag(&release_tag)
            .await?
            .ok_or_else(|| {
                Error::Attestation(format!(
                "no GitHub release found for tag '{release_tag}' to attach attestation assets to"
            ))
            })?;
        let release = client
            .github_rest
            .repos
            .get_release(client.owner(), client.repo(), release_ref.id)
            .send()
            .await?;
        let existing_assets: std::collections::HashSet<String> =
            release.assets.iter().map(|a| a.name.clone()).collect();
        if attestation_assets_already_uploaded(
            &existing_assets,
            &bundle_filename,
            &provenance_filename,
        ) {
            log::info!("Attestation assets already present on release {release_tag} — skipping");
            return Ok(CIExit::Released);
        }

        let attest_dir = std::path::Path::new("/tmp/attestation");
        std::fs::create_dir_all(attest_dir)?;
        let crate_path = attest_dir.join(&crate_filename);

        // Step 2: Download .crate from crates.io with retry
        log::info!(
            "Waiting {}s for crates.io indexing before download...",
            cmd.crates_io_delay
        );
        tokio::time::sleep(std::time::Duration::from_secs(cmd.crates_io_delay)).await;

        let http_client = reqwest::Client::new();
        let crate_bytes =
            download_with_retry(
                &crate_filename,
                cmd.max_attempts.into(),
                std::time::Duration::from_secs(30),
                || {
                    let client = http_client.clone();
                    let url = crate_url.clone();
                    async move {
                        let response =
                            client.get(&url).send().await.map_err(|e| {
                                Error::Attestation(format!("HTTP request failed: {e}"))
                            })?;
                        if !response.status().is_success() {
                            return Err(Error::Attestation(format!(
                                "HTTP {} for {url}",
                                response.status()
                            )));
                        }
                        response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
                            Error::Attestation(format!("Failed to read response: {e}"))
                        })
                    }
                },
            )
            .await?;
        std::fs::write(&crate_path, &crate_bytes)?;

        // Step 3: Read bytes and compute SHA256
        use sha2::Digest as _;
        let hash_hex = sha2::Sha256::digest(&crate_bytes).iter().fold(
            String::with_capacity(64),
            |mut s, b| {
                use std::fmt::Write as _;
                write!(s, "{b:02x}").unwrap();
                s
            },
        );
        log::info!("SHA256({crate_filename}) = {hash_hex}");

        // Step 4: Generate SLSA v0.2 provenance JSON
        let build_started = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let rust_version = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let provenance = serde_json::json!({
            "builder": {
                "id": std::env::var("CIRCLE_BUILD_URL").unwrap_or_default()
            },
            "buildType": "https://github.com/jerus-org/circleci-toolkit",
            "invocation": {
                "configSource": {
                    "uri": std::env::var("CIRCLE_REPOSITORY_URL").unwrap_or_default(),
                    "digest": { "sha1": std::env::var("CIRCLE_SHA1").unwrap_or_default() },
                    "entryPoint": ".circleci/release.yml"
                },
                "parameters": {
                    "package": pkg,
                    "version": &version,
                    "rust_version": &rust_version
                },
                "environment": {
                    "CIRCLE_BUILD_URL": std::env::var("CIRCLE_BUILD_URL").unwrap_or_default(),
                    "CIRCLE_WORKFLOW_ID": std::env::var("CIRCLE_WORKFLOW_ID").unwrap_or_default(),
                    "CIRCLE_PROJECT_USERNAME": std::env::var("CIRCLE_PROJECT_USERNAME").unwrap_or_default(),
                    "CIRCLE_PROJECT_REPONAME": std::env::var("CIRCLE_PROJECT_REPONAME").unwrap_or_default()
                }
            },
            "metadata": {
                "buildStartedOn": build_started,
                "completeness": { "parameters": true, "environment": true, "materials": true },
                "reproducible": false
            },
            "materials": [
                {
                    "uri": std::env::var("CIRCLE_REPOSITORY_URL").unwrap_or_default(),
                    "digest": { "sha1": std::env::var("CIRCLE_SHA1").unwrap_or_default() }
                }
            ],
            "subject": [
                {
                    "name": &crate_filename,
                    "digest": { "sha256": &hash_hex }
                }
            ]
        });

        let provenance_path = attest_dir.join(&provenance_filename);
        std::fs::write(&provenance_path, serde_json::to_string_pretty(&provenance)?)?;
        log::info!("Generated provenance: {provenance_filename}");

        // Step 5: Sign with Sigstore keyless (CircleCI OIDC → Fulcio v1 → Rekor)
        let oidc_token_str = get_oidc_token()?;

        log::info!("Signing {crate_filename} via Fulcio v1 API...");
        let bundle_json = sign_artifact_fulcio_v1(&crate_bytes, &oidc_token_str).await?;

        let bundle_path = attest_dir.join(&bundle_filename);
        std::fs::write(&bundle_path, &bundle_json)?;
        log::info!("Bundle written: {bundle_filename}");

        // Step 6: Upload bundle and provenance to GitHub release
        log::info!("Uploading attestation assets to release {release_tag}...");

        let upload_token = PersonalAccessToken::new(client.github_token.clone());
        let upload_config = APIConfig::new("https://uploads.github.com", upload_token);
        let upload_api = octocrate::GitHubAPI::new(&upload_config);

        for (path, name) in [
            (&bundle_path, bundle_filename.as_str()),
            (&provenance_path, provenance_filename.as_str()),
        ] {
            let file = tokio::fs::File::open(path).await?;
            let content_length = file.metadata().await?.len();
            let query = octocrate::repos::upload_release_asset::Query::builder()
                .name(name)
                .build();
            upload_api
                .repos
                .upload_release_asset(client.owner(), client.repo(), release.id)
                .query(&query)
                .header("Content-Type", "application/octet-stream")
                .header("Content-Length", content_length.to_string())
                .file(file)
                .send()
                .await?;
            log::info!("Uploaded {name}");
        }

        log::info!("Attestation complete.");
        log::info!(
            "Verify with: cosign verify-blob \
        --certificate-oidc-issuer 'https://oidc.circleci.com/org/<ORG_ID>' \
        --certificate-identity-regexp 'https://circleci.com/gh/{}/.*' \
        --bundle '{bundle_filename}' '{crate_filename}'",
            std::env::var("CIRCLE_PROJECT_USERNAME").unwrap_or_default()
        );

        Ok(CIExit::Released)
    }
}

/// Returns true if the version string indicates no release is needed.
fn should_skip_attest(version: &str) -> bool {
    version == "none"
}

/// Returns true if all expected attestation assets are already present on the
/// GitHub release, indicating the upload was completed in a previous run.
///
/// When true, the entire attest operation (download, sign, upload) can be
/// skipped, making `pcu release attest` idempotent on re-runs.
fn attestation_assets_already_uploaded(
    existing_asset_names: &std::collections::HashSet<String>,
    bundle_filename: &str,
    provenance_filename: &str,
) -> bool {
    existing_asset_names.contains(bundle_filename)
        && existing_asset_names.contains(provenance_filename)
}

/// Read the CircleCI OIDC token (v2) from the environment.
///
/// Returns `Error::Attestation` with a clear message if the variable is unset.
fn get_oidc_token() -> Result<String, Error> {
    std::env::var("CIRCLE_OIDC_TOKEN_V2").map_err(|_| {
        Error::Attestation(
            "CIRCLE_OIDC_TOKEN_V2 is not set. \
            Set it to a CircleCI OIDC token with audience 'sigstore'. \
            Use `circleci run oidc get --claims '{\"aud\":\"sigstore\"}'` to obtain one."
                .to_string(),
        )
    })
}

/// Extract the `sub` claim from a raw JWT string without requiring an `email` claim.
///
/// CircleCI machine OIDC tokens do not include an `email` field; only `sub` is needed
/// as the challenge value for the Fulcio v1 signing endpoint.
fn extract_sub_from_jwt(raw_jwt: &str) -> Result<String, Error> {
    use base64::Engine as _;
    let parts: Vec<&str> = raw_jwt.split('.').collect();
    if parts.len() < 2 {
        return Err(Error::Attestation(
            "Invalid JWT format: expected at least 2 dot-separated parts".to_string(),
        ));
    }
    // JWT uses base64url (URL_SAFE_NO_PAD); fall back to STANDARD_NO_PAD for test tokens.
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .map_err(|e| Error::Attestation(format!("JWT payload base64 decode failed: {e}")))?;
    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|e| Error::Attestation(format!("JWT payload JSON parse failed: {e}")))?;
    claims["sub"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Attestation("JWT missing 'sub' claim".to_string()))
}

/// Decode the first PEM certificate block to its raw DER bytes.
///
/// Fulcio v1 returns a certificate chain (leaf + intermediates) as multiple
/// PEM blocks.  Only the leaf (first block) is needed for the Sigstore bundle.
/// Joining all blocks before decoding would embed `=` padding mid-string,
/// causing base64 decode to fail with "Invalid symbol 61".
fn pem_to_der(pem_str: &str) -> Result<Vec<u8>, Error> {
    use base64::Engine as _;
    let b64: String = pem_str
        .lines()
        .skip_while(|l| !l.starts_with("-----BEGIN"))
        .skip(1)
        .take_while(|l| !l.starts_with("-----END"))
        .collect::<Vec<_>>()
        .join("");
    base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .map_err(|e| Error::Attestation(format!("PEM to DER conversion failed: {e}")))
}

/// Sign `artifact` bytes using the Fulcio v1 API with a CircleCI OIDC token.
///
/// The v1 path (`FulcioClient::request_cert`) uses `TokenProvider::Static` and signs
/// the challenge (= `sub` claim) to prove key possession.  It does NOT require an
/// `email` claim — making it compatible with CircleCI machine OIDC tokens.
///
/// Returns the Sigstore bundle JSON string.
async fn sign_artifact_fulcio_v1(artifact: &[u8], oidc_token_str: &str) -> Result<String, Error> {
    use base64::Engine as _;
    use sha2::Digest as _;
    use sigstore::crypto::SigningScheme;
    use sigstore::fulcio::{FulcioClient, TokenProvider, FULCIO_ROOT};
    use sigstore::rekor::apis::configuration::Configuration as RekorConfiguration;
    use sigstore::rekor::apis::entries_api::create_log_entry;
    use sigstore::rekor::models::hashedrekord;
    use sigstore::rekor::models::proposed_entry::ProposedEntry as ProposedLogEntry;
    use sigstore_protobuf_specs::dev::sigstore::bundle::v1::bundle;
    use sigstore_protobuf_specs::dev::sigstore::bundle::v1::verification_material;
    use sigstore_protobuf_specs::dev::sigstore::bundle::v1::{Bundle, VerificationMaterial};
    use sigstore_protobuf_specs::dev::sigstore::common::v1::{
        HashAlgorithm, HashOutput, MessageSignature, X509Certificate, X509CertificateChain,
    };
    use sigstore_protobuf_specs::dev::sigstore::rekor::v1::TransparencyLogEntry;
    use url::Url;

    // Extract sub claim (challenge for Fulcio)
    let sub = extract_sub_from_jwt(oidc_token_str)?;

    // Build CoreIdToken from raw JWT string
    let core_token: openidconnect::core::CoreIdToken =
        serde_json::from_value(serde_json::Value::String(oidc_token_str.to_string()))
            .map_err(|e| Error::Attestation(format!("Failed to parse OIDC token: {e}")))?;

    // Create Fulcio client with v1 Static provider
    let fulcio_url = Url::parse(FULCIO_ROOT)
        .map_err(|e| Error::Attestation(format!("Invalid Fulcio URL: {e}")))?;
    let fulcio = FulcioClient::new(fulcio_url, TokenProvider::Static((core_token, sub)));

    // Request Fulcio certificate via v1 endpoint
    log::info!("Requesting Fulcio signing certificate via v1 API...");
    let (signer, cert_pem) = fulcio
        .request_cert(SigningScheme::ECDSA_P256_SHA256_ASN1)
        .await
        .map_err(|e| Error::Attestation(format!("Fulcio certificate request failed: {e}")))?;

    // Compute SHA256 of artifact
    let sha256_hash = sha2::Sha256::digest(artifact);
    let sha256_hex = sha256_hash
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write as _;
            write!(s, "{b:02x}").unwrap();
            s
        });

    // Sign artifact bytes
    let signature_bytes = signer
        .sign(artifact)
        .map_err(|e| Error::Attestation(format!("Artifact signing failed: {e}")))?;

    // Convert cert PEM to DER for the bundle
    let cert_der = pem_to_der(&cert_pem.to_string())?;

    // Submit to Rekor transparency log
    let proposed_entry = ProposedLogEntry::Hashedrekord {
        api_version: "0.0.1".to_owned(),
        spec: hashedrekord::Spec {
            signature: hashedrekord::Signature {
                content: base64::engine::general_purpose::STANDARD.encode(&signature_bytes),
                public_key: hashedrekord::PublicKey::new(
                    base64::engine::general_purpose::STANDARD.encode(cert_pem.as_ref()),
                ),
            },
            data: hashedrekord::Data {
                hash: hashedrekord::Hash {
                    algorithm: hashedrekord::AlgorithmKind::sha256,
                    value: sha256_hex,
                },
            },
        },
    };

    log::info!("Submitting to Rekor transparency log...");
    let log_entry = create_log_entry(&RekorConfiguration::default(), proposed_entry)
        .await
        .map_err(|e| Error::Attestation(format!("Rekor submission failed: {e}")))?;
    let tlog_entry: TransparencyLogEntry = log_entry
        .try_into()
        .map_err(|_| Error::Attestation("Rekor returned malformed log entry".to_string()))?;

    // Build Sigstore bundle
    let x509_chain = X509CertificateChain {
        certificates: vec![X509Certificate {
            raw_bytes: cert_der,
        }],
    };
    let verification_material = Some(VerificationMaterial {
        timestamp_verification_data: None,
        tlog_entries: vec![tlog_entry],
        content: Some(verification_material::Content::X509CertificateChain(
            x509_chain,
        )),
    });
    let message_signature = MessageSignature {
        message_digest: Some(HashOutput {
            algorithm: HashAlgorithm::Sha2256.into(),
            digest: sha256_hash.to_vec(),
        }),
        signature: signature_bytes,
    };
    let bundle = Bundle {
        media_type: "application/vnd.dev.sigstore.bundle+json;version=0.2".to_string(),
        verification_material,
        content: Some(bundle::Content::MessageSignature(message_signature)),
    };

    serde_json::to_string_pretty(&bundle)
        .map_err(|e| Error::Attestation(format!("Bundle serialisation failed: {e}")))
}

/// Downloads a URL with retry, using a caller-supplied async attempt function.
///
/// `attempt_fn` is called up to `max_attempts` times. On success it returns
/// `Ok(Vec<u8>)` containing the downloaded bytes. On failure the error is
/// logged and (if attempts remain) the retry delay is observed before the next
/// attempt. After all attempts are exhausted an `Error::Attestation` is
/// returned naming the file and the attempt count.
async fn download_with_retry<F, Fut>(
    crate_filename: &str,
    max_attempts: u64,
    retry_delay: std::time::Duration,
    mut attempt_fn: F,
) -> Result<Vec<u8>, Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>, Error>>,
{
    for attempt in 1..=max_attempts {
        match attempt_fn().await {
            Ok(bytes) => {
                log::info!("Downloaded {crate_filename} (attempt {attempt})");
                return Ok(bytes);
            }
            Err(e) => {
                log::warn!("Download attempt {attempt} failed: {e}");
                if attempt < max_attempts {
                    log::info!("Retrying in {}s...", retry_delay.as_secs());
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }
    Err(Error::Attestation(format!(
        "Failed to download {crate_filename} after {max_attempts} attempts"
    )))
}

#[cfg(test)]
mod attest_tests {
    use super::*;

    #[tokio::test]
    async fn download_with_retry_succeeds_on_first_attempt() {
        let result =
            download_with_retry("test-1.0.0.crate", 3, std::time::Duration::ZERO, || async {
                Ok(b"crate-data".to_vec())
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"crate-data");
    }

    #[tokio::test]
    async fn download_with_retry_returns_err_after_all_attempts_exhausted() {
        let result =
            download_with_retry("test-1.0.0.crate", 3, std::time::Duration::ZERO, || async {
                Err(Error::Attestation("HTTP 503".to_string()))
            })
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("test-1.0.0.crate"),
            "error should name the file: {msg}"
        );
        assert!(
            msg.contains('3'),
            "error should mention attempt count: {msg}"
        );
    }

    #[tokio::test]
    async fn download_with_retry_succeeds_on_second_attempt() {
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let result =
            download_with_retry("test-1.0.0.crate", 3, std::time::Duration::ZERO, || async {
                let n = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 0 {
                    Err(Error::Attestation("first attempt failed".to_string()))
                } else {
                    Ok(b"crate-data".to_vec())
                }
            })
            .await;
        assert!(result.is_ok());
    }

    /// Build a minimal fake JWT string.
    ///
    /// Uses URL_SAFE_NO_PAD base64 (standard JWT encoding).
    /// The signature is fake — we only parse the payload claims.
    fn fake_jwt(sub: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(
            r#"{{"aud":"sigstore","exp":9999999999,"sub":"{sub}"}}"#
        ));
        format!("{header}.{payload}.fakesig")
    }

    #[test]
    fn attest_skips_when_version_is_none() {
        assert!(
            should_skip_attest("none"),
            "version 'none' should trigger skip"
        );
    }

    #[test]
    fn attest_does_not_skip_when_version_is_present() {
        assert!(
            !should_skip_attest("1.2.3"),
            "a real version should not trigger skip"
        );
    }

    #[test]
    fn get_oidc_token_errors_when_env_var_missing() {
        let saved = std::env::var("CIRCLE_OIDC_TOKEN_V2").ok();
        unsafe { std::env::remove_var("CIRCLE_OIDC_TOKEN_V2") };

        let result = get_oidc_token();

        if let Some(v) = saved {
            unsafe { std::env::set_var("CIRCLE_OIDC_TOKEN_V2", v) };
        }

        assert!(result.is_err(), "should error when env var is absent");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("CIRCLE_OIDC_TOKEN_V2"),
            "error should name the missing env var: {msg}"
        );
    }

    #[test]
    fn get_oidc_token_returns_value_when_env_var_set() {
        let saved = std::env::var("CIRCLE_OIDC_TOKEN_V2").ok();
        unsafe { std::env::set_var("CIRCLE_OIDC_TOKEN_V2", "some-token") };

        let result = get_oidc_token();

        unsafe { std::env::remove_var("CIRCLE_OIDC_TOKEN_V2") };
        if let Some(v) = saved {
            unsafe { std::env::set_var("CIRCLE_OIDC_TOKEN_V2", v) };
        }

        assert_eq!(result.unwrap(), "some-token");
    }

    #[test]
    fn extract_sub_from_jwt_returns_sub_claim() {
        let jwt = fake_jwt("https://circleci.com/org/abc/project/xyz/user/u");
        let result = extract_sub_from_jwt(&jwt);
        assert!(result.is_ok(), "should extract sub: {result:?}");
        assert_eq!(
            result.unwrap(),
            "https://circleci.com/org/abc/project/xyz/user/u"
        );
    }

    #[test]
    fn extract_sub_from_jwt_errors_on_malformed_jwt() {
        let result = extract_sub_from_jwt("not-a-jwt");
        assert!(result.is_err(), "malformed JWT should fail");
    }

    #[test]
    fn extract_sub_from_jwt_errors_when_sub_missing() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(r#"{"aud":"sigstore","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.fakesig");
        let result = extract_sub_from_jwt(&jwt);
        assert!(result.is_err(), "missing sub should fail");
        assert!(
            result.unwrap_err().to_string().contains("sub"),
            "error should mention 'sub'"
        );
    }

    #[test]
    fn pem_to_der_roundtrips_certificate_bytes() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        // Fabricate a fake "certificate" (just some bytes)
        let fake_der = b"FAKE_DER_BYTES_0123456789";
        let b64 = STANDARD.encode(fake_der);
        let pem = format!("-----BEGIN CERTIFICATE-----\n{b64}\n-----END CERTIFICATE-----\n");
        let result = pem_to_der(&pem);
        assert!(result.is_ok(), "pem_to_der should succeed: {result:?}");
        assert_eq!(result.unwrap(), fake_der);
    }

    #[test]
    fn pem_to_der_returns_only_first_cert_from_chain() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        // Fulcio v1 returns a chain: leaf cert + intermediate(s).
        // Only the leaf (first block) should be decoded; the intermediate's
        // base64 padding ('=') must not contaminate the leaf decode.
        let leaf_der = b"LEAF_CERT_BYTES";
        let intermediate_der = b"INTERMEDIATE_CERT_BYTES_LONGER";
        let leaf_b64 = STANDARD.encode(leaf_der);
        let intermediate_b64 = STANDARD.encode(intermediate_der);
        let chain_pem = format!(
            "-----BEGIN CERTIFICATE-----\n{leaf_b64}\n-----END CERTIFICATE-----\n\
             -----BEGIN CERTIFICATE-----\n{intermediate_b64}\n-----END CERTIFICATE-----\n"
        );
        let result = pem_to_der(&chain_pem);
        assert!(
            result.is_ok(),
            "pem_to_der should handle a cert chain: {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            leaf_der,
            "should return only the leaf (first) certificate"
        );
    }

    #[test]
    fn attestation_assets_already_uploaded_true_when_both_present() {
        let existing = std::collections::HashSet::from([
            "my-crate-1.2.3.crate.sigstore.json".to_string(),
            "my-crate-1.2.3.provenance.json".to_string(),
        ]);
        assert!(attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }
    #[test]
    fn attestation_assets_already_uploaded_false_when_both_absent() {
        let existing = std::collections::HashSet::new();
        assert!(!attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }
    #[test]
    fn attestation_assets_already_uploaded_false_when_only_bundle_present() {
        let existing =
            std::collections::HashSet::from(["my-crate-1.2.3.crate.sigstore.json".to_string()]);
        assert!(!attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }
    #[test]
    fn attestation_assets_already_uploaded_false_when_only_provenance_present() {
        let existing =
            std::collections::HashSet::from(["my-crate-1.2.3.provenance.json".to_string()]);
        assert!(!attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }
}
