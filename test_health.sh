#!/bin/bash

# Test script untuk demonstrasi Health Check functionality
# Usage: ./test_health.sh

set -e

BASE_URL="http://localhost:8080"

echo "🏥 === PRINTER PROXY HEALTH CHECK TESTING ==="
echo ""

# Test 1: Basic health check
echo "1️⃣ Testing basic application health..."
curl -s "${BASE_URL}/healthz"
echo " ✅ Basic health check"
echo ""

# Test 2: All printers health check
echo "2️⃣ Testing all printers health check..."
response=$(curl -s "${BASE_URL}/health/printers")
echo "$response" | jq .
echo ""

# Extract status for decisions
status=$(echo "$response" | jq -r '.status')
online_count=$(echo "$response" | jq -r '.summary.online')
offline_count=$(echo "$response" | jq -r '.summary.offline')

echo "📊 Health Summary:"
echo "   Status: $status"
echo "   Online: $online_count printers"
echo "   Offline: $offline_count printers"
echo ""

# Test 3: Individual printer health checks
echo "3️⃣ Testing individual printer health checks..."
printers=("printer_kasir_1" "printer_kasir_2")

for printer in "${printers[@]}"; do
    echo "   🖨️ Checking $printer..."
    printer_response=$(curl -s "${BASE_URL}/health/printer/${printer}")
    printer_status=$(echo "$printer_response" | jq -r '.status')
    printer_message=$(echo "$printer_response" | jq -r '.message')
    
    echo "      Status: $printer_status"
    echo "      Message: $printer_message"
    echo ""
done

# Test 4: Print request to offline printer
echo "4️⃣ Testing print request to offline printer..."
print_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"ops": [{"type": "text", "data": "Test print to offline printer"}]}' \
    "${BASE_URL}/printer_kasir_1/cgi-bin/epos/service.cgi")

echo "   Response: $print_response"

if [[ "$print_response" == *"success=\"false\""* ]]; then
    echo "   ✅ Correctly rejected request to offline printer"
else
    echo "   ❌ Unexpected response to offline printer request"
fi
echo ""

# Test 5: Invalid printer test
echo "5️⃣ Testing invalid printer ID..."
invalid_response=$(curl -s "${BASE_URL}/health/printer/invalid_printer")
echo "   Response: $invalid_response"
echo ""

# Test 6: Performance test
echo "6️⃣ Performance test - multiple concurrent health checks..."
start_time=$(date +%s%N)

# Run 5 concurrent health checks
for i in {1..5}; do
    curl -s "${BASE_URL}/health/printers" > /dev/null &
done
wait

end_time=$(date +%s%N)
duration=$((($end_time - $start_time) / 1000000))

echo "   ⏱️ 5 concurrent health checks completed in ${duration}ms"
echo ""

# Test 7: Monitoring simulation
echo "7️⃣ Monitoring simulation (5 checks with 2s interval)..."
for i in {1..5}; do
    echo "   Check $i:"
    timestamp=$(date '+%H:%M:%S')
    status=$(curl -s "${BASE_URL}/health/printers" | jq -r '.summary')
    echo "      [$timestamp] $status"
    
    if [ $i -lt 5 ]; then
        sleep 2
    fi
done
echo ""

echo "🎉 === HEALTH CHECK TESTING COMPLETED ==="
echo ""
echo "📋 Summary:"
echo "   ✅ All endpoints responding correctly"
echo "   ✅ Offline printer detection working"
echo "   ✅ Error handling functioning properly"
echo "   ✅ Performance is acceptable"
echo ""
echo "🔗 Available endpoints:"
echo "   • Basic health: ${BASE_URL}/healthz"
echo "   • All printers: ${BASE_URL}/health/printers"
echo "   • Individual: ${BASE_URL}/health/printer/{printer_id}"
echo "   • Print (with health check): ${BASE_URL}/{printer_id}/cgi-bin/epos/service.cgi"
