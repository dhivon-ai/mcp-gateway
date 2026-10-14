use gateway_cli::{
    clock::{ClockSample, NANOS_PER_SECOND},
    oauth_protocol::OAuthProtocol,
};
use gateway_fixtures::{FixtureState, http_router};

#[tokio::test]
async fn advertised_metadata_and_provider_negatives_never_follow_a_token_redirect()
-> anyhow::Result<()> {
    use axum::{
        Router,
        http::{StatusCode, header},
        response::IntoResponse,
        routing::{get, post},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    };
    let destination = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let other = format!("http://{}/leak", destination.local_addr()?);
    let hits = Arc::new(AtomicUsize::new(0));
    let witness = hits.clone();
    let redirected = tokio::spawn(
        axum::serve(
            destination,
            Router::new().route(
                "/leak",
                post(move || {
                    let hits = witness.clone();
                    async move {
                        hits.fetch_add(1, Ordering::Relaxed);
                        StatusCode::OK
                    }
                }),
            ),
        )
        .into_future(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let mode = Arc::new(AtomicU8::new(0));
    let resource = format!("{base}/mcp");
    let protected_base = base.clone();
    let metadata_base = base.clone();
    let issuer_base = base.clone();
    let protected_mode = mode.clone();
    let issuer_mode = mode.clone();
    let app=Router::new().route("/mcp",get(move || {let base=protected_base.clone();async move {(StatusCode::UNAUTHORIZED,[(header::WWW_AUTHENTICATE,format!("Bearer resource_metadata=\"{base}/advertised-resource\""))]).into_response()}}))
        .route("/advertised-resource",get(move || {let base=metadata_base.clone();let mode=protected_mode.clone();async move {axum::Json(serde_json::json!({"resource":if mode.load(Ordering::Relaxed)==1 {format!("{base}/wrong")} else {format!("{base}/mcp")},"authorization_servers":[base],"scopes_supported":["tools"]}))}}))
        .route("/.well-known/oauth-authorization-server",get(move || {let base=issuer_base.clone();let mode=issuer_mode.clone();async move {
            let mode=mode.load(Ordering::Relaxed);
            axum::Json(serde_json::json!({"issuer":if mode==2 {"https://unrelated.invalid".into()} else {base.clone()},"authorization_endpoint":format!("{base}/authorize"),"token_endpoint":format!("{base}/token"),"response_types_supported":["code"],"code_challenge_methods_supported":[if mode==3 {"plain"} else {"S256"}],"token_endpoint_auth_methods_supported":[if mode==4 {"client_secret_basic"} else {"none"}],"authorization_response_iss_parameter_supported":true}))
        }}))
        .route("/token",post(move || {let other=other.clone();async move {(StatusCode::TEMPORARY_REDIRECT,[(header::LOCATION,other)]).into_response()}}));
    let fixture = tokio::spawn(axum::serve(listener, app).into_future());
    for failure in 1..=4 {
        mode.store(failure, Ordering::Relaxed);
        assert!(
            OAuthProtocol::discover(
                &resource,
                "public-client",
                &["tools".into()],
                "http://127.0.0.1:19001/oauth/callback",
                true
            )
            .await
            .is_err(),
            "unsupported or mismatched metadata mode{failure}"
        );
    }
    mode.store(0, Ordering::Relaxed);
    let protocol = OAuthProtocol::discover(
        &resource,
        "public-client",
        &["tools".into()],
        "http://127.0.0.1:19001/oauth/callback",
        true,
    )
    .await?;
    let now = ClockSample {
        boot_id: uuid::Uuid::new_v4(),
        continuous_ns: NANOS_PER_SECOND,
        wall_ns: gateway_cli::epoch()? as u64 * NANOS_PER_SECOND,
    };
    let consent = protocol.consent(now).await?;
    let mut callback = url::Url::parse("http://127.0.0.1:19001/oauth/callback")?;
    callback
        .query_pairs_mut()
        .append_pair("state", consent.state())
        .append_pair("code", "fixture-one-use-code")
        .append_pair("iss", &base);
    assert!(consent.exchange(callback.as_str(), now).await.is_err());
    assert_eq!(
        hits.load(Ordering::Relaxed),
        0,
        "token-bearing redirect never reached another destination"
    );
    fixture.abort();
    redirected.abort();
    Ok(())
}

#[tokio::test]
async fn sdk_public_client_validates_callback_then_rotates_resource_bound_tokens()
-> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let base = format!("http://{address}");
    let fixture = tokio::spawn(
        axum::serve(
            listener,
            http_router(
                FixtureState::new(base.clone(), "oauth".into()),
                address.to_string(),
            ),
        )
        .into_future(),
    );
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let protocol = OAuthProtocol::discover(
        &format!("{base}/mcp"),
        "gateway-fixture-client",
        &["tools".into()],
        "http://127.0.0.1:19001/oauth/callback",
        true,
    )
    .await?;
    let now = ClockSample {
        boot_id: uuid::Uuid::new_v4(),
        continuous_ns: NANOS_PER_SECOND,
        wall_ns: gateway_cli::epoch()? as u64 * NANOS_PER_SECOND,
    };
    let consent = protocol.consent(now).await?;
    let response = client.get(&consent.authorization_url).send().await?;
    assert!(response.status().is_redirection());
    let callback = response.headers()["location"].to_str()?.to_owned();
    let mut changed = url::Url::parse(&callback)?;
    changed
        .set_port(Some(19002))
        .map_err(|_| anyhow::anyhow!("fixture port"))?;
    assert!(consent.callback(changed.as_str(), now).is_err());
    let mut changed = url::Url::parse(&callback)?;
    changed.query_pairs_mut().append_pair("state", "duplicate");
    assert!(consent.callback(changed.as_str(), now).is_err());
    let forged = callback.replace(
        &url::form_urlencoded::byte_serialize(base.as_bytes()).collect::<String>(),
        "https%3A%2F%2Fevil.invalid",
    );
    assert!(consent.callback(&forged, now).is_err());
    assert!(
        consent.exchange(&forged, now).await.is_err(),
        "forged issuer must never exchange a code"
    );
    let mut no_issuer = url::Url::parse(&callback)?;
    let pairs: Vec<_> = no_issuer
        .query_pairs()
        .filter(|(key, _)| key != "iss")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    no_issuer.set_query(None);
    no_issuer.query_pairs_mut().extend_pairs(pairs);
    assert!(
        consent.exchange(no_issuer.as_str(), now).await.is_err(),
        "required issuer cannot be omitted"
    );
    let mut expired = now;
    expired.continuous_ns += 300 * NANOS_PER_SECOND;
    assert!(consent.callback(&callback, expired).is_err());
    let credential = consent.exchange(&callback, now).await?;
    assert_eq!(credential.resource, format!("{base}/mcp"));
    assert_eq!(credential.issuer, base);
    assert_eq!(credential.scopes, ["tools"]);
    assert!(credential.refresh_token.is_some());
    assert!(
        consent.exchange(&callback, now).await.is_err(),
        "one-use callback"
    );
    let updated = consent.protocol.refresh(&credential, now).await?;
    let mut wrong = updated.clone();
    wrong.resource = "https://unrelated.invalid/mcp".into();
    assert!(
        consent.protocol.refresh(&wrong, now).await.is_err(),
        "resource mismatch cannot forward a refresh token"
    );
    assert_ne!(updated.access_token, credential.access_token);
    assert_ne!(updated.refresh_token, credential.refresh_token);
    assert!(
        consent.protocol.refresh(&credential, now).await.is_err(),
        "rotated token cannot be replayed"
    );
    let response = client.post(format!("{base}/mcp")).bearer_auth(updated.access_token)
        .header("content-type","application/json").header("accept","application/json, text/event-stream")
        .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}})).send().await?;
    assert!(response.status().is_success());
    fixture.abort();
    Ok(())
}
