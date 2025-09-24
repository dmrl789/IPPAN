import { Badge, Card } from '../../components/UI';

export default function ContractsPage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Contracts</h1>
          <p className="text-sm text-gray-600">
            Smart contract discovery is not yet wired to the chain indexer. Deploy the contracts index service to list and
            inspect deployed applications.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="Contract Catalog">
        <p className="text-sm text-gray-700">
          The mock contract list has been removed. When the chain indexer is available this page will enumerate deployed
          contracts along with metadata and interaction shortcuts.
        </p>
      </Card>
    </div>
  );
}
