import { Badge, Card } from '../components/UI';

export default function FileAvailabilityPage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">File Availability</h1>
          <p className="text-sm text-gray-600">
            Integrate the storage availability service to inspect the health of replicated datasets across the network.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="Storage Provider Integration">
        <p className="text-sm text-gray-700">
          Mock file listings have been removed. Connect the storage RPC service or on-chain contracts responsible for
          content auditing to populate this view with live availability metrics.
        </p>
      </Card>
    </div>
  );
}
