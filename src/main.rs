use anyhow::{Context, Result, bail};
use axum::{Router, routing::get};
use clap::Parser;
use rustls_acme::AcmeConfig;
use rustls_acme::caches::DirCache;
use std::net::{IpAddr, SocketAddr};
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

    #[clap(long, default_value = "0.0.0.0")]
    bind: IpAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info).context("failed to initialize logger")?;
    let args = Args::parse();

    let mut state = AcmeConfig::new(args.domains)
        .contact(args.email.iter().map(|e| format!("mailto:{}", e)))
        .cache_option(args.cache.clone().map(DirCache::new))
        .directory_lets_encrypt(args.prod)
        .state();
    let acceptor = state.axum_acceptor(state.default_rustls_config());

    let app = Router::new().route("/", get(|| async { "Hello world!" }));

    let addr = SocketAddr::from((args.bind, args.port));
    let server = axum_server::bind(addr)
        .acceptor(acceptor)
        .serve(app.into_make_service());

    tokio::pin!(server);

    loop {
        tokio::select! {
            acme_event = state.next() => match acme_event {
                Some(Ok(event)) => log::info!("acme event: {:?}", event),
                Some(Err(err)) => log::error!("acme error: {:?}", err),
                None => bail!("acme state stream ended unexpectedly"),
            },
            server_result = &mut server => {
                server_result.context("axum server exited unexpectedly")?;
                break;
            }
        }
    }

    Ok(())
}
