mod web;

use std::net::IpAddr;
use std::str::FromStr;
use actix_middleware_etag::Etag;
use actix_web::web::Data;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    {
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let registry = tracing_subscriber::registry();
        #[cfg(tokio_unstable)]
        let registry = registry.with(console_subscriber::spawn());
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
            )
            .init();
        log::info!("Initialized logging");
    }

    async_main()
}

#[actix_web::main]
async fn async_main() -> anyhow::Result<()> {
    let ip = IpAddr::from_str(&dotenvy::var("BIND_IP")?)?;
    let port = u16::from_str(&dotenvy::var("BIND_PORT")?)?;

    let options = sqlx::postgres::PgConnectOptions::new();
    let pool:sqlx::postgres::PgPool = sqlx::Pool::connect_with(options).await?;
    log::info!("Connected to postgres");


    let server = actix_web::HttpServer::new(move || {
        let app = actix_web::App::new();

        app
            .app_data(Data::new(pool.clone()))
            .wrap(actix_web::middleware::Logger::default())
            .wrap(Etag{force_strong_etag: true})
            .service(web::arc::selector_map)
            .service(web::arc::signing_table)
            .service(web::arc::key_table)
    });

    server.bind_auto_h2c((ip, port))?
        .run()
        .await?;

    Ok(())
}
