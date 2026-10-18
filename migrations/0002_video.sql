ALTER TABLE assets ADD COLUMN media_metadata jsonb NOT NULL DEFAULT '{}'::jsonb;
