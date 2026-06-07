#!/bin/bash

# Local PostgreSQL Database Setup Script for Cult Backend
# This script helps manage the local database for development

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Database configuration
DB_NAME="cult_db"
DB_USER="cult_user"
DB_PASSWORD="cult_password"
DB_HOST="localhost"
DB_PORT="5432"

echo -e "${GREEN}=== Cult Backend Local Database Setup ===${NC}"

# Function to check if PostgreSQL is running
check_postgres() {
    echo -e "${YELLOW}Checking PostgreSQL status...${NC}"
    if brew services list | grep -q "postgresql@15.*started"; then
        echo -e "${GREEN}✓ PostgreSQL is running${NC}"
        return 0
    else
        echo -e "${RED}✗ PostgreSQL is not running${NC}"
        return 1
    fi
}

# Function to start PostgreSQL
start_postgres() {
    echo -e "${YELLOW}Starting PostgreSQL...${NC}"
    brew services start postgresql@15
    sleep 3
    check_postgres
}

# Function to create database and user
setup_database() {
    echo -e "${YELLOW}Setting up database and user...${NC}"
    
    # Create user if it doesn't exist
    if ! psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='$DB_USER'" | grep -q 1; then
        echo "Creating user $DB_USER..."
        createuser -s $DB_USER
    else
        echo "User $DB_USER already exists"
    fi
    
    # Set password for user
    psql -d postgres -c "ALTER USER $DB_USER PASSWORD '$DB_PASSWORD';" 2>/dev/null || true
    
    # Create database if it doesn't exist
    if ! psql -lqt | cut -d \| -f 1 | grep -qw $DB_NAME; then
        echo "Creating database $DB_NAME..."
        createdb $DB_NAME
    else
        echo "Database $DB_NAME already exists"
    fi
}

# Function to run migrations
run_migrations() {
    echo -e "${YELLOW}Running database migrations...${NC}"
    
    for migration in migrations/*.sql; do
        if [ -f "$migration" ]; then
            echo "Running migration: $(basename $migration)"
            psql -d $DB_NAME -U $DB_USER -f "$migration"
        fi
    done
    
    echo -e "${GREEN}✓ All migrations completed${NC}"
}

# Function to test database connection
test_connection() {
    echo -e "${YELLOW}Testing database connection...${NC}"
    
    if psql -d $DB_NAME -U $DB_USER -c "SELECT version();" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ Database connection successful${NC}"
        return 0
    else
        echo -e "${RED}✗ Database connection failed${NC}"
        return 1
    fi
}

# Function to show database info
show_info() {
    echo -e "${GREEN}=== Database Information ===${NC}"
    echo "Database URL: postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"
    echo "Database Name: $DB_NAME"
    echo "Database User: $DB_USER"
    echo "Database Host: $DB_HOST"
    echo "Database Port: $DB_PORT"
    echo ""
    echo -e "${YELLOW}To connect manually:${NC}"
    echo "psql -d $DB_NAME -U $DB_USER"
    echo ""
    echo -e "${YELLOW}To view tables:${NC}"
    echo "psql -d $DB_NAME -U $DB_USER -c \"\\dt\""
}

# Function to reset database
reset_database() {
    echo -e "${YELLOW}Resetting database...${NC}"
    read -p "This will drop and recreate the database. Are you sure? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        dropdb $DB_NAME 2>/dev/null || true
        createdb $DB_NAME
        run_migrations
        echo -e "${GREEN}✓ Database reset completed${NC}"
    else
        echo "Database reset cancelled"
    fi
}

# Main script logic
case "${1:-setup}" in
    "setup")
        if ! check_postgres; then
            start_postgres
        fi
        setup_database
        run_migrations
        test_connection
        show_info
        ;;
    "start")
        start_postgres
        ;;
    "stop")
        echo -e "${YELLOW}Stopping PostgreSQL...${NC}"
        brew services stop postgresql@15
        echo -e "${GREEN}✓ PostgreSQL stopped${NC}"
        ;;
    "restart")
        echo -e "${YELLOW}Restarting PostgreSQL...${NC}"
        brew services restart postgresql@15
        sleep 3
        check_postgres
        ;;
    "migrate")
        run_migrations
        ;;
    "test")
        test_connection
        ;;
    "info")
        show_info
        ;;
    "reset")
        reset_database
        ;;
    "status")
        check_postgres
        ;;
    *)
        echo "Usage: $0 {setup|start|stop|restart|migrate|test|info|reset|status}"
        echo ""
        echo "Commands:"
        echo "  setup   - Complete database setup (default)"
        echo "  start   - Start PostgreSQL service"
        echo "  stop    - Stop PostgreSQL service"
        echo "  restart - Restart PostgreSQL service"
        echo "  migrate - Run database migrations"
        echo "  test    - Test database connection"
        echo "  info    - Show database information"
        echo "  reset   - Reset database (drop and recreate)"
        echo "  status  - Check PostgreSQL status"
        exit 1
        ;;
esac 