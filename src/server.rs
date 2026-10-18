use crate::{ai::Ai, media};
use anyhow::Result;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct App {
    pub db: PgPool,
    pub media: Arc<cap_std::fs::Dir>,
    pub ai: Ai,
    users: Arc<Vec<User>>,
    worker: String,
    gate: Arc<tokio::sync::Semaphore>,
    search_gate: Arc<tokio::sync::Semaphore>,
}
#[derive(Clone, Deserialize)]
struct User {
    name: String,
    token: String,
}
#[derive(Clone)]
struct Actor(String);
pub struct Error(StatusCode, &'static str);
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(json!({"error":{"code":self.1,"message":self.1,"request_id":Uuid::new_v4()}})),
        )
            .into_response()
    }
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        tracing::error!("database operation failed");
        Self(StatusCode::INTERNAL_SERVER_ERROR, "database_error")
    }
}
impl From<anyhow::Error> for Error {
    fn from(_: anyhow::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, "operation_failed")
    }
}
fn bad() -> Error {
    Error(StatusCode::BAD_REQUEST, "invalid_input")
}
fn conflict() -> Error {
    Error(StatusCode::CONFLICT, "conflict")
}
fn missing() -> Error {
    Error(StatusCode::NOT_FOUND, "not_found")
}
type Api<T> = std::result::Result<T, Error>;
fn hash(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
fn token_equal(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    let a = Sha256::digest(a.as_bytes());
    let b = Sha256::digest(b.as_bytes());
    bool::from(a.ct_eq(&b))
}

pub async fn start() -> Result<()> {
    let users: Vec<User> = serde_json::from_str(&std::env::var("ARCHIVIST_USERS")?)?;
    let worker = std::env::var("ARCHIVIST_WORKER_TOKEN")?;
    anyhow::ensure!(
        !users.is_empty() && worker.len() >= 32,
        "invalid auth config"
    );
    let mut tokens = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for u in &users {
        anyhow::ensure!(
            !u.name.is_empty()
                && u.token.len() >= 32
                && u.token != worker
                && tokens.insert(&u.token)
                && names.insert(&u.name),
            "invalid auth config"
        );
    }
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    sqlx::migrate!().run(&db).await?;
    let state = App {
        db,
        media: Arc::new(media::root(&std::env::var("MEDIA_ROOT")?)?),
        ai: Ai::from_env()?,
        users: Arc::new(users),
        worker,
        gate: Arc::new(tokio::sync::Semaphore::new(1)),
        search_gate: Arc::new(tokio::sync::Semaphore::new(2)),
    };
    let app = routes(state);
    let listener = tokio::net::TcpListener::bind(
        std::env::var("ARCHIVIST_BIND").unwrap_or("127.0.0.1:8098".into()),
    )
    .await?;
    tracing::info!("Archivist server ready");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
fn routes(s: App) -> Router {
    let api = Router::new()
        .route("/api/v1/imports", post(import))
        .route("/api/v1/imports/{id}", get(get_import))
        .route("/api/v1/assets", get(list))
        .route("/api/v1/assets/{id}", get(detail))
        .route("/api/v1/assets/{id}/preview", get(preview))
        .route("/api/v1/assets/{id}/original", get(original))
        .route("/api/v1/search", post(search))
        .route("/api/v1/jobs/{id}", get(job))
        .route("/api/v1/jobs/{id}/retry", post(retry))
        .route("/internal/v1/jobs/reserve", post(reserve))
        .route("/internal/v1/jobs/{id}/renew", post(renew))
        .route("/internal/v1/jobs/{id}/complete", post(complete))
        .route("/internal/v1/jobs/{id}/fail", post(fail))
        .route_layer(middleware::from_fn_with_state(s.clone(), auth));
    Router::new()
        .merge(api)
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "application/javascript")],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route(
            "/health/live",
            get(|| async { Json(json!({"status":"ok"})) }),
        )
        .route("/health/ready", get(ready))
        .layer(DefaultBodyLimit::max(128 * 1024))
        .layer(middleware::from_fn(security_headers))
        .with_state(s)
}
async fn security_headers(req: Request, next: Next) -> Response {
    let mut r = next.run(req).await;
    for (k, v) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
        (
            "content-security-policy",
            "default-src 'self'; img-src 'self' blob:; style-src 'self'; script-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    ] {
        r.headers_mut()
            .insert(axum::http::HeaderName::from_static(k), v.parse().unwrap());
    }
    r
}
async fn auth(State(s): State<App>, mut req: Request, next: Next) -> Api<Response> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if req.uri().path().starts_with("/internal/") {
        if !token_equal(token, &s.worker) {
            return Err(Error(StatusCode::UNAUTHORIZED, "unauthorized"));
        }
    } else {
        let user = s
            .users
            .iter()
            .find(|u| token_equal(token, &u.token))
            .ok_or(Error(StatusCode::UNAUTHORIZED, "unauthorized"))?;
        req.extensions_mut().insert(Actor(user.name.clone()));
    }
    Ok(next.run(req).await)
}
async fn ready(State(s): State<App>) -> StatusCode {
    if sqlx::query("SELECT 1").execute(&s.db).await.is_ok() && s.media.dir_metadata().is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Import {
    files: Vec<String>,
    copy_confirmed: bool,
}
async fn import(
    State(s): State<App>,
    Extension(actor): Extension<Actor>,
    headers: HeaderMap,
    Json(input): Json<Import>,
) -> Api<(StatusCode, Json<Value>)> {
    if !input.copy_confirmed || input.files.is_empty() || input.files.len() > 20 {
        return Err(bad());
    }
    let key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 128)
        .ok_or_else(bad)?;
    let fingerprint = hash(&serde_json::to_vec(&input).map_err(|_| bad())?);
    let _permit = s
        .gate
        .try_acquire()
        .map_err(|_| Error(StatusCode::TOO_MANY_REQUESTS, "import_busy"))?;
    if let Some(row) =
        sqlx::query("SELECT request_hash,result FROM imports WHERE actor=$1 AND idempotency_key=$2")
            .bind(&actor.0)
            .bind(key)
            .fetch_optional(&s.db)
            .await?
    {
        if row.get::<String, _>("request_hash") != fingerprint {
            return Err(conflict());
        }
        return Ok((StatusCode::ACCEPTED, Json(row.get("result"))));
    }
    let mut prepared = Vec::new();
    for path in &input.files {
        let dir = s.media.clone();
        let path = path.clone();
        let entry = tokio::task::spawn_blocking(move || -> Result<_> {
            let image = media::inspect(media::read(&dir, &path, false)?)?;
            let storage = media::persist(&dir, &image)?;
            Ok((path, storage, image))
        })
        .await
        .map_err(|_| bad())?
        .map_err(|_| bad())?;
        prepared.push(entry);
    }
    let mut tx = s.db.begin().await?;
    let mut ids = Vec::new();
    for (source, storage, image) in prepared {
        if let Some(row) =
            sqlx::query("SELECT id,sha256 FROM assets WHERE source_path=$1 FOR UPDATE")
                .bind(&source)
                .fetch_optional(&mut *tx)
                .await?
        {
            if row.get::<String, _>("sha256") != image.sha256 {
                return Err(conflict());
            }
            ids.push(row.get::<Uuid, _>("id"));
            continue;
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO assets(id,source_path,storage_path,sha256,mime,bytes,width,height) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
  .bind(id).bind(source).bind(storage).bind(image.sha256).bind(image.mime).bind(image.bytes.len() as i64).bind(image.width as i32).bind(image.height as i32).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO jobs(id,asset_id) VALUES($1,$2)")
            .bind(Uuid::new_v4())
            .bind(id)
            .execute(&mut *tx)
            .await?;
        ids.push(id);
    }
    let id = Uuid::new_v4();
    let result = json!({"id":id,"assets":ids,"status":"registered"});
    sqlx::query(
        "INSERT INTO imports(id,actor,idempotency_key,request_hash,result) VALUES($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(actor.0)
    .bind(key)
    .bind(fingerprint)
    .bind(&result)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(result)))
}
async fn get_import(State(s): State<App>, Path(id): Path<Uuid>) -> Api<Json<Value>> {
    Ok(Json(
        sqlx::query_scalar("SELECT result FROM imports WHERE id=$1")
            .bind(id)
            .fetch_optional(&s.db)
            .await?
            .ok_or_else(missing)?,
    ))
}
const ASSET_JSON: &str = "jsonb_build_object('id',a.id,'name',a.source_path,'mime',a.mime,'bytes',a.bytes,'width',a.width,'height',a.height,'description',a.description,'vision_model',a.vision_model,'embedding_model',a.embedding_model,'pipeline_version',a.pipeline_version,'job_id',j.id,'status',j.state,'error',j.error)";
#[derive(Deserialize)]
struct Page {
    limit: Option<i64>,
    offset: Option<i64>,
}
async fn list(State(s): State<App>, Query(p): Query<Page>) -> Api<Json<Value>> {
    let limit = p.limit.unwrap_or(25).clamp(1, 100);
    let offset = p.offset.unwrap_or(0).max(0);
    let query = format!(
        "SELECT {ASSET_JSON} FROM assets a JOIN jobs j ON j.asset_id=a.id ORDER BY a.created_at DESC,a.id LIMIT $1 OFFSET $2"
    );
    let items: Vec<Value> = sqlx::query_scalar(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&s.db)
        .await?;
    let next = if items.len() == limit as usize {
        Some(offset + limit)
    } else {
        None
    };
    Ok(Json(json!({"items":items,"next_offset":next})))
}
async fn detail(State(s): State<App>, Path(id): Path<Uuid>) -> Api<Json<Value>> {
    let query =
        format!("SELECT {ASSET_JSON} FROM assets a JOIN jobs j ON j.asset_id=a.id WHERE a.id=$1");
    Ok(Json(
        sqlx::query_scalar(&query)
            .bind(id)
            .fetch_optional(&s.db)
            .await?
            .ok_or_else(missing)?,
    ))
}
async fn file(s: &App, id: Uuid, preview: bool) -> Api<(Vec<u8>, String)> {
    let row = sqlx::query("SELECT storage_path,sha256,mime FROM assets WHERE id=$1")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(missing)?;
    let path = if preview {
        format!(".archivist/previews/{}.jpg", row.get::<String, _>("sha256"))
    } else {
        row.get("storage_path")
    };
    let mime = if preview {
        "image/jpeg".into()
    } else {
        row.get("mime")
    };
    let dir = s.media.clone();
    let bytes = tokio::task::spawn_blocking(move || media::read(&dir, &path, true))
        .await
        .map_err(|_| missing())?
        .map_err(|_| missing())?;
    Ok((bytes, mime))
}
async fn preview(State(s): State<App>, Path(id): Path<Uuid>) -> Api<Response> {
    let (bytes, mime) = file(&s, id, true).await?;
    Ok(([(header::CONTENT_TYPE, mime)], bytes).into_response())
}
async fn original(State(s): State<App>, Path(id): Path<Uuid>, headers: HeaderMap) -> Api<Response> {
    let (bytes, mime) = file(&s, id, false).await?;
    let len = bytes.len();
    let mut response = if let Some(value) = headers.get(header::RANGE) {
        let range = value
            .to_str()
            .ok()
            .and_then(|v| v.strip_prefix("bytes="))
            .and_then(|v| v.split_once('-'));
        let (start, end) = match range {
            Some(("", suffix)) => {
                let n = suffix
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or(Error(StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range"))?;
                (len.saturating_sub(n), len - 1)
            }
            Some((start, end)) => {
                let start = start
                    .parse::<usize>()
                    .map_err(|_| Error(StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range"))?;
                let end = if end.is_empty() {
                    len - 1
                } else {
                    end.parse::<usize>()
                        .map_err(|_| Error(StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range"))?
                        .min(len - 1)
                };
                (start, end)
            }
            None => return Err(Error(StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range")),
        };
        if start >= len || end < start {
            return Err(Error(StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range"));
        }
        (
            StatusCode::PARTIAL_CONTENT,
            [(header::CONTENT_RANGE, format!("bytes {start}-{end}/{len}"))],
            bytes[start..=end].to_vec(),
        )
            .into_response()
    } else {
        bytes.into_response()
    };
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, mime.parse().map_err(|_| bad())?);
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    let extension = match mime.as_str() {
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "jpg",
    };
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{id}.{extension}\"")
            .parse()
            .map_err(|_| bad())?,
    );
    Ok(response)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
    limit: Option<i64>,
    mode: Option<String>,
}
async fn search(State(s): State<App>, Json(input): Json<Search>) -> Api<Json<Value>> {
    let text = input.query.trim();
    if text.is_empty() || text.len() > 2000 {
        return Err(bad());
    }
    let limit = input.limit.unwrap_or(20).clamp(1, 100);
    let _permit = s
        .search_gate
        .try_acquire()
        .map_err(|_| Error(StatusCode::TOO_MANY_REQUESTS, "search_busy"))?;
    let mode = input.mode.as_deref().unwrap_or("hybrid");
    if mode == "text" {
        let q = format!(
            "SELECT {ASSET_JSON} FROM assets a JOIN jobs j ON j.asset_id=a.id WHERE to_tsvector('portuguese',a.source_path || ' ' || coalesce(a.description,'')) @@ plainto_tsquery('portuguese',$1) ORDER BY a.created_at DESC LIMIT $2"
        );
        let items: Vec<Value> = sqlx::query_scalar(&q)
            .bind(text)
            .bind(limit)
            .fetch_all(&s.db)
            .await?;
        return Ok(Json(json!({"items":items,"mode":"text"})));
    }
    if mode != "hybrid" {
        return Err(bad());
    }
    let vector = s.ai.embed(text, true).await.map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "embedding_unavailable_use_text",
        )
    })?;
    let q = format!(
        "WITH semantic AS (SELECT id,row_number() OVER(ORDER BY embedding <=> $1) AS rank FROM assets WHERE embedding_model=$2 AND embedding IS NOT NULL ORDER BY embedding <=> $1 LIMIT 100), lexical AS (SELECT id,row_number() OVER(ORDER BY ts_rank_cd(to_tsvector('portuguese',source_path || ' ' || coalesce(description,'')),plainto_tsquery('portuguese',$3)) DESC) AS rank FROM assets WHERE to_tsvector('portuguese',source_path || ' ' || coalesce(description,'')) @@ plainto_tsquery('portuguese',$3) LIMIT 100), scores AS (SELECT coalesce(s.id,l.id) AS id,coalesce(1.0/(60+s.rank),0)+coalesce(1.0/(60+l.rank),0) AS score FROM semantic s FULL JOIN lexical l ON l.id=s.id) SELECT {ASSET_JSON} || jsonb_build_object('score',scores.score) FROM scores JOIN assets a ON a.id=scores.id JOIN jobs j ON j.asset_id=a.id ORDER BY scores.score DESC,a.id LIMIT $4"
    );
    let items: Vec<Value> = sqlx::query_scalar(&q)
        .bind(pgvector::Vector::from(vector))
        .bind(&s.ai.embedding_version)
        .bind(text)
        .bind(limit)
        .fetch_all(&s.db)
        .await?;
    Ok(Json(json!({"items":items,"mode":"hybrid"})))
}
async fn job(State(s): State<App>, Path(id): Path<Uuid>) -> Api<Json<Value>> {
    Ok(Json(sqlx::query_scalar("SELECT jsonb_build_object('id',id,'asset_id',asset_id,'status',state,'attempt',attempt,'error',error) FROM jobs WHERE id=$1").bind(id).fetch_optional(&s.db).await?.ok_or_else(missing)?))
}
async fn retry(State(s): State<App>, Path(id): Path<Uuid>) -> Api<(StatusCode, Json<Value>)> {
    let n=sqlx::query("UPDATE jobs SET state='queued',attempt=0,error=NULL,available_at=now(),lease_token=NULL WHERE id=$1 AND state='failed'").bind(id).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(conflict());
    }
    Ok((StatusCode::ACCEPTED, Json(json!({"job_id":id}))))
}
async fn reserve(State(s): State<App>) -> Api<Response> {
    sqlx::query("UPDATE jobs SET state='failed',error='attempts_exhausted' WHERE state='running' AND lease_expires_at<now() AND attempt>=3").execute(&s.db).await?;
    let row:Option<Value>=sqlx::query_scalar("WITH candidate AS (SELECT id FROM jobs WHERE attempt<3 AND ((state='queued' AND available_at<=now()) OR (state='running' AND lease_expires_at<now())) ORDER BY created_at FOR UPDATE SKIP LOCKED LIMIT 1), claimed AS (UPDATE jobs j SET state='running',attempt=attempt+1,lease_token=$1,lease_expires_at=now()+interval '120 seconds' FROM candidate c WHERE j.id=c.id RETURNING j.*) SELECT jsonb_build_object('id',j.id,'asset_id',j.asset_id,'generation',j.generation,'attempt',j.attempt,'lease_token',j.lease_token,'storage_path',a.storage_path,'sha256',a.sha256) FROM claimed j JOIN assets a ON a.id=j.asset_id")
 .bind(Uuid::new_v4()).fetch_optional(&s.db).await?;
    Ok(match row {
        Some(value) => Json(value).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Lease {
    lease_token: Uuid,
    attempt: i32,
}
async fn renew(
    State(s): State<App>,
    Path(id): Path<Uuid>,
    Json(lease): Json<Lease>,
) -> Api<Json<Value>> {
    let n=sqlx::query("UPDATE jobs SET lease_expires_at=now()+interval '120 seconds' WHERE id=$1 AND lease_token=$2 AND attempt=$3 AND state='running' AND lease_expires_at>now()").bind(id).bind(lease.lease_token).bind(lease.attempt).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(conflict());
    }
    Ok(Json(json!({"status":"renewed"})))
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Completed {
    lease_token: Uuid,
    attempt: i32,
    generation: i32,
    description: String,
    embedding: Vec<f32>,
    embedding_model: String,
    vision_model: String,
    pipeline_version: String,
}
async fn complete(
    State(s): State<App>,
    Path(id): Path<Uuid>,
    Json(v): Json<Completed>,
) -> Api<Json<Value>> {
    crate::domain::validate_embedding(&v.embedding, s.ai.dimension).map_err(|_| bad())?;
    if v.description.trim().is_empty()
        || v.description.len() > 8000
        || v.embedding_model != s.ai.embedding_version
        || v.vision_model != s.ai.vision_version
        || v.pipeline_version != "image-v1"
    {
        return Err(bad());
    }
    let fingerprint = hash(&serde_json::to_vec(&v).map_err(|_| bad())?);
    let mut tx = s.db.begin().await?;
    let row =
        sqlx::query("SELECT *,lease_expires_at>now() AS valid FROM jobs WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(missing)?;
    if row.get::<Option<Uuid>, _>("lease_token") != Some(v.lease_token)
        || row.get::<i32, _>("attempt") != v.attempt
        || row.get::<i32, _>("generation") != v.generation
    {
        return Err(conflict());
    }
    if row.get::<String, _>("state") == "completed" {
        if row.get::<Option<String>, _>("result_hash").as_deref() != Some(&fingerprint) {
            return Err(conflict());
        }
        return Ok(Json(json!({"status":"completed"})));
    }
    if row.get::<String, _>("state") != "running"
        || row.get::<Option<bool>, _>("valid") != Some(true)
    {
        return Err(conflict());
    }
    sqlx::query("UPDATE assets SET description=$1,embedding=$2,embedding_model=$3,vision_model=$4,pipeline_version=$5,updated_at=now() WHERE id=$6")
 .bind(v.description).bind(pgvector::Vector::from(v.embedding)).bind(v.embedding_model).bind(v.vision_model).bind(v.pipeline_version).bind(row.get::<Uuid,_>("asset_id")).execute(&mut *tx).await?;
    sqlx::query("UPDATE jobs SET state='completed',result_hash=$1,error=NULL WHERE id=$2")
        .bind(fingerprint)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"status":"completed"})))
}
async fn fail(
    State(s): State<App>,
    Path(id): Path<Uuid>,
    Json(v): Json<Lease>,
) -> Api<Json<Value>> {
    let n=sqlx::query("UPDATE jobs SET state=CASE WHEN attempt>=3 THEN 'failed' ELSE 'queued' END,error='analysis_failed',available_at=now()+interval '30 seconds' WHERE id=$1 AND lease_token=$2 AND attempt=$3 AND state='running' AND lease_expires_at>now()")
 .bind(id).bind(v.lease_token).bind(v.attempt).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(conflict());
    }
    Ok(Json(json!({"status":"recorded"})))
}
