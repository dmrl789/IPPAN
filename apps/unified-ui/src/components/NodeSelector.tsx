import { useState, useEffect } from 'react';
import { Card, Button } from './UI';

interface NodeConfig {
  name: string;
  url: string;
  status: 'connected' | 'disconnected' | 'checking';
}

const NODES: NodeConfig[] = [
  { name: 'Local Blockchain', url: 'http://localhost:3001', status: 'checking' },
  { name: 'Server 1 (Primary)', url: 'http://188.245.97.41:3000', status: 'checking' },
  { name: 'Server 2 (Secondary)', url: 'http://135.181.145.174:3001', status: 'checking' },
];

export default function NodeSelector() {
  const [selectedNode, setSelectedNode] = useState<string>(NODES[0].url);
  const [nodes, setNodes] = useState<NodeConfig[]>(NODES);

  const checkNodeStatus = async (node: NodeConfig) => {
    try {
      const response = await fetch(`${node.url}/health`, { 
        method: 'GET',
        signal: AbortSignal.timeout(3000)
      });
      return response.ok ? 'connected' : 'disconnected';
    } catch {
      return 'disconnected';
    }
  };

  useEffect(() => {
    const checkAllNodes = async () => {
      const updatedNodes = await Promise.all(
        NODES.map(async (node) => ({
          ...node,
          status: await checkNodeStatus(node)
        }))
      );
      setNodes(updatedNodes);
    };

    checkAllNodes();
    const interval = setInterval(checkAllNodes, 10000); // Check every 10 seconds
    return () => clearInterval(interval);
  }, []);

  const handleNodeSelect = (url: string) => {
    setSelectedNode(url);
    // Update the global API base URL
    window.location.reload(); // Simple approach - reload with new config
  };

  return (
    <Card title="IPPAN Node Selection">
      <div className="space-y-3">
        <p className="text-sm text-gray-600">
          Select which IPPAN node to connect to:
        </p>
        
        {nodes.map((node) => (
          <div 
            key={node.url}
            className={`p-3 rounded-lg border cursor-pointer transition-colors ${
              selectedNode === node.url 
                ? 'border-blue-500 bg-blue-50' 
                : 'border-gray-200 hover:border-gray-300'
            }`}
            onClick={() => handleNodeSelect(node.url)}
          >
            <div className="flex items-center justify-between">
              <div>
                <div className="font-medium">{node.name}</div>
                <div className="text-sm text-gray-500">{node.url}</div>
              </div>
              <div className="flex items-center gap-2">
                <div className={`w-2 h-2 rounded-full ${
                  node.status === 'connected' ? 'bg-green-500' :
                  node.status === 'disconnected' ? 'bg-red-500' :
                  'bg-yellow-500'
                }`} />
                <span className="text-xs text-gray-500 capitalize">
                  {node.status}
                </span>
              </div>
            </div>
          </div>
        ))}
        
        <div className="text-xs text-gray-500 mt-2">
          Current API URL: {selectedNode}
        </div>
      </div>
    </Card>
  );
}
