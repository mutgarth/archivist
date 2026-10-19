ALTER TABLE assets ADD COLUMN organization jsonb;
CREATE TABLE automation_status (id boolean PRIMARY KEY DEFAULT true CHECK(id), body jsonb NOT NULL, updated_at timestamptz NOT NULL DEFAULT now());
