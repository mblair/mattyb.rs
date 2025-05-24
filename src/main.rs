use axum::{
    extract::Request,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use handlebars::Handlebars;
use include_dir::{include_dir, Dir};
use pulldown_cmark::{html, Options, Parser};
use rustls_acme::{caches::DirCache, AcmeConfig};
use serde_json::json;
use std::sync::LazyLock;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

// Embed static files at compile time
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

// Pre-compiled templates and content
static HANDLEBARS: LazyLock<Handlebars> = LazyLock::new(|| {
    let mut hb = Handlebars::new();

    let template_content = STATIC_DIR
        .get_file("template.html")
        .expect("template.html not found")
        .contents_utf8()
        .expect("template.html is not valid UTF-8");

    hb.register_template_string("main", template_content)
        .expect("Failed to register template");

    hb
});

static MARKDOWN_CONTENT: LazyLock<String> = LazyLock::new(|| {
    let home_md = STATIC_DIR
        .get_file("home.md")
        .expect("home.md not found")
        .contents_utf8()
        .expect("home.md is not valid UTF-8");

    // Convert markdown to HTML
    let parser = Parser::new_ext(home_md, Options::all());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
});

static CSS_CONTENT: LazyLock<&str> = LazyLock::new(|| {
    STATIC_DIR
        .get_file("style.css")
        .expect("style.css not found")
        .contents_utf8()
        .expect("style.css is not valid UTF-8")
});

async fn serve_css() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css")
        .body(CSS_CONTENT.to_string())
        .unwrap()
}

async fn serve_home() -> Result<Html<String>, (StatusCode, String)> {
    let data = json!({
        "body": *MARKDOWN_CONTENT
    });

    match HANDLEBARS.render("main", &data) {
        Ok(rendered) => Ok(Html(rendered)),
        Err(e) => {
            warn!("Template rendering error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            ))
        }
    }
}

fn create_app() -> Router {
    Router::new()
        .route("/", get(serve_home))
        .route("/style.css", get(serve_css))
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let app = create_app();

    // ACME configuration for automatic HTTPS
    let config = AcmeConfig::new(["matthewblair.net"])
        .contact(["mailto:me@matthewblair.net"])
        .cache_option(Some(DirCache::new("/var/cache/acme")))
        .directory_lets_encrypt(true);

    let state = config.state();
    let acceptor = state.axum_acceptor(state.default_rustls_config());

    // Start HTTP server for ACME challenges and redirects
    tokio::spawn(async move {
        let redirect_app = Router::new().fallback(|request: Request| async move {
            let host = request
                .headers()
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("matthewblair.net");

            let uri = format!(
                "https://{}{}",
                host,
                request
                    .uri()
                    .path_and_query()
                    .map(|pq| pq.as_str())
                    .unwrap_or("/")
            );

            Response::builder()
                .status(StatusCode::MOVED_PERMANENTLY)
                .header(header::LOCATION, uri)
                .body(String::new())
                .unwrap()
        });

        let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
        info!("HTTP server listening on port 80 for redirects and ACME challenges");

        axum::serve(listener, redirect_app).await.unwrap();
    });

    // Start HTTPS server using axum-server
    info!("Starting HTTPS server on port 443");
    info!("Visit https://matthewblair.net");

    axum_server::bind("0.0.0.0:443".parse()?)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_home_page() {
        let app = create_app();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let text = response.text();
        assert!(text.contains("Matt Blair"));
        assert!(text.contains("Google"));
    }

    #[tokio::test]
    async fn test_css_endpoint() {
        let app = create_app();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/style.css").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.header("content-type"), "text/css");

        let text = response.text();
        assert!(text.contains("Inter"));
        assert!(text.contains("background"));
    }
}
