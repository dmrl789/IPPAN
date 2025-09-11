-- Migration: Add block parents support to IPPAN (SQLite version)
-- Version: 001
-- Description: Adds parents and parent_rounds columns to blocks table

-- Add parents and parent_rounds columns to blocks table
-- SQLite stores arrays as JSON text
ALTER TABLE blocks ADD COLUMN parents TEXT DEFAULT '[]';
ALTER TABLE blocks ADD COLUMN parent_rounds TEXT DEFAULT '[]';

-- Create indexes for efficient parent lookups
-- Note: SQLite doesn't have GIN indexes, so we use regular indexes
CREATE INDEX IF NOT EXISTS idx_blocks_parents ON blocks(parents);
CREATE INDEX IF NOT EXISTS idx_blocks_parent_rounds ON blocks(parent_rounds);

-- Create function to validate parent references (SQLite doesn't support functions)
-- This validation will be done at the application level

-- Create view for block parent relationships
CREATE VIEW IF NOT EXISTS block_parent_relationships AS
SELECT 
  b.hash as block_hash,
  b.round as block_round,
  b.height as block_height,
  json_extract(parent.value, '$') as parent_hash,
  json_extract(parent_round.value, '$') as parent_round,
  parent.key + 1 as parent_index
FROM blocks b,
json_each(b.parents) as parent,
json_each(b.parent_rounds) as parent_round
WHERE parent.key = parent_round.key;

-- Create indexes on the view for efficient lookups
CREATE INDEX IF NOT EXISTS idx_block_parent_relationships_block_hash 
  ON block_parent_relationships (block_hash);

CREATE INDEX IF NOT EXISTS idx_block_parent_relationships_parent_hash 
  ON block_parent_relationships (parent_hash);

-- Add comments for documentation (SQLite doesn't support comments on columns)
-- parents: Array of parent block hashes (32-byte each) stored as JSON
-- parent_rounds: Array of parent round numbers corresponding to parents array stored as JSON

-- Note: SQLite doesn't support:
-- - Array data types (using JSON instead)
-- - Custom functions (validation done in application)
-- - Triggers with complex logic (validation done in application)
-- - Recursive CTEs for ancestor/descendant queries (done in application)

-- Application-level validation rules:
-- 1. parents and parent_rounds must be valid JSON arrays
-- 2. parents array length must equal parent_rounds array length
-- 3. parents array length must be between 0 and 8 (0 only for genesis)
-- 4. All parent hashes must exist in the blocks table
-- 5. All parent rounds must be <= block round
-- 6. No cycles in the block DAG
-- 7. No duplicate parent hashes
