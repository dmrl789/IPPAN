#!/bin/bash

# Migration script for adding block parents support to IPPAN
# This script applies the database migration to add parents and parent_rounds columns

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Configuration
DB_TYPE="${DB_TYPE:-postgresql}"  # Default to PostgreSQL
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-ippan}"
DB_USER="${DB_USER:-ippan}"
DB_PASSWORD="${DB_PASSWORD:-}"

# Migration file paths
POSTGRES_MIGRATION="migrations/001_add_block_parents.sql"
SQLITE_MIGRATION="migrations/001_add_block_parents_sqlite.sql"

# Function to check if database exists
check_database_exists() {
    case $DB_TYPE in
        postgresql)
            PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" > /dev/null 2>&1
            ;;
        sqlite)
            if [ -f "$DB_NAME" ]; then
                sqlite3 "$DB_NAME" "SELECT 1;" > /dev/null 2>&1
            else
                return 1
            fi
            ;;
        *)
            error "Unsupported database type: $DB_TYPE"
            exit 1
            ;;
    esac
}

# Function to check if migration has already been applied
check_migration_applied() {
    case $DB_TYPE in
        postgresql)
            PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT column_name FROM information_schema.columns WHERE table_name='blocks' AND column_name='parents';" | grep -q "parents"
            ;;
        sqlite)
            sqlite3 "$DB_NAME" "PRAGMA table_info(blocks);" | grep -q "parents"
            ;;
    esac
}

# Function to backup database
backup_database() {
    log "Creating database backup..."
    
    case $DB_TYPE in
        postgresql)
            BACKUP_FILE="backup_${DB_NAME}_$(date +%Y%m%d_%H%M%S).sql"
            PGPASSWORD="$DB_PASSWORD" pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME" > "$BACKUP_FILE"
            success "Database backed up to: $BACKUP_FILE"
            ;;
        sqlite)
            BACKUP_FILE="backup_${DB_NAME}_$(date +%Y%m%d_%H%M%S).db"
            cp "$DB_NAME" "$BACKUP_FILE"
            success "Database backed up to: $BACKUP_FILE"
            ;;
    esac
}

# Function to apply PostgreSQL migration
apply_postgres_migration() {
    log "Applying PostgreSQL migration..."
    
    if [ ! -f "$POSTGRES_MIGRATION" ]; then
        error "Migration file not found: $POSTGRES_MIGRATION"
        exit 1
    fi
    
    PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$POSTGRES_MIGRATION"
    
    if [ $? -eq 0 ]; then
        success "PostgreSQL migration applied successfully"
    else
        error "Failed to apply PostgreSQL migration"
        exit 1
    fi
}

# Function to apply SQLite migration
apply_sqlite_migration() {
    log "Applying SQLite migration..."
    
    if [ ! -f "$SQLITE_MIGRATION" ]; then
        error "Migration file not found: $SQLITE_MIGRATION"
        exit 1
    fi
    
    sqlite3 "$DB_NAME" < "$SQLITE_MIGRATION"
    
    if [ $? -eq 0 ]; then
        success "SQLite migration applied successfully"
    else
        error "Failed to apply SQLite migration"
        exit 1
    fi
}

# Function to verify migration
verify_migration() {
    log "Verifying migration..."
    
    case $DB_TYPE in
        postgresql)
            # Check if columns exist
            PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
                SELECT column_name, data_type 
                FROM information_schema.columns 
                WHERE table_name='blocks' 
                AND column_name IN ('parents', 'parent_rounds')
                ORDER BY column_name;
            "
            
            # Check if indexes exist
            PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
                SELECT indexname 
                FROM pg_indexes 
                WHERE tablename='blocks' 
                AND indexname LIKE '%parent%';
            "
            ;;
        sqlite)
            # Check if columns exist
            sqlite3 "$DB_NAME" "PRAGMA table_info(blocks);" | grep -E "(parents|parent_rounds)"
            
            # Check if indexes exist
            sqlite3 "$DB_NAME" "PRAGMA index_list(blocks);" | grep -E "parent"
            ;;
    esac
    
    success "Migration verification completed"
}

# Function to rollback migration
rollback_migration() {
    warning "Rolling back migration..."
    
    case $DB_TYPE in
        postgresql)
            PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
                DROP TRIGGER IF EXISTS trigger_validate_block_parents ON blocks;
                DROP TRIGGER IF EXISTS trigger_check_block_cycles ON blocks;
                DROP FUNCTION IF EXISTS validate_block_parents();
                DROP FUNCTION IF EXISTS check_block_cycles();
                DROP FUNCTION IF EXISTS get_block_ancestors(BYTEA, INTEGER);
                DROP FUNCTION IF EXISTS get_block_descendants(BYTEA, INTEGER);
                DROP VIEW IF EXISTS block_parent_relationships;
                DROP INDEX IF EXISTS idx_blocks_parents_gin;
                DROP INDEX IF EXISTS idx_blocks_parent_rounds;
                DROP INDEX IF EXISTS idx_block_parent_relationships_block_hash;
                DROP INDEX IF EXISTS idx_block_parent_relationships_parent_hash;
                ALTER TABLE blocks DROP COLUMN IF EXISTS parents;
                ALTER TABLE blocks DROP COLUMN IF EXISTS parent_rounds;
            "
            ;;
        sqlite)
            sqlite3 "$DB_NAME" "
                DROP VIEW IF EXISTS block_parent_relationships;
                DROP INDEX IF EXISTS idx_blocks_parents;
                DROP INDEX IF EXISTS idx_blocks_parent_rounds;
                DROP INDEX IF EXISTS idx_block_parent_relationships_block_hash;
                DROP INDEX IF EXISTS idx_block_parent_relationships_parent_hash;
                -- Note: SQLite doesn't support DROP COLUMN, so we need to recreate the table
                -- This is a simplified rollback - in production, you'd want a more sophisticated approach
            "
            ;;
    esac
    
    success "Migration rolled back"
}

# Main function
main() {
    log "Starting block parents migration for $DB_TYPE database"
    
    # Check if database exists
    if ! check_database_exists; then
        error "Database connection failed or database does not exist"
        exit 1
    fi
    
    # Check if migration has already been applied
    if check_migration_applied; then
        warning "Migration appears to have already been applied"
        read -p "Do you want to continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log "Migration cancelled"
            exit 0
        fi
    fi
    
    # Create backup
    backup_database
    
    # Apply migration based on database type
    case $DB_TYPE in
        postgresql)
            apply_postgres_migration
            ;;
        sqlite)
            apply_sqlite_migration
            ;;
        *)
            error "Unsupported database type: $DB_TYPE"
            exit 1
            ;;
    esac
    
    # Verify migration
    verify_migration
    
    success "Block parents migration completed successfully!"
    log "New features available:"
    log "  - Block parent relationships"
    log "  - Parent validation and cycle detection"
    log "  - Ancestor/descendant queries"
    log "  - Efficient parent lookups with indexes"
}

# Handle command line arguments
case "${1:-}" in
    --rollback)
        rollback_migration
        exit 0
        ;;
    --help)
        echo "Usage: $0 [--rollback] [--help]"
        echo ""
        echo "Environment variables:"
        echo "  DB_TYPE      Database type (postgresql|sqlite) [default: postgresql]"
        echo "  DB_HOST      Database host [default: localhost]"
        echo "  DB_PORT      Database port [default: 5432]"
        echo "  DB_NAME      Database name [default: ippan]"
        echo "  DB_USER      Database user [default: ippan]"
        echo "  DB_PASSWORD  Database password [default: empty]"
        echo ""
        echo "Options:"
        echo "  --rollback   Rollback the migration"
        echo "  --help       Show this help message"
        exit 0
        ;;
    "")
        main
        ;;
    *)
        error "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac
