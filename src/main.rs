use axum::{routing::get, Router};
use clap::Parser;
use rustls_acme::caches::DirCache;
use rustls_acme::AcmeConfig;
use std::net::{Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use tokio_stream::StreamExt;

#[derive(Parser, Debug)]
struct Args {
    /// Domains
    #[clap(short, required = true)]
    domains: Vec<String>,

    /// Contact info
    #[clap(short)]
    email: Vec<String>,

    /// Cache directory
    #[clap(short, value_parser)]
    cache: Option<PathBuf>,

    /// Use Let's Encrypt production environment
    /// (see https://letsencrypt.org/docs/staging-environment/)
    /// Default is false, aka staging
    #[clap(long)]
    prod: bool,

    #[clap(short, long, default_value = "443")]
    port: u16,
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let args = Args::parse();

    println!("{:?}", args);

    let mut state = AcmeConfig::new(args.domains)
        .contact(args.email.iter().map(|e| format!("mailto:{}", e)))
        .cache_option(args.cache.clone().map(DirCache::new))
        .directory_lets_encrypt(args.prod)
        .state();
    let acceptor = state.axum_acceptor(state.default_rustls_config());

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => log::info!("event: {:?}", ok),
                Err(err) => log::error!("error: {:?}", err),
            }
        }
    });

    let app = Router::new().route("/", get(|| async { "Hello world!" }));

    let addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, args.port));
    axum_server::bind(addr)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
