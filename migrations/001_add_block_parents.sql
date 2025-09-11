-- Migration: Add block parents support to IPPAN
-- Version: 001
-- Description: Adds parents and parent_rounds columns to blocks table

-- Add parents and parent_rounds columns to blocks table
ALTER TABLE blocks ADD COLUMN IF NOT EXISTS parents BYTEA[] NOT NULL DEFAULT '{}';
ALTER TABLE blocks ADD COLUMN IF NOT EXISTS parent_rounds BIGINT[] NOT NULL DEFAULT '{}';

-- Create GIN index for efficient parent lookups
CREATE INDEX IF NOT EXISTS idx_blocks_parents_gin ON blocks USING GIN (parents);

-- Create index for parent rounds
CREATE INDEX IF NOT EXISTS idx_blocks_parent_rounds ON blocks USING GIN (parent_rounds);

-- Add constraint to ensure parent arrays are not empty (except for genesis)
-- Note: This constraint will be added after data migration
-- ALTER TABLE blocks ADD CONSTRAINT chk_blocks_has_parents 
--   CHECK (block_height = 0 OR array_length(parents, 1) > 0);

-- Add constraint to limit number of parents (max 8)
ALTER TABLE blocks ADD CONSTRAINT chk_blocks_max_parents 
  CHECK (array_length(parents, 1) IS NULL OR array_length(parents, 1) <= 8);

-- Add constraint to ensure parent_rounds matches parents length
ALTER TABLE blocks ADD CONSTRAINT chk_blocks_parent_rounds_length 
  CHECK (array_length(parents, 1) = array_length(parent_rounds, 1));

-- Add constraint to ensure parent rounds are <= block round
-- This will be enforced at application level for now
-- ALTER TABLE blocks ADD CONSTRAINT chk_blocks_parent_rounds_valid 
--   CHECK (parent_rounds[i] <= round FOR ALL i IN 1..array_length(parent_rounds, 1));

-- Create function to validate parent references
CREATE OR REPLACE FUNCTION validate_block_parents()
RETURNS TRIGGER AS $$
BEGIN
  -- Check that all parent hashes exist in the blocks table
  FOR i IN 1..array_length(NEW.parents, 1) LOOP
    IF NOT EXISTS (
      SELECT 1 FROM blocks 
      WHERE hash = NEW.parents[i]
    ) THEN
      RAISE EXCEPTION 'Parent block % does not exist', encode(NEW.parents[i], 'hex');
    END IF;
  END LOOP;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to validate parent references
DROP TRIGGER IF EXISTS trigger_validate_block_parents ON blocks;
CREATE TRIGGER trigger_validate_block_parents
  BEFORE INSERT OR UPDATE ON blocks
  FOR EACH ROW
  EXECUTE FUNCTION validate_block_parents();

-- Create function to check for cycles in block DAG
CREATE OR REPLACE FUNCTION check_block_cycles()
RETURNS TRIGGER AS $$
DECLARE
  parent_hash BYTEA;
  visited_blocks BYTEA[] := ARRAY[NEW.hash];
  current_blocks BYTEA[] := NEW.parents;
  next_blocks BYTEA[];
  i INTEGER;
BEGIN
  -- Perform BFS to detect cycles
  WHILE array_length(current_blocks, 1) > 0 LOOP
    next_blocks := ARRAY[]::BYTEA[];
    
    FOREACH parent_hash IN ARRAY current_blocks LOOP
      -- Check if we've already visited this block (cycle detected)
      IF parent_hash = ANY(visited_blocks) THEN
        RAISE EXCEPTION 'Cycle detected in block DAG involving block %', encode(parent_hash, 'hex');
      END IF;
      
      -- Add to visited blocks
      visited_blocks := array_append(visited_blocks, parent_hash);
      
      -- Get parents of this block for next iteration
      SELECT array_cat(next_blocks, parents) INTO next_blocks
      FROM blocks WHERE hash = parent_hash;
    END LOOP;
    
    current_blocks := next_blocks;
  END LOOP;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to check for cycles
DROP TRIGGER IF EXISTS trigger_check_block_cycles ON blocks;
CREATE TRIGGER trigger_check_block_cycles
  BEFORE INSERT OR UPDATE ON blocks
  FOR EACH ROW
  EXECUTE FUNCTION check_block_cycles();

-- Create view for block parent relationships
CREATE OR REPLACE VIEW block_parent_relationships AS
SELECT 
  b.hash as block_hash,
  b.round as block_round,
  b.height as block_height,
  parent_hash,
  parent_round,
  row_number() OVER (PARTITION BY b.hash ORDER BY parent_round, parent_hash) as parent_index
FROM blocks b,
LATERAL unnest(b.parents, b.parent_rounds) AS t(parent_hash, parent_round);

-- Create index on the view for efficient lookups
CREATE INDEX IF NOT EXISTS idx_block_parent_relationships_block_hash 
  ON block_parent_relationships (block_hash);

CREATE INDEX IF NOT EXISTS idx_block_parent_relationships_parent_hash 
  ON block_parent_relationships (parent_hash);

-- Create function to get block ancestors
CREATE OR REPLACE FUNCTION get_block_ancestors(target_hash BYTEA, max_depth INTEGER DEFAULT 100)
RETURNS TABLE(
  ancestor_hash BYTEA,
  ancestor_round BIGINT,
  ancestor_height BIGINT,
  depth INTEGER
) AS $$
BEGIN
  RETURN QUERY
  WITH RECURSIVE ancestor_tree AS (
    -- Base case: start with the target block
    SELECT 
      b.hash,
      b.round,
      b.height,
      0 as depth
    FROM blocks b
    WHERE b.hash = target_hash
    
    UNION ALL
    
    -- Recursive case: get parents
    SELECT 
      b.hash,
      b.round,
      b.height,
      at.depth + 1
    FROM blocks b
    JOIN ancestor_tree at ON b.hash = ANY(at.parents)
    WHERE at.depth < max_depth
  )
  SELECT 
    at.hash,
    at.round,
    at.height,
    at.depth
  FROM ancestor_tree at
  WHERE at.depth > 0  -- Exclude the target block itself
  ORDER BY at.depth, at.round, at.hash;
END;
$$ LANGUAGE plpgsql;

-- Create function to get block descendants
CREATE OR REPLACE FUNCTION get_block_descendants(target_hash BYTEA, max_depth INTEGER DEFAULT 100)
RETURNS TABLE(
  descendant_hash BYTEA,
  descendant_round BIGINT,
  descendant_height BIGINT,
  depth INTEGER
) AS $$
BEGIN
  RETURN QUERY
  WITH RECURSIVE descendant_tree AS (
    -- Base case: start with the target block
    SELECT 
      b.hash,
      b.round,
      b.height,
      0 as depth
    FROM blocks b
    WHERE b.hash = target_hash
    
    UNION ALL
    
    -- Recursive case: get children
    SELECT 
      b.hash,
      b.round,
      b.height,
      dt.depth + 1
    FROM blocks b
    JOIN descendant_tree dt ON target_hash = ANY(b.parents)
    WHERE dt.depth < max_depth
  )
  SELECT 
    dt.hash,
    dt.round,
    dt.height,
    dt.depth
  FROM descendant_tree dt
  WHERE dt.depth > 0  -- Exclude the target block itself
  ORDER BY dt.depth, dt.round, dt.hash;
END;
$$ LANGUAGE plpgsql;

-- Add comments for documentation
COMMENT ON COLUMN blocks.parents IS 'Array of parent block hashes (32-byte each)';
COMMENT ON COLUMN blocks.parent_rounds IS 'Array of parent round numbers corresponding to parents array';
COMMENT ON INDEX idx_blocks_parents_gin IS 'GIN index for efficient parent hash lookups';
COMMENT ON INDEX idx_blocks_parent_rounds IS 'GIN index for efficient parent round lookups';
COMMENT ON VIEW block_parent_relationships IS 'Normalized view of block parent relationships';
COMMENT ON FUNCTION get_block_ancestors IS 'Get all ancestors of a block up to max_depth';
COMMENT ON FUNCTION get_block_descendants IS 'Get all descendants of a block up to max_depth';
