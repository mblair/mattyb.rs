// src/main.rs

use actix_files::NamedFile;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use pulldown_cmark::{html, Options, Parser};
use rustls_acme::{AcmeConfig, AcmeAcceptor, caches::DirCache};
use std::{fs, io};
use tera::{Context, Tera};

struct AppState {
    tera: Tera,
    md: String,
    css: String,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Load and parse template.html
    let mut tera = Tera::default();
    tera.add_template_file("template.html", Some("page"))
        .expect("Failed to load template.html");

    // Read Markdown and CSS at startup
    let md = fs::read_to_string("home.md").expect("Could not read home.md");
    let css = fs::read_to_string("style.css").expect("Could not read style.css");

    let state = web::Data::new(AppState { tera, md, css });

    // Configure ACME (Let's Encrypt)
    let cache_dir = "/var/cache/acme/";
    let domains = vec!["matthewblair.net".to_string()];
    let contact = vec!["mailto:me@matthewblair.net".to_string()];

    let config = AcmeConfig::new(domains)
        .contact(contact)
        .cache(DirCache::new(cache_dir))
        .directory_lets_encrypt(true);

    let acme_state = config.state();
    let acceptor = acme_state.acceptor();

    // Extract the acceptor for the HTTP-01 challenge server
    let challenger = acceptor.challenger();

    // HTTP on port 80: handles ACME challenges and redirects to HTTPS
    let http = HttpServer::new(move || {
        App::new()
            .wrap(challenger.clone()) // ACME HTTP-01 challenge handler
            .app_data(state.clone())
            .service(css_handler)
            .service(index)
    })
    .bind(("0.0.0.0", 80))?
    .run();

    // HTTPS on port 443 with TLS from ACME
    let https = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(css_handler)
            .service(index)
    })
    .bind_rustls_with_acceptor(("0.0.0.0", 443), acceptor)?
    .run();

    // Run both servers concurrently
    futures::try_join!(http, https)?;
    Ok(())
}

/// Serve style.css
#[get("/style.css")]
async fn css_handler(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/css")
        .body(state.css.clone())
}

/// Catch-all for everything else: render Markdown via Tera
#[get("/{_:.*}")]
async fn index(state: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    // Convert Markdown to HTML
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(&state.md, opts);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    // Render into template.html
    let mut ctx = Context::new();
    ctx.insert("Body", &html_body);
    let rendered = state
        .tera
        .render("page", &ctx)
        .unwrap_or_else(|e| format!("Template error: {}", e));

    HttpResponse::Ok().body(rendered)
}
