DROP TRIGGER IF EXISTS UPDATE_OLDER_PURCHASE_ONLY_TRIGGER ON last_sold_metadatas;
DROP TRIGGER IF EXISTS INSERT_LAST_SOLD_METADATA_TRIGGER ON purchases;
DROP TABLE IF EXISTS last_sold_metadatas;
DROP FUNCTION IF EXISTS INSERT_LAST_SOLD_METADATA();
DROP FUNCTION IF EXISTS UPDATE_OLDER_PURCHASE_ONLY();