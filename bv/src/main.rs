use arc_swap::ArcSwap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::{
    extract::Query,
    routing::{get, post},
    Router,
};
use bv_templates::{ContentType, Item};
use envconfig::Envconfig;
use futures::StreamExt;
use log::{debug, info};
use once_cell::sync::Lazy;
use opendal::EntryMode;
use serde::Deserialize;
use std::collections::BTreeMap;
use tokio::signal;
use tower_http::trace::{self, TraceLayer};
use tracing_subscriber::EnvFilter;

static CACHE: Lazy<ArcSwap<BTreeMap<String, Item>>> =
    Lazy::new(|| ArcSwap::from_pointee(BTreeMap::new()));

static CONFIG: Lazy<ArcSwap<Config>> =
    Lazy::new(|| ArcSwap::from_pointee(Config::init_from_env().unwrap()));

async fn reload_cache() {
    let config = CONFIG.load();

    let mut cache = BTreeMap::new();

    for bucket in config.buckets.split(',') {
        let client = opendal::raw::HttpClient::with(reqwest::Client::new());
        let op = opendal::services::S3::default()
            .root("/")
            .bucket(bucket)
            .endpoint(&config.endpoint)
            .http_client(client)
            .access_key_id(&config.access_key_id)
            .secret_access_key(&config.secret_access_key)
            .region(&config.region);

        let dal = opendal::Operator::new(op)
            .expect("successfully replace OpenDAL bucket")
            .finish();
        let lister = dal.lister_with("");

        let items: Vec<Item> = lister
            .recursive(true)
            .await
            .expect("failure")
            .filter_map(move |e| async {
                debug!("{:?}", e);
                let Ok(e) = e else { return None };
                match e.metadata().mode() {
                    EntryMode::FILE => Some(e.into_parts()),
                    _ => None,
                }
            })
            .filter_map(|(path, _metadata)| async {
                Some(Item {
                    link: format!("{}/{}/{}", config.endpoint, bucket, path),
                    path: path.clone(),
                    content_type: {
                        match mime_guess::from_path(path).first() {
                            Some(mime) => match mime.type_().as_str() {
                                "image" => ContentType::Image,
                                "video" => ContentType::Video,
                                _ => return None,
                            },
                            None => return None,
                        }
                    },
                })
            })
            .collect()
            .await;
        info!("{} items found in bucket {}", items.len(), bucket);
        for item in items.clone() {
            cache.insert(format!("{}/{}", bucket, item.path), item.clone());
        }
    }
    CACHE.store(std::sync::Arc::new(cache));
}

async fn reindex() -> impl IntoResponse {
    reload_cache().await;
    StatusCode::OK
}
#[derive(Envconfig, Clone, Debug)]
struct Config {
    #[envconfig(from = "PAGE_SIZE")]
    page_size: u8,
    #[envconfig(from = "ADDR")]
    addr: String,
    #[envconfig(from = "PORT")]
    port: u64,
    #[allow(dead_code)]
    #[envconfig(from = "BUCKETS")]
    buckets: String,
    #[envconfig(from = "REGION")]
    region: String,
    #[envconfig(from = "ENDPOINT")]
    endpoint: String,
    #[envconfig(from = "ACCESS_KEY_ID")]
    access_key_id: String,
    #[envconfig(from = "SECRET_ACCESS_KEY")]
    secret_access_key: String,
}

#[derive(
    serde::Serialize, Clone, Deserialize, Debug, Default, PartialEq, PartialOrd, Ord, Eq, Hash,
)]
struct QueryQuery {
    bucket: String,
    prefix: String,
    offset: Option<u64>,
}

async fn style_include() -> impl IntoResponse {
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(axum::body::Body::from(include_str!("./static/style.css")))
        .unwrap()
}

#[axum::debug_handler]
async fn query(Query(q): Query<QueryQuery>) -> impl IntoResponse {
    let config = CONFIG.load();
    info!("query {:?} {:?}", q, config);
    let offset = q.offset.unwrap_or_else(|| 0);

    if !config.buckets.contains(&q.bucket) {
        CONFIG.store(
            Config {
                page_size: config.page_size,
                buckets: format!("{},{}", config.buckets, q.bucket),
                addr: config.addr.clone(),
                port: config.port,
                region: config.region.clone(),
                endpoint: config.endpoint.clone(),
                access_key_id: config.access_key_id.clone(),
                secret_access_key: config.secret_access_key.clone(),
            }
            .into(),
        );
        reload_cache().await;
    };

    let items = {
        let cache = CACHE.load();
        let prefix_str = format!("{}/{}", q.bucket, q.prefix);
        let result: Vec<_> = cache
            .range((
                std::ops::Bound::Included(prefix_str.clone()),
                match prefix_range::upper_bound_from_prefix(&prefix_str) {
                    Some(bound) => std::ops::Bound::Excluded(bound),
                    None => std::ops::Bound::Unbounded,
                },
            ))
            .skip(offset as usize)
            .take(config.page_size as usize)
            .map(|(_k, v)| v.clone())
            .collect();
        info!("results for {}/{}: {}", q.bucket, q.prefix, result.len());
        result
    };
    debug!("{:?}", items);
    let view = bv_templates::Browse {
        title: format!("{}/{}", q.bucket, q.prefix),
        bucket: q.bucket,
        prefix_str: q.prefix,
        prev_offset: u64::saturating_sub(offset, config.page_size as u64),
        offset: if items.len() < config.page_size as usize {
            offset
        } else {
            offset + config.page_size as u64
        },
        items,
    };
    axum::response::Html(view.to_string())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .init();

    tokio::spawn(async {
        reload_cache().await;
    });

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(tracing::Level::DEBUG))
        .on_response(trace::DefaultOnResponse::new().level(tracing::Level::DEBUG));
    let app = Router::new()
        .route(
            "/",
            get(|| async { Redirect::temporary("/browse?bucket=ai-art&prefix=") }),
        )
        .route("/browse", get(query))
        .route("/browse", post(query))
        .route("/reindex", get(reindex))
        .route("/static/style.css", get(style_include))
        // .nest_service("/static", ServeDir::new("static"))
        .layer(trace_layer);

    let config = CONFIG.load();
    println!("{:?}", config);
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.addr, config.port))
        .await
        .ok()
        .unwrap_or_else(|| panic!("binding {}:{}", config.addr, config.port));
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("application started")
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("install SIGINT handler") };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
