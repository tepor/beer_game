
use server::game::{GameSettings, GameState, PlayerRequest, PlayerRole, PlayerState, Game};

#[macro_use] extern crate rocket;
use rocket_db_pools::{sqlx::{self}, Connection, Database};
use rocket::fairing::{self, AdHoc};
use rocket::{Rocket, Build};
use rocket::http::Status;
use rocket::serde::json::Json;

// Database tools
#[derive(Database)]
#[database("sqlite_games")]
struct GamesDB(sqlx::SqlitePool);

async fn configure_db(rocket: Rocket<Build>) -> fairing::Result {
    if let Some(db) = GamesDB::fetch(&rocket) {
        // Get the inner type
        let dbi = &db.0;
        
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS games (
                    id    INTEGER PRIMARY KEY AUTOINCREMENT,
                    state TEXT
                )"
            ).execute(dbi).await.unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS requests (
                    id      INTEGER PRIMARY KEY AUTOINCREMENT,
                    amount  INTEGER,
                    game_id INTEGER,
                    FOREIGN KEY(game_id) REFERENCES games(id)
                )"
            ).execute(dbi).await.unwrap();

        println!("Games database configured");
        Ok(rocket)
    } else {
        Err(rocket)
    }
}


// Requests
#[get("/gamestate/<id>", format="application/json")]
async fn serve_gamestate(mut db: Connection<GamesDB>, id: i64) -> (Status, rocket::serde::json::Value) {
    let result = sqlx::query_as::<_, (String,)>("SELECT state FROM games WHERE id = $1 ORDER BY id ASC LIMIT 1")
        .bind(id)
        .fetch_one(&mut **db)
        .await;

    match result {
        Ok(v) => (Status::Ok, serde_json::json!(serde_json::from_str::<Game>(&v.0).unwrap())),
        Err(_) =>           (Status::BadRequest, serde_json::json!(None::<Game>))
    }
}

#[post("/creategame", format="application/json", data="<gs>")]
async fn create_game(mut db: Connection<GamesDB>, gs: Json<GameSettings>) -> (Status, rocket::serde::json::Value) {
    // Create a new game with the incoming settings
    let gs = gs.into_inner();
    let game = Game::new(gs);   

    // Insert game into DB and get the row ID
    let result = sqlx::query_as::<_, (i64,)>("INSERT INTO games (state) VALUES (?) RETURNING id")
        .bind(serde_json::to_string(&game).unwrap())
        .fetch_one(&mut **db)
        .await;

    match result {
        Ok(v) => {
            println!("New game {:?} created with id: {:?}", &game.settings.name, v.0);
            (Status::Created, serde_json::json!(Some(v.0)))
        },
        Err(e) => {
            println!("Failed to create new game due to error: {:?}", e.to_string());
            (Status::BadRequest, serde_json::json!(None::<i64>))
        }
    }
}

#[post("/submitrequest", format="application/json", data="<pr>")]
async fn receive_request(mut db: Connection<GamesDB>, pr: Json<PlayerRequest>) {
    println!("Player {:?} requested {:?}", pr.role, pr.request);
    // Confused why a ? instead of .ok etc. doesnt work here. Also how to do the type specification manually
    let result: (String,) = sqlx::query_as("SELECT name FROM games")
        .fetch_one(&mut **db)
        .await.ok().unwrap();
    println!("Query result {:}", result.0);
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(GamesDB::init())
        .attach(AdHoc::try_on_ignite("DB Configuration", configure_db))
        .mount("/", routes![create_game, serve_gamestate, receive_request])
}
