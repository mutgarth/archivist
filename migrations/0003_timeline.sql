ALTER TABLE assets ADD COLUMN transcription_model text;
CREATE TABLE asset_segments (
 id uuid PRIMARY KEY,
 asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
 generation integer NOT NULL,
 kind text NOT NULL CHECK (kind IN ('visual','speech')),
 start_ms bigint NOT NULL CHECK (start_ms >= 0),
 end_ms bigint NOT NULL CHECK (end_ms > start_ms),
 text text NOT NULL,
 embedding vector NOT NULL,
 embedding_model text NOT NULL,
 CHECK (end_ms <= 86400000)
);
CREATE INDEX asset_segments_asset_time ON asset_segments(asset_id,start_ms);
CREATE INDEX asset_segments_text ON asset_segments USING gin(to_tsvector('portuguese',text));
