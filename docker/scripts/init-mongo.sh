#!/bin/bash
# MongoDB initialization script for local development
# This script runs in a separate container after MongoDB is healthy

set -e

MONGODB_HOST="mongodb"
MONGODB_PORT="27017"

echo "========================================="
echo "MongoDB Initialization Script"
echo "========================================="
echo ""

echo "Waiting for MongoDB to be ready..."
sleep 5

echo "Step 1: Initializing replica set..."

# Initialize replica set (no auth for local dev)
mongosh --host $MONGODB_HOST:$MONGODB_PORT --quiet <<'EOF'
try {
  const status = rs.status();
  print("✅ Replica set already initialized");
  print(JSON.stringify(status.set, null, 2));
} catch (err) {
  print("⚙️  Initializing replica set 'rs0'...");
  rs.initiate({
    _id: "rs0",
    members: [
      { _id: 0, host: "mongodb:27017" }
    ]
  });
  print("✅ Replica set initialized successfully");
}
EOF

echo ""
echo "Step 2: Waiting for replica set to become primary..."
sleep 10

echo ""
echo "Step 3: Creating database and collections..."

mongosh --host $MONGODB_HOST:$MONGODB_PORT --quiet <<'EOF'
// Switch to test database
use testdb;

// Create collections
print("Creating collections...");
db.createCollection("users");
db.createCollection("orders");
db.createCollection("products");

print("✅ Collections created: users, orders, products");

// Insert sample data into users collection
print("");
print("Inserting sample data into 'users'...");
db.users.insertMany([
  {
    _id: ObjectId(),
    name: "Alice Johnson",
    email: "alice@example.com",
    age: 28,
    department: "Engineering",
    createdAt: new Date()
  },
  {
    _id: ObjectId(),
    name: "Bob Smith",
    email: "bob@example.com",
    age: 35,
    department: "Sales",
    createdAt: new Date()
  },
  {
    _id: ObjectId(),
    name: "Charlie Brown",
    email: "charlie@example.com",
    age: 42,
    department: "Marketing",
    createdAt: new Date()
  }
]);
print("✅ Inserted 3 users");

// Insert sample data into orders collection
print("");
print("Inserting sample data into 'orders'...");
db.orders.insertMany([
  {
    _id: ObjectId(),
    userId: "user_1",
    items: [
      { productId: "prod_1", name: "Laptop", quantity: 1, price: 999.99 }
    ],
    total: 999.99,
    status: "completed",
    createdAt: new Date(),
    updatedAt: new Date()
  },
  {
    _id: ObjectId(),
    userId: "user_2",
    items: [
      { productId: "prod_2", name: "Mouse", quantity: 2, price: 29.99 },
      { productId: "prod_3", name: "Keyboard", quantity: 1, price: 89.99 }
    ],
    total: 149.97,
    status: "pending",
    createdAt: new Date(),
    updatedAt: new Date()
  }
]);
print("✅ Inserted 2 orders");

// Insert sample data into products collection
print("");
print("Inserting sample data into 'products'...");
db.products.insertMany([
  {
    _id: ObjectId(),
    name: "Laptop",
    price: 999.99,
    category: "electronics",
    inStock: true,
    inventory: 50,
    createdAt: new Date()
  },
  {
    _id: ObjectId(),
    name: "Wireless Mouse",
    price: 29.99,
    category: "electronics",
    inStock: true,
    inventory: 150,
    createdAt: new Date()
  },
  {
    _id: ObjectId(),
    name: "Desk Chair",
    price: 299.99,
    category: "furniture",
    inStock: true,
    inventory: 25,
    createdAt: new Date()
  }
]);
print("✅ Inserted 3 products");

print("");
print("========================================");
print("📊 Database Summary");
print("========================================");
print("Database: testdb");
print("Collections: users, orders, products");
print("Users:    " + db.users.countDocuments({}));
print("Orders:   " + db.orders.countDocuments({}));
print("Products: " + db.products.countDocuments({}));
print("========================================");
EOF

echo ""
echo "✅ MongoDB initialization complete!"
echo ""
echo "You can now:"
echo "  • View data: http://localhost:8081"
echo "  • Connect:   mongodb://localhost:27017/testdb?replicaSet=rs0"
echo ""
