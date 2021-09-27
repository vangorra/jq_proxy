use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::exit;
use std::str;

use actix_web::{App, client::Client, HttpRequest, HttpResponse, HttpServer, middleware::Logger, web};
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::Method;
use actix_web::web::Bytes;
use chrono::Local;
use clap;
use clap::{AppSettings, Clap};
use env_logger::Builder;
use jq_rs;
use log::LevelFilter;
use serde::Deserialize;
use serde::Serialize;
use serde_yaml;

#[derive(Serialize)]
struct ErrorResponse {
    is_error: bool,
    message: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct PathConfig {
    source_url: String,
    jq_filter: String,
}

fn default_listen() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_paths() -> HashMap<String, PathConfig> {
    HashMap::new()
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct AppConfig {
    #[serde(default = "default_listen")]
    listen: String,

    #[serde(default = "default_paths")]
    paths: HashMap<String, PathConfig>,
}

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct CliArgs {
    #[clap(short, long)]
    config_file_path: String,
}

async fn proxy(req: HttpRequest, body: Bytes, data: web::Data<AppConfig>) -> HttpResponse {
    // Path will exist as the only paths configured are based on the app context data.
    let path_config = data.paths.get(req.path())
        .unwrap()
        .clone();

    // Request from the target.
    let mut client_builder = Client::default()
        .request(
            Method::from_bytes(req.method().to_string().as_str().as_bytes()).unwrap(),
            path_config.source_url
        );

    // Proxy the request headers.
    for (key, value) in req.headers() {
        client_builder = client_builder.header(
            key,
            value.to_str().unwrap()
        );
    }

    // Proxy the request body.
    let response_result = match body.is_empty() {
        true => client_builder.send().await,
        false => client_builder.send_body(body).await,
    };

    let mut response = response_result.unwrap();
    let body_string = String::from_utf8(response.body().await.unwrap().to_vec()).unwrap();


    // Query the JSON with jq.
    let jq_result = jq_rs::run(&path_config.jq_filter, &body_string)
        .map_err(|e| format!("Failed to run jq: {}", e));

    if jq_result.is_err() {
        return HttpResponse::InternalServerError()
            .json(ErrorResponse {
                is_error: true,
                message: jq_result.unwrap_err().to_string(),
            });
    }


    // Proxy the response.
    let mut builder = HttpResponseBuilder::new(response.status());
    builder.content_type("application/json");

    // Proxy the response headers.
    for (key, value) in response.headers() {
        builder.header(
            key,
            value.to_str().unwrap()
        );
    }

    // Proxy the response body.
    return builder.body(jq_result.unwrap());
}

fn parse_config(config_file_path: String) -> Result<AppConfig, String> {
    log::debug!("Reading config file {}.", config_file_path);
    let config_file_contents = fs::read_to_string(config_file_path)
        .map_err(|e| format!("Config read error: {}", e))?;

    log::debug!("Parse config file.");
    let config: AppConfig = serde_yaml::from_str(&config_file_contents)
        .map_err(|e| format!("Config parse error: {}", e))?;

    return match config.paths.is_empty() {
        true => Err("No paths configured in config file.".to_string()),
        false => Ok(config),
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configure logging output.
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] - {}",
                     Local::now().format("%Y-%m-%dT%H:%M:%S"),
                     record.level(),
                     record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    log::debug!("Parsing CLI args.");
    let cli_args: CliArgs = CliArgs::parse();

    let config_result = parse_config(cli_args.config_file_path);
    if config_result.is_err() {
        eprintln!("{}", config_result.unwrap_err());
        exit(1);
    }

    let config = config_result.unwrap();

    println!("Configured paths:");
    (&config.paths).into_iter().for_each(|item| {
        println!("  {} -> {}", item.0, item.1.source_url);
    });

    let moved_config = config.clone();
    HttpServer
        ::new(move || {
            let mut app = App::new()
                .wrap(Logger::new("%r"))
                .app_data(web::Data::new(moved_config.clone()));

            // Configure the paths.
            for (path, _path_config) in &(moved_config.paths) {
                app = app.service(web::resource(path).route(
                    web::get().to(proxy)
                ));
            }

            return app;
        })
        .bind(config.listen)?
        .run()
        .await
}
