use actix_web::{web, App, HttpRequest, HttpServer, Responder};

const SERVER: &str = "127.0.0.1:8000";

async fn compare_hands(req: HttpRequest) -> impl Responder {
    println!("compare_hands()");
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

async fn get_cpu_action(req: HttpRequest) -> impl Responder {
    let query = req.query_string();
    let query = qstring::QString::from(query);
    let cpu_cards = query.get("cpuCards").unwrap();
    let board = query.get("board").unwrap();
    let history = query.get("history").unwrap();
    println!("cpu: {}", cpu_cards);
    println!("board: {}", board);
    println!("history: {}", history);
    format!("unimplemented")
}

#[actix_rt::main]
pub async fn main() -> std::io::Result<()> {
    println!("[INFO] Launching server at {}", SERVER);
    HttpServer::new(|| {
        App::new()
            .route("/compare", web::get().to(compare_hands))
            .route("/bot", web::get().to(get_cpu_action))
    })
    .bind(SERVER)?
    .run()
    .await
}
