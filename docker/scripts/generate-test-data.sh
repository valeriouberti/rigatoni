#!/bin/bash
# Generate test data in MongoDB to trigger change stream events

set -e

MONGODB_URI="${MONGODB_URI:-mongodb://localhost:27017/testdb?replicaSet=rs0&directConnection=true}"

echo "🔧 Generating test data in MongoDB..."
echo "URI: $MONGODB_URI"
echo ""

mongosh "$MONGODB_URI" <<'EOF'
// Insert new users
print("📝 Inserting new users...");
db.users.insertMany([
  {
    name: "David Wilson",
    email: "david@example.com",
    age: 29,
    department: "Engineering",
    createdAt: new Date()
  },
  {
    name: "Emma Davis",
    email: "emma@example.com",
    age: 31,
    department: "Marketing",
    createdAt: new Date()
  },
  {
    name: "Frank Miller",
    email: "frank@example.com",
    age: 45,
    department: "Sales",
    createdAt: new Date()
  }
]);
print("✅ Inserted 3 users");

// Update existing users
print("\n📝 Updating existing users...");
const updateResult = db.users.updateMany(
  { age: { $lt: 30 } },
  { $set: { status: "young_professional", updatedAt: new Date() } }
);
print(`✅ Updated ${updateResult.modifiedCount} users`);

// Insert new orders
print("\n📝 Inserting new orders...");
db.orders.insertMany([
  {
    userId: "user_" + Math.floor(Math.random() * 1000),
    items: [
      { productId: "prod_1", quantity: 2, price: 29.99 },
      { productId: "prod_2", quantity: 1, price: 49.99 }
    ],
    total: 109.97,
    status: "pending",
    createdAt: new Date()
  },
  {
    userId: "user_" + Math.floor(Math.random() * 1000),
    items: [
      { productId: "prod_3", quantity: 5, price: 9.99 }
    ],
    total: 49.95,
    status: "processing",
    createdAt: new Date()
  }
]);
print("✅ Inserted 2 orders");

// Update orders
print("\n📝 Updating order statuses...");
const orderUpdateResult = db.orders.updateMany(
  { status: "pending" },
  {
    $set: {
      status: "confirmed",
      confirmedAt: new Date()
    }
  }
);
print(`✅ Updated ${orderUpdateResult.modifiedCount} orders`);

// Insert new products
print("\n📝 Inserting new products...");
db.products.insertMany([
  {
    name: "Wireless Mouse",
    price: 29.99,
    category: "electronics",
    inStock: true,
    inventory: 150,
    createdAt: new Date()
  },
  {
    name: "Ergonomic Keyboard",
    price: 89.99,
    category: "electronics",
    inStock: true,
    inventory: 75,
    createdAt: new Date()
  },
  {
    name: "Monitor Stand",
    price: 39.99,
    category: "accessories",
    inStock: false,
    inventory: 0,
    createdAt: new Date()
  }
]);
print("✅ Inserted 3 products");

// Delete some products
print("\n📝 Deleting out-of-stock products...");
const deleteResult = db.products.deleteMany({ inStock: false });
print(`✅ Deleted ${deleteResult.deletedCount} products`);

// Summary
print("\n" + "=".repeat(50));
print("📊 Summary of changes:");
print("=".repeat(50));
print(`Users: ${db.users.countDocuments({})} total`);
print(`Orders: ${db.orders.countDocuments({})} total`);
print(`Products: ${db.products.countDocuments({})} total`);
print("\n✅ Test data generation complete!");
print("📡 These changes should now appear in your pipeline!");
EOF

echo ""
echo "🎉 Done! Check your pipeline logs to see the changes being processed."
echo ""
echo "💡 Run this script multiple times to generate more test events:"
echo "   ./scripts/generate-test-data.sh"
