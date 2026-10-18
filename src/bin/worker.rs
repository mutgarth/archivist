use anyhow::{Result, ensure};
use archivist::{ai::Ai, media};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
#[derive(Deserialize)]
struct Job {
    id: String,
    generation: i32,
    attempt: i32,
    lease_token: String,
    storage_path: String,
    sha256: String,
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let ai = Ai::from_env()?;
    let root = media::root(&std::env::var("MEDIA_ROOT")?)?;
    let base = std::env::var("ARCHIVIST_URL")?;
    let token = std::env::var("ARCHIVIST_WORKER_TOKEN")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    loop {
        let response = client
            .post(format!("{base}/internal/v1/jobs/reserve"))
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await;
        let response = match response {
            Ok(r) if r.status().is_success() => r,
            _ => {
                tracing::warn!("reservation unavailable");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        if response.status() == 204 {
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        }
        let job: Job = response.json().await?;
        let lease = json!({"lease_token":job.lease_token,"attempt":job.attempt});
        let url = format!("{base}/internal/v1/jobs/{}", job.id);
        let result = async {
            let data = media::read(&root, &job.storage_path, true)?;
            let image = media::inspect(data)?;
            ensure!(image.sha256 == job.sha256, "content_changed");
            let description = ai.describe(&image.preview).await?;
            let embedding = ai.embed(&description, false).await?;
            Ok::<_, anyhow::Error>(
                json!({"lease_token":job.lease_token,"attempt":job.attempt,"generation":job.generation,
     "description":description,"embedding":embedding,"embedding_model":ai.embedding_version,
     "vision_model":ai.vision_version,"pipeline_version":"image-v1"}),
            )
        };
        tokio::pin!(result);
        let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
        let outcome = loop {
            tokio::select! {
             value=&mut result=>break value,
             _=heartbeat.tick()=>{
              match client.post(format!("{url}/renew")).bearer_auth(&token).json(&lease).send().await {
               Ok(r) if r.status().is_success()=>{},
               _=>break Err(anyhow::anyhow!("lease_lost")),
              }
             }
            }
        };
        match outcome {
            Ok(body) => {
                // A lost acknowledgement can be retried without rerunning inference.
                for attempt in 0..3 {
                    match client
                        .post(format!("{url}/complete"))
                        .bearer_auth(&token)
                        .json(&body)
                        .send()
                        .await
                    {
                        Ok(r) if r.status().is_success() => {
                            tracing::info!(job_id = job.id, "analysis completed");
                            break;
                        }
                        Ok(r) if r.status().is_client_error() => {
                            tracing::warn!(job_id = job.id, "result rejected");
                            break;
                        }
                        _ => {
                            tracing::warn!(attempt, "completion unavailable");
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                    }
                }
            }
            Err(_) => {
                tracing::warn!(job_id = job.id, "analysis failed");
                let _ = client
                    .post(format!("{url}/fail"))
                    .bearer_auth(&token)
                    .json(&lease)
                    .send()
                    .await;
            }
        }
    }
}
