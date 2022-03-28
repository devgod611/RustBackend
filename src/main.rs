#[macro_use]
extern crate actix_web;
extern crate clap;

const APPNAME: &'static str = "flights";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEF_CFG_FN: &'static str = "cfg-flights.json";
const DEF_DB_NAME: &'static str = "db";
const DEF_DB_DIR: &'static str = "db.kv";
const DEF_BIND_ADDR: &'static str = "127.0.0.1";
const DEF_BIND_PORT: &'static str = "8080";

use std::sync::{Arc, Mutex};
use std::{env, fs, io};
use std::str;

use actix_web::http::StatusCode;
use actix_web::{guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::{ConfigBuilder, Db};


#[derive(Serialize, Deserialize)]
struct DbConfig {
    name: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct ServerConfig {
    databases: Vec<DbConfig>,
}

struct ServerState {
    name: String, // db nickname
    db: Db,       // open db handle
}

// helper function, 404 not found
fn err_not_found() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::NOT_FOUND)
        .content_type("application/json")
        .body(
            json!({
          "error": {
             "code" : -404,
              "message": "not found"}})
            .to_string(),
        ))
}

// helper function, server error
fn err_500() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
        .content_type("application/json")
        .body(
            json!({
          "error": {
             "code" : -500,
              "message": "internal server error"}})
            .to_string(),
        ))
}

// helper function, success + binary response
fn ok_binary(val: Vec<u8>) -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/octet-stream")
        .body(val))
}

// helper function, success + json response
fn ok_json(jval: serde_json::Value) -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body(jval.to_string()))
}

/// PUT data item.  key and value both in URI path.
fn req_put(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    req: HttpRequest,
    (path, body): (web::Path<(String, String)>, web::Bytes),
) -> Result<HttpResponse> {
    let state = m_state.lock().unwrap();

    // we only support 1 db, for now...  user must specify db name
    if state.name != path.0 {
        return err_not_found();
    }
    let mut _body = body;
    let req_str = match str::from_utf8(&_body) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };

    let req_str_len = req_str.len();
    let mut flights: Vec<(String, String)> = vec![];
    let mut word_cnt = 0u128;
    let mut key: String = String::new();
    let mut value: String = String::new();
    let mut kv: String = String::new();
    let mut prev = 0usize;
    for mut i in 0 .. (req_str_len - 1usize) {
        let mut b: u8 = req_str.as_bytes()[i];
        let mut c: char = b as char;
        if c == '\'' && prev != i {
            for j in i + 1 .. req_str_len - 1usize {
                b = req_str.as_bytes()[j];
                c = b as char;
                if c != '\'' {
                    let mut _c = c;
                    kv.push_str(&_c.to_string());
                }
                else {
                    prev = j;
                    word_cnt = word_cnt + 1u128;
                    break;
                }
            }
        }
        
        if !kv.eq(&String::new()) {
            if word_cnt == 1u128 {
                key = kv.clone();
            } else if word_cnt == 2u128 {
                value = kv.clone();
                word_cnt = 0;
                flights.push((key.clone(), value.clone()));
                key = String::new();
                value = String::new();
            }
            kv = String::new();
        }
    }
    let mut i: usize = 0usize;
    loop {
        if i >= flights.len() {
            break;
        }
        let (path_from_i, path_to_i) = flights[i].clone();
        let mut flag = false;
        for j in 0usize .. flights.len() {
            if i != j {
                let (path_from_j, path_to_j) = flights[j].clone();
                if path_to_i == path_from_j {
                    if i > j {
                        flights.remove(i);
                        flights.remove(j);
                    } else {
                        flights.remove(j);
                        flights.remove(i);
                    }
                    i = 0usize;
                    flag = true;
                    flights.push((path_from_i.clone(), path_to_j.clone()));
                    break;
                }
            }
        }
        if !flag {
            i = i + 1usize;
        }
    }
    ok_json(json!({"result": flights}))
}

/// 404 handler
fn p404() -> Result<HttpResponse> {
    err_not_found()
}

fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    // parse command line
    let cli_matches = clap::App::new(APPNAME)
        .version(VERSION)
        .author("Kyle Wilson")
        .about("Database server for key/value db")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("JSON-FILE")
                .help(&format!(
                    "Sets a custom configuration file (default: {})",
                    DEF_CFG_FN
                ))
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("bind-addr")
                .long("bind-addr")
                .value_name("IP-ADDRESS")
                .help(&format!(
                    "Custom server socket bind address (default: {})",
                    DEF_BIND_ADDR
                ))
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("bind-port")
                .long("bind-port")
                .value_name("PORT")
                .help(&format!(
                    "Custom server socket bind port (default: {})",
                    DEF_BIND_PORT
                ))
                .takes_value(true),
        )
        .get_matches();

    // configure based on CLI options
    let bind_addr = cli_matches.value_of("bind-addr").unwrap_or(DEF_BIND_ADDR);
    let bind_port = cli_matches.value_of("bind-port").unwrap_or(DEF_BIND_PORT);
    let bind_pair = format!("{}:{}", bind_addr, bind_port);
    let server_hdr = format!("{}/{}", APPNAME, VERSION);

    // read JSON configuration file
    let cfg_fn = cli_matches.value_of("config").unwrap_or(DEF_CFG_FN);
    let cfg_text = fs::read_to_string(cfg_fn)?;
    let server_cfg: ServerConfig = serde_json::from_str(&cfg_text)?;

    // special case, until we have multiple dbs: find first db config, use it
    let db_name;
    let db_path;
    if server_cfg.databases.len() == 0 {
        db_name = String::from(DEF_DB_NAME);
        db_path = String::from(DEF_DB_DIR);
    } else {
        db_name = server_cfg.databases[0].name.clone();
        db_path = server_cfg.databases[0].path.clone();
    }

    // configure & open db
    let db_config = ConfigBuilder::new()
        .path(db_path)
        .use_compression(false)
        .build();
    let db = Db::start(db_config).unwrap();

    let srv_state = Arc::new(Mutex::new(ServerState {
        name: db_name.clone(),
        db: db.clone(),
    }));

    // configure web server
    let sys = actix_rt::System::new(APPNAME);

    HttpServer::new(move || {
        App::new()
            // pass application state to each handler
            .data(Arc::clone(&srv_state))
            // apply default headers
            .wrap(middleware::DefaultHeaders::new().header("Server", server_hdr.to_string()))
            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // register our routes
            .service(
                web::resource("/api/{db}/{key}")
                    .route(web::put().to(req_put))
            )
            // default
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET` -- redundant?
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    .bind(bind_pair.to_string())?
    .start();

    println!("Starting http server: {}", bind_pair);
    sys.run()
}
