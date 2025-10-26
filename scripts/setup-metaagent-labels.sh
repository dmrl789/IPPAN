#!/bin/bash

# Setup MetaAgent Labels
# This script creates all the necessary labels for the MetaAgent system

echo "🏷️  Setting up MetaAgent labels..."

# Agent labels
gh label create "agent-alpha" --color "A1D6FF" --description "Assigned to Agent Alpha" || echo "Label agent-alpha already exists"
gh label create "agent-beta" --color "A1FFA1" --description "Assigned to Agent Beta" || echo "Label agent-beta already exists"
gh label create "agent-gamma" --color "FFA1A1" --description "Assigned to Agent Gamma" || echo "Label agent-gamma already exists"
gh label create "agent-delta" --color "FFFFA1" --description "Assigned to Agent Delta" || echo "Label agent-delta already exists"
gh label create "agent-epsilon" --color "FFA1FF" --description "Assigned to Agent Epsilon" || echo "Label agent-epsilon already exists"
gh label create "agent-zeta" --color "A1FFFF" --description "Assigned to Agent Zeta" || echo "Label agent-zeta already exists"
gh label create "agent-theta" --color "FFD6A1" --description "Assigned to Agent Theta" || echo "Label agent-theta already exists"
gh label create "agent-lambda" --color "D6A1FF" --description "Assigned to Agent Lambda" || echo "Label agent-lambda already exists"

# MetaAgent system labels
gh label create "metaagent:approved" --color "00FF00" --description "Approved by MetaAgent for merge" || echo "Label metaagent:approved already exists"
gh label create "locked" --color "FF0000" --description "Resource locked by MetaAgent" || echo "Label locked already exists"
gh label create "conflict:pending" --color "FF8000" --description "Conflict detected, waiting for resolution" || echo "Label conflict:pending already exists"

# Crate lock labels (will be created dynamically)
echo "✅ MetaAgent labels setup complete!"
echo ""
echo "Labels created:"
echo "  - agent-alpha, agent-beta, agent-gamma, agent-delta"
echo "  - agent-epsilon, agent-zeta, agent-theta, agent-lambda"
echo "  - metaagent:approved, locked, conflict:pending"
echo ""
echo "Crate lock labels (locked:crates) will be created automatically by the workflow."