import { Badge, Card } from '../components/UI';

export default function StoragePage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Storage Marketplace</h1>
          <p className="text-sm text-gray-600">
            Connect the storage provider API to browse offers, upload datasets, and track replication agreements.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="Storage Providers">
        <p className="text-sm text-gray-700">
          Demo pricing tables and mock uploads have been removed. Implement the storage module RPC handlers to expose real
          provider offers and dataset management tools here.
        </p>
      </Card>
    </div>
  );
}
