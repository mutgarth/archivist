CREATE EXTENSION IF NOT EXISTS vector;
CREATE TABLE imports (
 id uuid PRIMARY KEY,
 actor text NOT NULL,
 idempotency_key text NOT NULL,
 request_hash text NOT NULL,
 result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(actor,idempotency_key)
);
CREATE TABLE assets (
 id uuid PRIMARY KEY,
 source_path text NOT NULL UNIQUE,
 storage_path text NOT NULL,
 sha256 text NOT NULL,
 mime text NOT NULL,
 bytes bigint NOT NULL,
 width integer NOT NULL,
 height integer NOT NULL,
 description text,
 embedding vector,
 embedding_model text,
 vision_model text,
 pipeline_version text,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE jobs (
 id uuid PRIMARY KEY,
 asset_id uuid NOT NULL REFERENCES assets(id),
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','completed','failed')),
 generation integer NOT NULL DEFAULT 1,
 attempt integer NOT NULL DEFAULT 0,
 lease_token uuid,
 lease_expires_at timestamptz,
 available_at timestamptz NOT NULL DEFAULT now(),
 result_hash text,
 error text,
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(asset_id,generation)
);
CREATE INDEX jobs_reservation ON jobs(state,available_at);
