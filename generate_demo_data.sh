#!/bin/bash

# Script to generate demo Redis data
# Usage: ./generate_demo_data.sh

REDIS_HOST="localhost"
REDIS_PORT="6379"
REDIS_PASSWORD="admin@123"

echo "Generating 1000 sample Redis keys..."

# Function to execute Redis command
redis_cmd() {
    redis-cli -h $REDIS_HOST -p $REDIS_PORT -a $REDIS_PASSWORD "$@"
}

# Clear existing sample keys first
redis_cmd --scan --pattern "sample:*" | xargs -r redis-cli -h $REDIS_HOST -p $REDIS_PORT -a $REDIS_PASSWORD DEL 2>/dev/null

echo "Creating demo keys..."

# Generate 1000 keys with different types
for i in {1..200}; do
    # String keys (20%)
    redis_cmd SET "sample:user:$i:name" "User $i" > /dev/null
    redis_cmd SET "sample:user:$i:email" "user$i@example.com" > /dev/null
    redis_cmd SETEX "sample:session:$i" 3600 "session_token_$i" > /dev/null
done

for i in {1..150}; do
    # Hash keys (15%)
    redis_cmd HSET "sample:product:$i" \
        "id" "$i" \
        "name" "Product $i" \
        "price" "$((RANDOM % 1000 + 10))" \
        "stock" "$((RANDOM % 100))" \
        "category" "Category $((i % 5))" > /dev/null
done

for i in {1..100}; do
    # List keys (10%)
    redis_cmd RPUSH "sample:cart:$i" "item1" "item2" "item3" "item4" "item5" > /dev/null
    redis_cmd LPUSH "sample:notifications:$i" "Welcome!" "New message" "Update available" > /dev/null
done

for i in {1..100}; do
    # Set keys (10%)
    redis_cmd SADD "sample:tags:$i" "tag1" "tag2" "tag3" "tag4" "tag5" > /dev/null
    redis_cmd SADD "sample:followers:$i" "user$((RANDOM % 100))" "user$((RANDOM % 100))" > /dev/null
done

for i in {1..50}; do
    # Sorted Set (ZSet) keys (5%)
    redis_cmd ZADD "sample:leaderboard:$i" \
        "$((RANDOM % 1000))" "player1" \
        "$((RANDOM % 1000))" "player2" \
        "$((RANDOM % 1000))" "player3" \
        "$((RANDOM % 1000))" "player4" \
        "$((RANDOM % 1000))" "player5" > /dev/null
done

# Additional nested directory structure for better tree view
for category in users products orders sessions analytics; do
    for i in {1..80}; do
        redis_cmd SET "sample:$category:region:eu:item:$i" "value_$i" > /dev/null
        redis_cmd SET "sample:$category:region:us:item:$i" "value_$i" > /dev/null
    done
done

# Count total keys created
total=$(redis_cmd --scan --pattern "sample:*" | wc -l)

echo "✓ Done! Created $total sample keys"
echo "Redis: $REDIS_HOST:$REDIS_PORT"
echo ""
echo "Example keys created:"
echo "  - sample:user:1:name (String)"
echo "  - sample:product:1 (Hash)"
echo "  - sample:cart:1 (List)"
echo "  - sample:tags:1 (Set)"
echo "  - sample:leaderboard:1 (ZSet)"
echo "  - sample:users:region:eu:item:1 (String with nested dirs)"
