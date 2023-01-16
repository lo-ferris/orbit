ALTER TABLE posts DROP COLUMN ext_apub_uri;
ALTER TABLE posts ADD COLUMN replies_uri VARCHAR(2048) NULL;
ALTER TABLE comments ADD COLUMN uri VARCHAR(2048) NULL;
ALTER TABLE comments ADD COLUMN replies_uri VARCHAR(2048) NULL;

UPDATE posts SET replies_uri = '';
UPDATE comments SET uri = '';
UPDATE comments SET replies_uri = '';
ALTER TABLE posts ALTER COLUMN replies_uri SET NOT NULL;
ALTER TABLE comments ALTER COLUMN uri SET NOT NULL;
ALTER TABLE comments ALTER COLUMN replies_uri SET NOT NULL;
