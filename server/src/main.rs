
use server::game::{GameSettings, GameState, PlayerRequest, PlayerRole, PlayerState};

#[macro_use] extern crate rocket;
use rocket_db_pools::{sqlx::{self}, Connection, Database};
use rocket::fairing::{self, AdHoc};
use rocket::{Rocket, Build};
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
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    settings    TEXT
                )"
            ).execute(dbi).await.unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS states (
                    id      INTEGER PRIMARY KEY AUTOINCREMENT,
                    state   TEXT,
                    game_id INTEGER,
                    FOREIGN KEY(game_id) REFERENCES games(id)
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

fn configure_game(settings: &GameSettings) -> GameState {
    GameState {
        week: 0,
        game_end: false,
        players: [PlayerState {
            stock: settings.initial_request,
            deficit: 0,
            incoming: settings.initial_request,
            outgoing: settings.initial_request,
            incoming_request: settings.initial_request,
            outgoing_request: None,
            costs: 0,
        }; 4],
        production: settings.initial_request,
    }
}

// Requests
#[get("/gamestate/<id>", format="application/json")]
async fn serve_gamestate(mut db: Connection<GamesDB>, id: i64) -> String{
    let result: (String,) = sqlx::query_as("SELECT state FROM states WHERE game_id = ? ORDER BY id ASC LIMIT 1")
        .bind(id)
        .fetch_one(&mut **db)
        .await.ok().unwrap();
    let response = result.0;
    println!("Query result {:}", response);
    response
}

#[post("/creategame", format="application/json", data="<gs>")]
async fn create_game(mut db: Connection<GamesDB>, gs: Json<GameSettings>) {
    let gs = gs.into_inner();   // Unsure how to pass the JSON through so we de and reserialize
    let result: (String,) = sqlx::query_as("INSERT INTO games (settings) VALUES (?) RETURNING id")
        .bind(serde_json::to_string(&gs).unwrap())
        .fetch_one(&mut **db)
        .await.ok().unwrap();

    // let game_state = configure_game(&gs);
    // sqlx::query("INSERT INTO states (state) VALUES (?)")
    //     .bind(serde_json::to_string(&game_state).unwrap())
    //     .execute(&mut **db)
    //     .await.ok().unwrap();

    println!("New game {:?} created with id: {:?}", &gs.name, result.0);
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
