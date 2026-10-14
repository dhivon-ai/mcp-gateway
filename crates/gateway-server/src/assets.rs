use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
pub async fn index() -> Response {
    match INDEX {
        Some(html)=>Html(html).into_response(),
        None=>(StatusCode::SERVICE_UNAVAILABLE,"Administrator assets are not bundled. Build the website and compile with GATEWAY_WEB_ASSETS pointing to its output directory.").into_response(),
    }
}
pub async fn asset(Path(path): Path<String>) -> Response {
    let name = format!("assets/{path}");
    match ASSETS.iter().find(|(file, _, _)| *file == name) {
        Some((_, mime, bytes)) => {
            ([(header::CONTENT_TYPE, *mime)], Body::from(*bytes)).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
