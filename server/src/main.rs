
use game::{Game, GameListing, GameSettings, PlayerInfo, PlayerRequest, PlayerRole};

#[macro_use] extern crate rocket;
use rocket_db_pools::{sqlx::{self}, Connection, Database};
use rocket::fairing::{self, Fairing, AdHoc, Info, Kind};
use rocket::{Rocket, Build};
use rocket::http::Status;
use rocket::serde::json::Json;

// CORS
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, request: &'r rocket::Request<'_>, response: &mut rocket::Response<'r>) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "GET, POST, OPTIONS"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "Content-Type"));
        response.set_header(rocket::http::Header::new("Access-Control-Request-Method", "GET, POST, OPTIONS"));

        // Handle preflight OPTIONS requests
        // COuld perhaps do this with a wildcard options matcher
        if request.method() == rocket::http::Method::Options {
            let body = "";
            response.set_header(rocket::http::ContentType::Plain);
            response.set_sized_body(body.len(), std::io::Cursor::new(body));
            response.set_status(Status::Ok);
        }
    }
}


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
#[get("/games")]
async fn serve_games(mut db: Connection<GamesDB>) -> (Status, rocket::serde::json::Value) {
    let result = sqlx::query_as::<_, (i64, String)>("SELECT id, state FROM games")
        .fetch_all(&mut **db)
        .await;

    match result {
        Ok(v) => {
            let listings: Vec<GameListing> = v.into_iter().map(|s| {
                let game = serde_json::from_str::<Game>(&s.1).unwrap();
                GameListing {   id: s.0, 
                                name: game.settings.name.clone(),
                                available_roles: game.get_available_roles(),
                            }
            }).collect();
            (Status::Ok, serde_json::json!(listings))
        }
        Err(_) => (Status::BadRequest, serde_json::json!(None::<GameListing>))
    }
}

#[get("/gameweek/<id>")]
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

#[get("/gamestate/<id>")]
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

#[post("/joingame/<id>", format="application/json", data="<pi>")]
async fn join_game(mut db: Connection<GamesDB>, id: i64, pi: Json<PlayerInfo>) -> (Status, rocket::serde::json::Value) {
    let pi = pi.into_inner();
    // Check to see whether that role is still available
    // Not contention safe, but little of this is without breaking everything down into the DB
    // Fetch the game, check the players, insert if available, then update state and send it to the client
    // Fetch game
    let result = sqlx::query_as::<_, (String,)>("SELECT state FROM games WHERE id = $1")
    .bind(id)
    .fetch_one(&mut **db)
    .await;

    match &result {
        Ok(_) => (),
        Err(_) => return (Status::BadRequest, serde_json::json!(None::<Game>))
    }
    let mut game = serde_json::from_str::<Game>(&result.unwrap().0).unwrap();

    // Check the existing player roles
    if game.settings.players.get(&pi.role).unwrap().is_none() {
        // Insert player
        // Since the hashmap support simply not having an entry for a given key, the Option<String> in there is very overkill
        game.settings.players.insert(pi.role, Some(pi.name));
    } else {
        return (Status::BadRequest, serde_json::json!(None::<Game>))
    }

    // Update state in DB
    sqlx::query("REPLACE INTO games (id, state) VALUES ($1, $2)")
    .bind(id)
    .bind(serde_json::to_string(&game).unwrap())
    .execute(&mut **db)
    .await.ok().unwrap();

    (Status::Ok, serde_json::json!(game))
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
        .attach(CORS)
        .mount("/", routes![serve_games,
                            serve_gameweek,
                            serve_gamestate,
                            create_game,
                            join_game,
                            receive_request])
}
