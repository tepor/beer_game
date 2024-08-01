
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
                    state TEXT NOT NULL
                )"
            ).execute(dbi).await.unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS requests (
                    game_id INTEGER NOT NULL,
                    week    INTEGER NOT NULL,
                    role    INTEGER NOT NULL,
                    amount  INTEGER NOT NULL,
                    FOREIGN KEY (game_id) REFERENCES games (id),
                    PRIMARY KEY (game_id, week, role)
                )"
            ).execute(dbi).await.unwrap();

        println!("Games database configured");
        Ok(rocket)
    } else {
        Err(rocket)
    }
}


// Requests
#[get("/games", format="application/json")]
async fn serve_games(mut db: Connection<GamesDB>) -> (Status, rocket::serde::json::Value) {
    let result = sqlx::query_as::<_, (i64, String)>("SELECT id, state FROM games")
        .fetch_all(&mut **db)
        .await;

    match result {
        Ok(v) => {
            let names: Vec<(i64, String)> = v.into_iter().map(|s| {
                let game = serde_json::from_str::<Game>(&s.1).unwrap();
                (s.0, game.settings.name)
            }).collect();
            (Status::Ok, serde_json::json!(names))
        }
        Err(_) => (Status::BadRequest, serde_json::json!(None::<String>))
    }
}

#[get("/gameweek/<id>", format="application/json")]
async fn serve_gameweek(mut db: Connection<GamesDB>, id: i64) -> (Status, rocket::serde::json::Value) {
    let result = sqlx::query_as::<_, (String,)>("SELECT state FROM games WHERE id = $1")
        .bind(id)
        .fetch_one(&mut **db)
        .await;

    match result {
        Ok(v) => {
            let game = serde_json::from_str::<Game>(&v.0).unwrap();
            let week = game.get_current_week();
            (Status::Ok, serde_json::json!(week))
        },
        Err(_) => (Status::BadRequest, serde_json::json!(None::<u32>))
    }
}

#[get("/gamestate/<id>", format="application/json")]
async fn serve_gamestate(mut db: Connection<GamesDB>, id: i64) -> (Status, rocket::serde::json::Value) {
    let result = sqlx::query_as::<_, (String,)>("SELECT state FROM games WHERE id = $1")
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
    let result = sqlx::query_as::<_, (i64,)>("INSERT INTO games (state) VALUES ($1) RETURNING id")
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
async fn receive_request(mut db: Connection<GamesDB>, pr: Json<PlayerRequest>) -> Status {
    // Submit the request to the DB
    println!("Player {:?} requested {:?} in game {:?}", pr.role, pr.amount, pr.game_id);
    let result = sqlx::query("INSERT OR IGNORE INTO requests (game_id, week, role, amount) VALUES ($1, $2, $3, $4)")
        .bind(pr.game_id)
        .bind(pr.week)
        .bind(pr.role as u32)
        .bind(pr.amount)
        .execute(&mut **db)
        .await;

    match result {
        Ok(v) => println!("{:?}", v),
        Err(e) => { 
            println!("Query failed: {:?}", e);
            return Status::BadRequest
        }
    }

    // Check to see if all requests are ready then step the game forward
    // Get the current game week
    let result = sqlx::query_as::<_, (String,)>("SELECT state FROM games WHERE id = $1")
    .bind(pr.game_id)
    .fetch_one(&mut **db)
    .await.ok().unwrap().0;

    let mut game: Game = serde_json::from_str(&result).unwrap();
    let week = game.get_current_week();

    // Check we have all four requests for this week
    let requests = sqlx::query_as::<_, (u32, u32)>("SELECT role, amount FROM requests WHERE game_id = $1 AND week = $2")
        .bind(pr.game_id)
        .bind(week)
        .fetch_all(&mut **db)
        .await.ok().unwrap();

    if requests.len() == 4 {
        // Plug in the request values
        for (role, amount) in requests {
            let role: PlayerRole = role.try_into().unwrap();
            game.states.last_mut().unwrap().players[role].outgoing_request = Some(amount);
        }

        // Step the game forward
        game.take_turn();

        // Update the game state
        sqlx::query("REPLACE INTO games (id, state) VALUES ($1, $2)")
        .bind(pr.game_id)
        .bind(serde_json::to_string(&game).unwrap())
        .execute(&mut **db)
        .await.ok().unwrap();

        println!("Game {:?} took a step to week {:?}", pr.game_id, week + 1);
    }

    Status::Ok
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(GamesDB::init())
        .attach(AdHoc::try_on_ignite("DB Configuration", configure_db))
        .mount("/", routes![serve_games,
                            serve_gameweek,
                            serve_gamestate,
                            create_game,
                            receive_request])
}
