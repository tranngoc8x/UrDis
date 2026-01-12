#!/usr/bin/env python3
"""
Generate demo Redis data for UrDis app screenshots
"""

import redis
import random

# Redis connection settings
REDIS_HOST = "localhost"
REDIS_PORT = 6379
REDIS_PASSWORD = "admin123"

def main():
    print("Connecting to Redis...")
    
    # Try different auth methods
    auth_methods = [
        ("username + password", {"host": REDIS_HOST, "port": REDIS_PORT, "username": "admin", "password": REDIS_PASSWORD, "decode_responses": True}),
        ("password only", {"host": REDIS_HOST, "port": REDIS_PORT, "password": REDIS_PASSWORD, "decode_responses": True}),
        ("no auth", {"host": REDIS_HOST, "port": REDIS_PORT, "decode_responses": True}),
    ]
    
    r = None
    for method_name, kwargs in auth_methods:
        try:
            print(f"  Trying {method_name}...")
            r = redis.Redis(**kwargs)
            r.ping()
            print(f"✓ Connected to Redis at {REDIS_HOST}:{REDIS_PORT} ({method_name})")
            break
        except Exception as e:
            print(f"  ✗ {method_name} failed: {e}")
            continue
    
    if r is None:
        print("\n✗ Could not connect to Redis with any method")
        print("\nPlease ensure:")
        print("  1. Redis is running")
        print("  2. Check your Redis config for password")
        print("  3. Install redis-py: pip install redis")
        return

    print("\nGenerating 1000+ sample Redis keys...")
    
    # Clear existing sample keys
    print("Clearing old sample keys...")
    for key in r.scan_iter("sample:*", count=100):
        r.delete(key)
    
    count = 0
    
    # String keys at root (100 keys)
    print("Creating String keys...")
    for i in range(1, 101):
        r.set(f"sample:string_{i}", f"String value {i}")
        count += 1
    
    # Hash keys at root (100 keys)
    print("Creating Hash keys...")
    for i in range(1, 101):
        r.hset(f"sample:hash_{i}", mapping={
            "id": str(i),
            "name": f"Hash {i}",
            "value": str(random.randint(10, 1000)),
        })
        count += 1
    
    # List keys at root (100 keys)
    print("Creating List keys...")
    for i in range(1, 101):
        r.rpush(f"sample:list_{i}", "item1", "item2", "item3")
        count += 1
    
    # Set keys at root (100 keys)
    print("Creating Set keys...")
    for i in range(1, 101):
        r.sadd(f"sample:set_{i}", "member1", "member2", "member3")
        count += 1
    
    # ZSet keys at root (100 keys)
    print("Creating ZSet keys...")
    for i in range(1, 101):
        r.zadd(f"sample:zset_{i}", {
            "player1": random.randint(1, 1000),
            "player2": random.randint(1, 1000),
            "player3": random.randint(1, 1000),
        })
        count += 1
    
    # Create 2-level directory structure (500 keys)
    print("Creating 2-level directories...")
    categories = ["users", "products", "orders", "sessions", "analytics"]
    for category in categories:
        for i in range(1, 101):
            r.set(f"sample:{category}:item_{i}", f"{category} value {i}")
            count += 1
    
    # Create demo directory with realistic key names
    print("Creating demo directory with realistic examples...")
    
    # String keys (2)
    r.set("sample:demo:user_email", "john.doe@example.com")
    r.set("sample:demo:api_token", "sk_live_abc123xyz789")
    count += 2
    
    # Hash keys (2)
    r.hset("sample:demo:user_profile", mapping={
        "user_id": "12345",
        "username": "johndoe",
        "email": "john@example.com",
        "created_at": "2024-01-01"
    })
    # Create product_info hash with 5000+ fields
    import json
    
    # JSON object with ~50 items for stock field
    stock_json = {
        "total_quantity": 5000,
        "available": 4850,
        "reserved": 150,
        "warehouse": {
            "main": 3000,
            "secondary": 1500,
            "retail": 500
        },
        "history": [
            {"date": f"2024-01-{i:02d}", "quantity": 5000 - i*10, "action": "restock" if i % 2 == 0 else "sale"}
            for i in range(1, 11)
        ],
        "suppliers": {
            f"supplier_{i}": {
                "name": f"Supplier Company {i}",
                "contact": f"contact{i}@example.com",
                "quantity": i * 100,
                "price": round(1299.99 - i*10, 2),
                "delivery_time": f"{i*2} days",
                "rating": round(4.5 - i*0.1, 2)
            }
            for i in range(1, 6)
        },
        "locations": {
            f"location_{i}": {
                "warehouse_id": f"WH-{i:03d}",
                "address": f"{i*100} Main St, City {i}",
                "quantity": i * 200,
                "capacity": i * 300,
                "utilization": round((i * 200) / (i * 300) * 100, 2)
            }
            for i in range(1, 6)
        },
        "categories": {
            f"category_{i}": {
                "name": f"Category {i}",
                "quantity": i * 150,
                "last_updated": f"2024-01-{i:02d}",
                "trend": "up" if i % 2 == 0 else "down"
            }
            for i in range(1, 6)
        },
        "alerts": [
            {
                "id": i,
                "type": "low_stock" if i % 3 == 0 else "reorder",
                "message": f"Alert message {i}",
                "priority": i % 5,
                "timestamp": f"2024-01-12T{i:02d}:00:00Z"
            }
            for i in range(1, 6)
        ]
    }
    
    product_info_data = {
        "product_id": "PROD-001",
        "name": "Laptop ThinkPad X1",
        "price": "1299.99",
        "stock": json.dumps(stock_json, ensure_ascii=False, indent=2)
    }
    
    # Add 5000 additional fields
    for i in range(1, 5001):
        product_info_data[f"field_{i}"] = f"value_{i}"
    
    r.hset("sample:demo:product_info", mapping=product_info_data)
    count += 2
    
    # List keys (2)
    r.rpush("sample:demo:shopping_cart", "item-123", "item-456", "item-789")
    r.rpush("sample:demo:recent_searches", "redis", "database", "cache")
    count += 2
    
    # Set keys (2)
    r.sadd("sample:demo:user_permissions", "read", "write", "delete", "admin")
    r.sadd("sample:demo:product_tags", "electronics", "laptop", "business", "premium")
    count += 2
    
    # ZSet keys (2)
    r.zadd("sample:demo:top_players", {
        "Alice": 9500,
        "Bob": 8700,
        "Charlie": 8200,
        "Diana": 7900
    })
    r.zadd("sample:demo:popular_posts", {
        "post-101": 1520,
        "post-205": 1340,
        "post-187": 980
    })
    count += 2
    
    print(f"\n✓ Done! Created {count} sample keys")
    print(f"Redis: {REDIS_HOST}:{REDIS_PORT}")
    print("\nExample keys created:")
    print("  - sample:string_1 (String)")
    print("  - sample:hash_1 (Hash)")
    print("  - sample:list_1 (List)")
    print("  - sample:set_1 (Set)")
    print("  - sample:zset_1 (ZSet)")
    print("  - sample:users:item_1 (String in 2-level dir)")
    print("\nYou can now connect with UrDis app to take screenshots!")

if __name__ == "__main__":
    main()
