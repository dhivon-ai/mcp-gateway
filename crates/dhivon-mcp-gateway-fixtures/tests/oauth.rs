use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dhivon_mcp_gateway_fixtures::{FixtureState, http_router};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn oauth_fixture_binds_pkce_resource_and_rotates_refresh_tokens() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let base = format!("http://{addr}");
    let router = http_router(
        FixtureState::new(base.clone(), "oauth".into()),
        addr.to_string(),
    );
    let task = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let verifier = "a".repeat(64);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let resource = format!("{base}/mcp");
    let redirect = "http://127.0.0.1:19001/oauth/callback";
    let response = client
        .get(format!("{base}/authorize"))
        .query(&[
            ("client_id", "dhivon-mcp-gateway-fixture-client"),
            ("redirect_uri", redirect),
            ("response_type", "code"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", "fixture-state"),
            ("resource", resource.as_str()),
        ])
        .send()
        .await?;
    assert!(response.status().is_redirection());
    let location = url::Url::parse(
        response
            .headers()
            .get("location")
            .ok_or_else(|| anyhow::anyhow!("missing redirect"))?
            .to_str()?,
    )?;
    let query = location
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        query.get("state").map(|s| s.as_ref()),
        Some("fixture-state")
    );
    let code = query
        .get("code")
        .ok_or_else(|| anyhow::anyhow!("missing code"))?;
    let token: Value = client
        .post(format!("{base}/token"))
        .form(&[
            ("client_id", "dhivon-mcp-gateway-fixture-client"),
            ("grant_type", "authorization_code"),
            ("resource", resource.as_str()),
            ("code", code.as_ref()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let refresh = token["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing refresh"))?;
    let form = [
        ("client_id", "dhivon-mcp-gateway-fixture-client"),
        ("grant_type", "refresh_token"),
        ("resource", resource.as_str()),
        ("refresh_token", refresh),
    ];
    assert!(
        client
            .post(format!("{base}/token"))
            .form(&form)
            .send()
            .await?
            .status()
            .is_success()
    );
    assert_eq!(
        client
            .post(format!("{base}/token"))
            .form(&form)
            .send()
            .await?
            .status(),
        400
    );
    task.abort();
    Ok(())
}
