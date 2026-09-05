use starter_server::{
    app::{config::Config, router::router},
    platform::{db::Database, mail::Mailer},
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::new("starter_server=info"))
        .init();
    let config = Config::from_env()?;
    let database = Database::new(&config)?;
    match std::env::args().nth(1).as_deref().unwrap_or("serve") {
        "migrate" => {
            database.migrate().await?;
            println!("Migrations verified and applied.");
        }
        "check" => {
            database.ready().await?;
            println!("Database ready.");
        }
        "prune" => {
            database.ready().await?;
            database.prune().await?;
            println!("Expired authentication records removed.");
        }
        "serve" => {
            database.ready().await?;
            let listener = tokio::net::TcpListener::bind(config.bind).await?;
            let (mailer, mut mail_task) = Mailer::start(&config)?;
            let app = router(config, database.clone(), mailer);
            tracing::info!(event = "server_started");
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown())
                .await?;
            if tokio::time::timeout(Duration::from_secs(10), &mut mail_task)
                .await
                .is_err()
            {
                mail_task.abort();
            }
        }
        _ => return Err("usage: starter-server [serve|migrate|check|prune]".into()),
    }
    database.close();
    Ok(())
}
async fn shutdown() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {_ = interrupt=>{},_ = terminate=>{}}
}
