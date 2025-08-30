#!/bin/bash

echo "IPPAN Two Nodes Connection Demo"
echo "==============================="
echo ""
echo "Choose which demo to run:"
echo "1) Simple Two Nodes Demo (simplified implementation)"
echo "2) Real IPPAN Nodes Demo (actual node implementation)"
echo -n "Enter choice [1-2]: "
read choice

case $choice in
    1)
        echo ""
        echo "Running Simple Two Nodes Demo..."
        echo "--------------------------------"
        cargo run --example two_nodes_connect
        ;;
    2)
        echo ""
        echo "Running Real IPPAN Nodes Demo..."
        echo "---------------------------------"
        echo "Note: This will start actual IPPAN nodes on ports 8080/9001 and 8081/9002"
        cargo run --example real_nodes_connect
        ;;
    *)
        echo "Invalid choice. Please run the script again and choose 1 or 2."
        exit 1
        ;;
esac